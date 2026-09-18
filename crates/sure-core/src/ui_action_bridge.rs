//! UI-action completeness bridge.
//!
//! `P6-T006`'s acceptance:
//!
//! > *Runtime-backed UI evidence used where available; static-only result remains
//! > inference.*
//!
//! # What it does
//!
//! Scans frontend source files for declared UI action bindings (`onClick`,
//! `onSubmit`, etc.) and produces candidate checks. When runtime browser
//! observations are available, matching declared actions are upgraded from
//! [`EvidenceClass::Inference`] to [`EvidenceClass::ObservedFact`].
//!
//! # What it does not do
//!
//! It does not drive a browser — that is [`crate::browser_driver`]'s work. It
//! accepts observed elements as input and performs the matching.

use std::collections::HashSet;
use std::path::Path;

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::ids::FingerprintId;
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::candidate_context::classify_path;
use crate::checks::check_id;
use crate::discover::Discovery;
use crate::finding_gravity::{GapKind, Reach, gravity_of};
use crate::redact::escape_control_characters;
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

/// What kind of interactive element was observed at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObservedElementKind {
    /// A clickable button.
    Button,
    /// A form with a submit action.
    Form,
    /// A link that triggers an action rather than plain navigation.
    Link,
}

/// One interactive element the browser observed on the page.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObservedElement {
    /// What kind of element was seen.
    pub kind: ObservedElementKind,
    /// Text, label, or other identifier that describes the element.
    pub label: String,
}

/// A UI action kind found in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UiActionKind {
    Click,
    Submit,
}

impl UiActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Submit => "submit",
        }
    }
}

/// One UI action declared in frontend source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UiAction {
    kind: UiActionKind,
    element: String,
    file: String,
    line: usize,
    context: String,
}

/// Runtime evidence of interactive UI elements, produced by a browser check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiRuntimeEvidence {
    /// Elements the browser observed on the page.
    pub elements: Vec<ObservedElement>,
}

/// The UI-action completeness checks SURE proposes.
///
/// Built by [`Self::of`], which is the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActionBridge {
    actions: Vec<UiAction>,
    evidence: UiRuntimeEvidence,
}

impl UiActionBridge {
    /// What SURE would check about UI-action completeness.
    ///
    /// One proposal per frontend UI action binding found in source. A project
    /// with no detectable action bindings produces nothing.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        let mut actions = Vec::new();
        scan_frontend_files(discovery, &mut actions);
        Self {
            actions,
            evidence: UiRuntimeEvidence::default(),
        }
    }

    /// Incorporate runtime browser evidence.
    ///
    /// After calling this, [`Self::proposals`] will use
    /// [`EvidenceClass::ObservedFact`] for declared actions that have a
    /// matching observed element.
    #[must_use]
    pub fn with_evidence(mut self, evidence: UiRuntimeEvidence) -> Self {
        self.evidence = evidence;
        self
    }

    /// The checks SURE would run, in no particular order.
    #[must_use]
    pub fn proposals(&self) -> Vec<CheckProposal> {
        let mut proposals = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for action in &self.actions {
            let key = format!("{}:{}:{}", action.file, action.line, action.context);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key.clone());

            let has_runtime_evidence = self
                .evidence
                .elements
                .iter()
                .any(|observed| matches_action(action, observed));

            let title = format!(
                "UI action: {} handler on {} in `{}`",
                action.kind.as_str(),
                escape_control_characters(&action.element),
                escape_control_characters(&action.file)
            );

            let (evidence_class, actions): (EvidenceClass, &[ActionKind]) = if has_runtime_evidence
            {
                (
                    EvidenceClass::ObservedFact,
                    &[ActionKind::BrowserObservation],
                )
            } else {
                (EvidenceClass::Inference, &[ActionKind::ReadFile])
            };

            let reason = CheckReason::CandidateFound {
                path: action.file.clone(),
                line: action.line,
                context: action.context.clone(),
            };
            // **A declared UI action SURE has no evidence about is rated
            // `must_fix`, and that is the corpus's decision rather than a
            // convenience.** `fixtures/adversarial/dead-button/scenario.json`
            // requires `must_fix` for this very proposal and says why in its own
            // words: *an inference about a primary action must stay visible
            // rather than be assumed fine*. Until `P7-T011` this module said
            // `Note` here, and the aggregator filed it as style noise, so the
            // only evidence about the fixture's dead button was invisible.
            //
            // What is claimed is narrow: a user-facing action is declared, and
            // SURE has not established what it does. That is not a statement that
            // the handler is broken — no detector here reads a handler body — and
            // it is the reason the severity does not travel with a raised
            // evidence class: the class stays `Inference` unless the browser
            // observed the element, exactly as `P6-T006`'s acceptance requires.
            //
            // The cost is stated rather than hidden: this fires on every declared
            // `onClick`/`onSubmit` in a frontend source file, so a healthy
            // single-page app now yields `must_fix` candidates. It does not move
            // the verdict — `critical` stays false, so `CheckResult::blocks_green`
            // is untouched — and it is the price of not letting an unchecked
            // primary action disappear into a bucket named style noise.
            // `classify_path` reads the path itself and ignores both the root and
            // the component graph, so the empty root here is the honest argument
            // rather than a placeholder that changes the answer.
            let reach = Reach::from(classify_path(Path::new(&action.file), Path::new(""), None));
            let gravity = gravity_of(&reason, evidence_class, reach, GapKind::SubstitutedAction);

            proposals.push(CheckProposal::new(
                check_id(&action.file, &format!("ui_action_{}", action.line)),
                title,
                gravity.severity(),
                false,
                evidence_class,
                reason,
                actions,
            ));
        }

        proposals.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
        proposals
    }

    /// Whether there is nothing to check.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Hands every proposal to a plan builder.
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for proposal in self.proposals() {
            if let Err(refusal) = builder.propose(proposal) {
                debug_assert!(
                    builder.refused().contains(&refusal),
                    "the builder returned a refusal it did not record"
                );
            }
        }
    }

    /// One skipped result per proposed check that has no runtime evidence.
    ///
    /// A UI-action check without runtime evidence cannot be confirmed as a
    /// defect from static inspection alone, so every inference proposal becomes
    /// a skipped result with [`NotCheckedReason::UnknownReason`].
    #[must_use]
    pub fn not_checked(&self, project_fingerprint: &FingerprintId) -> Vec<CheckResult> {
        self.proposals()
            .iter()
            .filter(|p| p.evidence_class() == EvidenceClass::Inference)
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

/// Whether an observed element matches a declared UI action.
fn matches_action(action: &UiAction, observed: &ObservedElement) -> bool {
    let kind_matches = matches!(
        (action.kind, observed.kind.clone()),
        (UiActionKind::Click, ObservedElementKind::Button)
            | (UiActionKind::Click, ObservedElementKind::Link)
            | (UiActionKind::Submit, ObservedElementKind::Form)
    );

    if !kind_matches {
        return false;
    }

    // Fuzzy match: the observed label appears in the source context.
    let context_lower = action.context.to_lowercase();
    let label_lower = observed.label.to_lowercase();
    context_lower.contains(&label_lower)
}

/// Scan source files for UI action bindings.
fn scan_frontend_files(discovery: &Discovery, actions: &mut Vec<UiAction>) {
    let mut candidates: Vec<&Path> = discovery
        .scan
        .files()
        .map(|entry| entry.path.as_path())
        .filter(|path| is_frontend_source(path))
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
        let Ok(metadata) = std::fs::metadata(&full) else {
            continue;
        };
        if metadata.len() > 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = std::fs::read(&full) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };

        files_read += 1;
        bytes_read = bytes_read.saturating_add(metadata.len());

        let file = display_path(path);
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            for (kind, element) in actions_on_line(line) {
                let context = escape_control_characters(line.trim());
                actions.push(UiAction {
                    kind,
                    element,
                    file: file.clone(),
                    line: line_number,
                    context,
                });
            }
        }
    }
}

/// Whether a file is a frontend source file this module reads.
fn is_frontend_source(path: &Path) -> bool {
    let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    matches!(
        extension.as_deref(),
        Some("js" | "jsx" | "ts" | "tsx" | "vue" | "svelte" | "mjs" | "cjs" | "mts" | "cts")
    )
}

/// Every UI action binding on one line.
fn actions_on_line(line: &str) -> Vec<(UiActionKind, String)> {
    let mut found = Vec::new();

    // onClick= or onClick: (React/JSX/Vue bindings)
    for pattern in ["onClick=", "onClick:"] {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find(pattern) {
            let before = &line[..search_from + start];
            let element = tag_name_before(before).unwrap_or_else(|| "element".to_owned());
            found.push((UiActionKind::Click, element));
            search_from += start + pattern.len();
        }
    }
    // Vue @click with optional modifiers (@click.prevent, @click.stop, etc.)
    {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find("@click") {
            let end = search_from + start + "@click".len();
            let next = line[end..].chars().next();
            if next == Some('=') || next == Some('.') || next == Some(' ') || next == Some('\t') {
                let before = &line[..search_from + start];
                let element = tag_name_before(before).unwrap_or_else(|| "element".to_owned());
                found.push((UiActionKind::Click, element));
            }
            search_from = end + 1;
        }
    }

    // onSubmit= or onSubmit: (React/JSX/Vue bindings)
    for pattern in ["onSubmit=", "onSubmit:"] {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find(pattern) {
            let before = &line[..search_from + start];
            let element = tag_name_before(before).unwrap_or_else(|| "form".to_owned());
            found.push((UiActionKind::Submit, element));
            search_from += start + pattern.len();
        }
    }
    // Vue @submit with optional modifiers (@submit.prevent, etc.)
    {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find("@submit") {
            let end = search_from + start + "@submit".len();
            let next = line[end..].chars().next();
            if next == Some('=') || next == Some('.') || next == Some(' ') || next == Some('\t') {
                let before = &line[..search_from + start];
                let element = tag_name_before(before).unwrap_or_else(|| "form".to_owned());
                found.push((UiActionKind::Submit, element));
            }
            search_from = end + 1;
        }
    }

    // addEventListener('click', ...) or addEventListener("click", ...)
    for pattern in ["addEventListener('click',", "addEventListener(\"click\","] {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find(pattern) {
            let before = &line[..search_from + start];
            let element = receiver_before(before).unwrap_or_else(|| "element".to_owned());
            found.push((UiActionKind::Click, element));
            search_from += start + pattern.len();
        }
    }

    // addEventListener('submit', ...) or addEventListener("submit", ...)
    for pattern in ["addEventListener('submit',", "addEventListener(\"submit\","] {
        let mut search_from = 0;
        while let Some(start) = line[search_from..].find(pattern) {
            let before = &line[..search_from + start];
            let element = receiver_before(before).unwrap_or_else(|| "form".to_owned());
            found.push((UiActionKind::Submit, element));
            search_from += start + pattern.len();
        }
    }

    found
}

/// The tag name immediately before an attribute, if we're inside a tag.
fn tag_name_before(before: &str) -> Option<String> {
    let trimmed = before.trim_end();
    let after_lt = trimmed.rsplit_once('<')?.1;
    let name = after_lt.split_whitespace().next()?;
    if name.is_empty() || name.starts_with('/') {
        return None;
    }
    Some(name.to_lowercase())
}

/// The receiver before an `addEventListener` call.
fn receiver_before(before: &str) -> Option<String> {
    let trimmed = before.trim_end();
    let idx = trimmed.rfind(".addEventListener")?;
    let receiver_end = trimmed[..idx].trim_end();
    let receiver = receiver_end
        .rsplit(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .next()?;
    if receiver.is_empty() {
        return None;
    }
    Some(receiver.to_lowercase())
}

/// Whether a resolved path is still inside the project root.
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::severity::Severity;

    #[test]
    fn a_project_with_no_frontend_produces_nothing() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-bridge-clean-test-{}-{}",
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
        let bridge = UiActionBridge::of(&discovery);
        assert!(bridge.is_empty());
        assert!(bridge.proposals().is_empty());
    }

    #[test]
    fn a_declared_click_without_runtime_evidence_is_inference() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-inference-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("App.tsx"),
            "<button onClick={handleSave}>Save</button>\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        assert!(!bridge.is_empty());

        let proposals = bridge.proposals();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert!(proposal.title().contains("click"));
        assert!(proposal.title().contains("button"));
        assert_eq!(proposal.severity(), Severity::MustFix);
        assert!(!proposal.critical());
        assert_eq!(proposal.evidence_class(), EvidenceClass::Inference);
        assert_eq!(proposal.requirements().actions(), &[ActionKind::ReadFile]);
    }

    #[test]
    fn a_declared_click_confirmed_by_browser_observation_becomes_observed_fact() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-observed-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("App.tsx"),
            "<button onClick={handleSave}>Save</button>\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery).with_evidence(UiRuntimeEvidence {
            elements: vec![ObservedElement {
                kind: ObservedElementKind::Button,
                label: "Save".to_owned(),
            }],
        });

        let proposals = bridge.proposals();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert_eq!(proposal.evidence_class(), EvidenceClass::ObservedFact);
        assert_eq!(
            proposal.requirements().actions(),
            &[ActionKind::BrowserObservation]
        );
        assert_eq!(
            proposal.severity(),
            Severity::MustFix,
            "observing the element does not establish what the handler does, so the \
             severity is the same one the unobserved case gets and the evidence \
             class is what carries the difference"
        );
    }

    #[test]
    fn a_plain_div_without_event_handler_produces_nothing() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-negative-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(temp.join("App.tsx"), "<div>Hello</div>\n").unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        assert!(bridge.is_empty());
    }

    #[test]
    fn a_declared_submit_handler_is_detected() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-submit-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("Form.vue"),
            "<form @submit.prevent=\"handleSubmit\">\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        let proposals = bridge.proposals();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].title().contains("submit"));
    }

    #[test]
    fn not_checked_returns_skipped_for_inference_only() {
        let fingerprint = FingerprintId::generate();
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-not-checked-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("App.tsx"),
            "<button onClick={handleSave}>Save</button>\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        let results = bridge.not_checked(&fingerprint);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, sure_domain::status::CheckStatus::Skipped);
    }

    #[test]
    fn nothing_this_module_builds_is_refused_and_the_plan_holds_everything() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-plan-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("App.tsx"),
            "<button onClick={handleSave}>Save</button>\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        assert_eq!(bridge.proposals().len(), 1);

        let mut builder = PlanBuilder::new(
            sure_domain::execution::ExecutionMode::HostConfirmed,
            sure_domain::execution::ExecutionPermissions::inspect_only(),
        );
        bridge.add_to(&mut builder);

        assert!(builder.refused().is_empty());
        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert!(schedule.duplicates().is_empty());
    }

    #[test]
    fn a_link_click_is_detected() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-link-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("Link.tsx"),
            "<a onClick={handleNavigate}>Go</a>\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        let proposals = bridge.proposals();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].title().contains("click"));
        assert!(proposals[0].title().contains("a"));
    }

    #[test]
    fn addeventlistener_click_is_detected() {
        let temp = std::env::temp_dir().join(format!(
            "sure-ui-addevent-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(
            temp.join("script.js"),
            "document.getElementById('btn').addEventListener('click', handleClick);\n",
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let bridge = UiActionBridge::of(&discovery);
        let proposals = bridge.proposals();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].title().contains("click"));
    }
}
