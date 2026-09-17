//! Candidate scanner for TODO/mock/stub/placeholder patterns.
//!
//! `P6-T001`'s acceptance, and it is one sentence:
//!
//! > Candidates include context and are not automatically product defects.
//!
//! # What it detects
//!
//! Four categories of incomplete code, detected statically from source files:
//! - `TODO` and `FIXME` comments indicating unfinished work
//! - `mock` usage in code
//! - `stub` usage in code
//! - `placeholder` usage in code
//!
//! # What it does not do
//!
//! It does not judge whether a candidate is a defect. A test mock is not a
//! product defect, and a TODO in a prototype is not either. This module finds
//! patterns and records where they were found; what they mean is decided by
//! the caller or by later filters.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::candidate_context::{CandidateContext, classify_path};
use crate::checks::check_id;
use crate::discover::Discovery;
use crate::redact::escape_control_characters;
use crate::references::is_source_candidate;
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

/// What kind of candidate SURE detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateCategory {
    /// A TODO or FIXME comment indicating unfinished work.
    Todo,
    /// A mock object or function, often in tests but not always.
    Mock,
    /// A stub implementation standing in for real code.
    Stub,
    /// A placeholder value or text waiting to be replaced.
    Placeholder,
}

/// The weights one category carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CategoryRow {
    /// How bad it is if the check does not pass.
    severity: Severity,
    /// Whether the project cannot be trusted for hand-off when it does not pass.
    critical: bool,
    /// What the check's result would be worth.
    evidence_class: EvidenceClass,
}

/// The categories SURE checks, and what each one's check is.
///
/// **The order is the report's**: todo, mock, stub, placeholder.
const CATEGORIES: &[(CandidateCategory, CategoryRow)] = &[
    (
        CandidateCategory::Todo,
        CategoryRow {
            severity: Severity::Note,
            critical: false,
            evidence_class: EvidenceClass::Inference,
        },
    ),
    (
        CandidateCategory::Mock,
        CategoryRow {
            severity: Severity::Note,
            critical: false,
            evidence_class: EvidenceClass::Inference,
        },
    ),
    (
        CandidateCategory::Stub,
        CategoryRow {
            severity: Severity::Note,
            critical: false,
            evidence_class: EvidenceClass::Inference,
        },
    ),
    (
        CandidateCategory::Placeholder,
        CategoryRow {
            severity: Severity::Note,
            critical: false,
            evidence_class: EvidenceClass::Inference,
        },
    ),
];

impl CandidateCategory {
    /// The readable half of this category's check identifiers.
    const fn tag(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Mock => "mock",
            Self::Stub => "stub",
            Self::Placeholder => "placeholder",
        }
    }

    /// The phrase a check's title is built from.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Todo => "project contains TODO or FIXME comments",
            Self::Mock => "project contains mock usage",
            Self::Stub => "project contains stub usage",
            Self::Placeholder => "project contains placeholder usage",
        }
    }

    /// The phrase a check's title is built from, with context.
    #[must_use]
    pub fn contextual_description(self, context: CandidateContext) -> String {
        let base = match self {
            Self::Todo => "project contains TODO or FIXME comments",
            Self::Mock => "project contains mock usage",
            Self::Stub => "project contains stub usage",
            Self::Placeholder => "project contains placeholder usage",
        };
        let suffix = match context {
            CandidateContext::Test => " in tests",
            CandidateContext::Example => " in examples",
            CandidateContext::Doc => " in documentation",
            CandidateContext::MockFixture => " in mock fixtures",
            CandidateContext::Product => " in production code",
        };
        format!("{base}{suffix}")
    }

    /// The patterns that signal this category.
    const fn patterns(self) -> &'static [&'static str] {
        match self {
            Self::Todo => &["TODO", "FIXME"],
            Self::Mock => &["mock"],
            Self::Stub => &["stub"],
            Self::Placeholder => &["placeholder"],
        }
    }
}

/// Where a candidate pattern was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Detection {
    category: CandidateCategory,
    file: String,
    line: usize,
    context: String,
    path_context: CandidateContext,
}

/// The checks SURE proposes for detected candidate patterns.
///
/// Built from a discovery result and nothing else — see the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateScanner {
    proposals: Vec<CheckProposal>,
}

impl CandidateScanner {
    /// What SURE would check about candidate patterns in this project.
    ///
    /// **One proposal per detected category**: if a project contains both TODO
    /// comments and mock usage, two checks are proposed. A project that contains
    /// none produces nothing.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        let mut detections: Vec<Detection> = Vec::new();
        let mut seen = HashSet::new();

        let graph = crate::components::ComponentGraph::of(discovery);
        scan_source_files(discovery, &graph, &mut seen, &mut detections);

        // Build proposals, one per category, using the first detection as the
        // reason's anchor. Prefer a Product-context detection when one exists,
        // because a candidate in production code is the more significant claim.
        let mut proposals = Vec::new();
        for &(category, row) in CATEGORIES {
            let detection = detections
                .iter()
                .find(|d| d.category == category && d.path_context == CandidateContext::Product)
                .or_else(|| detections.iter().find(|d| d.category == category));
            let Some(detection) = detection else {
                continue;
            };
            proposals.push(CheckProposal::new(
                check_id(&detection.file, &format!("candidate{}", category.tag())),
                category.contextual_description(detection.path_context),
                row.severity,
                row.critical,
                row.evidence_class,
                CheckReason::CandidateFound {
                    path: detection.file.clone(),
                    line: detection.line,
                    context: detection.context.clone(),
                },
                &[ActionKind::ReadFile],
            ));
        }

        // Stable order, so two runs over one project cannot produce two plans.
        proposals.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));

        Self { proposals }
    }

    /// The checks SURE would run, in no particular order.
    #[must_use]
    pub fn proposed(&self) -> &[CheckProposal] {
        &self.proposals
    }

    /// Whether there is nothing to check.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.proposals.is_empty()
    }

    /// Hands every proposal to a plan builder.
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for proposal in &self.proposals {
            if let Err(refusal) = builder.propose(proposal.clone()) {
                debug_assert!(
                    builder.refused().contains(&refusal),
                    "the builder returned a refusal it did not record"
                );
            }
        }
    }

    /// One skipped result per proposed check.
    ///
    /// **This is the acceptance sentence, and it is the only thing this module
    /// produces that is a result.** A candidate check cannot be confirmed as a
    /// defect from static inspection alone, so every proposal becomes a skipped
    /// result with [`NotCheckedReason::UnknownReason`].
    #[must_use]
    pub fn not_checked(&self, project_fingerprint: &FingerprintId) -> Vec<CheckResult> {
        self.proposals
            .iter()
            .map(|proposal| {
                CheckResult::not_run(
                    proposal.id().clone(),
                    proposal.title().to_owned(),
                    proposal.severity(),
                    proposal.critical(),
                    NotCheckedReason::UnknownReason,
                    project_fingerprint.clone(),
                )
            })
            .collect()
    }
}

/// Scan source files for candidate patterns.
fn scan_source_files(
    discovery: &Discovery,
    graph: &crate::components::ComponentGraph,
    seen: &mut HashSet<CandidateCategory>,
    detections: &mut Vec<Detection>,
) {
    let mut candidates: Vec<&Path> = discovery
        .scan
        .files()
        .map(|entry| entry.path.as_path())
        .filter(|path| is_source_candidate(path))
        .collect();
    candidates.sort_unstable();

    let mut files_read = 0_usize;
    let mut bytes_read = 0_u64;

    for path in candidates {
        if files_read >= 512 || bytes_read > 16 * 1024 * 1024 {
            break;
        }

        let full = discovery.root.join(path);
        if !is_within_project(&discovery.root, &full) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&full) else {
            continue;
        };
        if metadata.len() > 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(&full) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };

        files_read += 1;
        bytes_read = bytes_read.saturating_add(metadata.len());

        let file = display_path(path);
        let path_context = classify_path(path, &discovery.root, Some(graph));
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            for &(category, _) in CATEGORIES {
                if seen.contains(&category) {
                    continue;
                }
                if let Some(context) = line_contains_pattern(line, category.patterns()) {
                    let mut context = escape_control_characters(context.trim());
                    if context.len() > 120 {
                        context.truncate(120);
                        context.push_str("...");
                    }
                    seen.insert(category);
                    detections.push(Detection {
                        category,
                        file: file.clone(),
                        line: line_number,
                        context,
                        path_context,
                    });
                    break;
                }
            }
        }
    }
}

/// Whether a resolved path is still inside the project root.
///
/// The scan module already promises containment and refuses symlinks, but this
/// is a cheap second lock on the door: a path that climbs out with `..` or that
/// resolves to something outside the root is not read.
fn is_within_project(root: &Path, full: &Path) -> bool {
    let Ok(root_canon) = std::fs::canonicalize(root) else {
        return false;
    };
    let Ok(full_canon) = std::fs::canonicalize(full) else {
        return false;
    };
    full_canon.starts_with(&root_canon)
        && full_canon.components().count() > root_canon.components().count()
}

/// Whether a line contains any of the given patterns as whole words.
///
/// Returns the trimmed line if a pattern matches, so the caller has context.
fn line_contains_pattern<'line>(line: &'line str, patterns: &[&str]) -> Option<&'line str> {
    let lower = line.to_lowercase();
    for pattern in patterns {
        let pattern_lower = pattern.to_lowercase();
        let mut search_from = 0;
        while let Some(index) = lower[search_from..].find(&pattern_lower) {
            let absolute = search_from + index;
            let before =
                absolute == 0 || !line[..absolute].ends_with(|c: char| c.is_ascii_alphanumeric());
            let after = absolute + pattern.len() >= line.len()
                || !line[absolute + pattern.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric());
            if before && after {
                return Some(line.trim());
            }
            search_from = absolute + 1;
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::status::{CheckStatus, NotCheckedReason};

    #[test]
    fn the_table_covers_every_category_exactly_once() {
        let listed: Vec<CandidateCategory> = CATEGORIES.iter().map(|(c, _)| *c).collect();
        assert_eq!(
            listed,
            vec![
                CandidateCategory::Todo,
                CandidateCategory::Mock,
                CandidateCategory::Stub,
                CandidateCategory::Placeholder,
            ]
        );
    }

    #[test]
    fn todo_is_detected_in_source() {
        assert_eq!(
            line_contains_pattern("// TODO: fix this", &["TODO", "FIXME"]),
            Some("// TODO: fix this")
        );
    }

    #[test]
    fn fixme_is_detected_in_source() {
        assert_eq!(
            line_contains_pattern("// FIXME: handle error", &["TODO", "FIXME"]),
            Some("// FIXME: handle error")
        );
    }

    #[test]
    fn lowercase_todo_is_detected() {
        assert_eq!(
            line_contains_pattern("// todo: fix this", &["TODO", "FIXME"]),
            Some("// todo: fix this")
        );
    }

    #[test]
    fn mock_is_detected_in_source() {
        assert_eq!(
            line_contains_pattern("const mock = {}", &["mock"]),
            Some("const mock = {}")
        );
    }

    #[test]
    fn stub_is_detected_in_source() {
        assert_eq!(
            line_contains_pattern("function stub() {}", &["stub"]),
            Some("function stub() {}")
        );
    }

    #[test]
    fn placeholder_is_detected_in_source() {
        assert_eq!(
            line_contains_pattern("const name = 'placeholder'", &["placeholder"]),
            Some("const name = 'placeholder'")
        );
    }

    #[test]
    fn partial_word_is_not_detected() {
        assert_eq!(
            line_contains_pattern("// mockdown is a tool", &["mock"]),
            None
        );
    }

    #[test]
    fn stubborn_is_not_detected_as_stub() {
        assert_eq!(line_contains_pattern("// stubborn logic", &["stub"]), None);
    }

    #[test]
    fn a_project_with_no_candidates_produces_no_detections() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-clean-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("main.rs"), "fn main() {}").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert!(scanner.is_empty());
        assert!(scanner.proposed().is_empty());
        assert!(scanner.not_checked(&FingerprintId::generate()).is_empty());
    }

    #[test]
    fn a_project_with_todo_produces_a_check() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-todo-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(
            temp.join("main.rs"),
            "// TODO: implement this\nfn main() {}",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert!(!scanner.is_empty());
        assert_eq!(scanner.proposed().len(), 1);

        let proposal = &scanner.proposed()[0];
        assert_eq!(
            proposal.title(),
            "project contains TODO or FIXME comments in production code"
        );
        assert_eq!(proposal.severity(), Severity::Note);
        assert!(!proposal.critical());
        assert_eq!(proposal.evidence_class(), EvidenceClass::Inference);
        assert_eq!(proposal.requirements().actions(), &[ActionKind::ReadFile]);
        assert!(!proposal.requirements().runs_project_code());
    }

    #[test]
    fn produced_check_does_not_aggregate_to_local_pass() {
        let fingerprint = FingerprintId::generate();
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-agg-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("main.rs"), "// TODO: fix me\nfn main() {}").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert!(!scanner.is_empty());

        let results = scanner.not_checked(&fingerprint);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(!result.status.is_green());
        assert!(
            !result.blocks_green(),
            "a non-critical candidate check that did not run must not block green"
        );
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::UnknownReason)
        );
        assert_eq!(
            result.reason,
            NotCheckedReason::UnknownReason.plain_explanation()
        );
    }

    #[test]
    fn nothing_this_module_builds_is_refused_and_the_plan_holds_everything() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-plan-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(
            temp.join("main.rs"),
            "// TODO: fix me\n// FIXME: also this\nfn main() {}\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);

        let mut builder = PlanBuilder::new(
            sure_domain::execution::ExecutionMode::HostConfirmed,
            sure_domain::execution::ExecutionPermissions::inspect_only(),
        );
        scanner.add_to(&mut builder);

        assert!(builder.refused().is_empty());
        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert!(schedule.duplicates().is_empty());
    }

    #[test]
    fn a_blocked_candidate_check_stays_in_the_plan() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-blocked-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("main.rs"), "// TODO: fix me\nfn main() {}").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);

        let mut builder = PlanBuilder::new(
            sure_domain::execution::ExecutionMode::InspectOnly,
            sure_domain::execution::ExecutionPermissions::inspect_only(),
        );
        scanner.add_to(&mut builder);

        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.blocked().count(), 0);

        let check = &schedule.checks()[0];
        assert_eq!(
            check.proposal().requirements().actions(),
            &[ActionKind::ReadFile]
        );
        assert!(check.proposal().requirements().is_inspection_only());
        assert!(check.may_run());
    }

    #[test]
    fn candidate_found_reason_names_something_and_includes_context() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-reason-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("main.rs"), "// TODO: fix me\nfn main() {}").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);

        let proposal = scanner.proposed().first().unwrap();
        let reason = proposal.reason();
        assert!(reason.names_something());
        let desc = reason.plain_description();
        assert!(
            desc.contains("TODO"),
            "reason should include context: {desc}"
        );
        assert!(
            desc.contains("main.rs"),
            "reason should include file: {desc}"
        );
        assert!(
            desc.contains("line 1"),
            "reason should include line: {desc}"
        );
    }

    #[test]
    fn a_project_with_todo_in_tests_produces_test_context_title() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-test-ctx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("tests")).unwrap();
        std::fs::write(temp.join("tests/foo.rs"), "// TODO: fix me\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);
        assert_eq!(
            scanner.proposed()[0].title(),
            "project contains TODO or FIXME comments in tests"
        );
    }

    #[test]
    fn a_project_with_todo_in_src_produces_product_context_title() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-src-ctx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("src")).unwrap();
        std::fs::write(temp.join("src/lib.rs"), "// TODO: fix me\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);
        assert_eq!(
            scanner.proposed()[0].title(),
            "project contains TODO or FIXME comments in production code"
        );
    }

    #[test]
    fn product_context_is_preferred_when_both_test_and_product_exist() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-prefer-prod-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("tests")).unwrap();
        std::fs::create_dir_all(temp.join("src")).unwrap();
        std::fs::write(temp.join("tests/foo.rs"), "// TODO: test fix\n").unwrap();
        std::fs::write(temp.join("src/lib.rs"), "// TODO: prod fix\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);
        assert_eq!(
            scanner.proposed()[0].title(),
            "project contains TODO or FIXME comments in production code",
            "Product context should be preferred over Test context"
        );
    }

    #[test]
    fn mock_in_tests_produces_test_context_title() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-mock-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("tests")).unwrap();
        std::fs::write(temp.join("tests/foo.rs"), "let mock = 1;\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);
        assert_eq!(
            scanner.proposed()[0].title(),
            "project contains mock usage in tests"
        );
    }

    #[test]
    fn mock_in_src_produces_product_context_title() {
        let temp = std::env::temp_dir().join(format!(
            "sure-candidate-mock-src-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("src")).unwrap();
        std::fs::write(temp.join("src/lib.rs"), "let mock = 1;\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let scanner = CandidateScanner::of(&discovery);
        assert_eq!(scanner.proposed().len(), 1);
        assert_eq!(
            scanner.proposed()[0].title(),
            "project contains mock usage in production code"
        );
    }
}
