//! External-service verification boundary.
//!
//! `P5-T006`'s acceptance, and it is one sentence:
//!
//! > Payment/email/cloud behaviors requiring real external systems become
//! > needs_external_verification / cannot_confirm rather than local pass.
//!
//! # What it detects
//!
//! Three categories of external service, detected statically from:
//! - `package.json` dependencies
//! - import/require statements in source files
//! - environment variable references
//!
//! # What it does not do
//!
//! It runs nothing, starts nothing, and opens no network connections.
//! Detection is entirely static.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::checks::{check_id, nothing_observed_yet};
use crate::discover::node::{MANIFEST, NodeProject, Package};
use crate::discover::{Discovery, Ecosystem};
use crate::planned_work::PlannedWork;
use crate::references::is_source_candidate;
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

/// What kind of external service SURE detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceCategory {
    /// A payment processor such as Stripe, PayPal or Square.
    Payment,
    /// An email delivery service such as SMTP, SendGrid or Mailgun.
    Email,
    /// A cloud storage provider such as AWS S3, Google Cloud Storage or Azure Blob.
    Cloud,
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
/// **The order is the report's**: payment, email, cloud.
const CATEGORIES: &[(ServiceCategory, CategoryRow)] = &[
    (
        ServiceCategory::Payment,
        CategoryRow {
            severity: Severity::ShouldFixFirst,
            critical: true,
            evidence_class: EvidenceClass::Inference,
        },
    ),
    (
        ServiceCategory::Email,
        CategoryRow {
            severity: Severity::ShouldFixFirst,
            critical: true,
            evidence_class: EvidenceClass::Inference,
        },
    ),
    (
        ServiceCategory::Cloud,
        CategoryRow {
            severity: Severity::ShouldFixFirst,
            critical: true,
            evidence_class: EvidenceClass::Inference,
        },
    ),
];

impl ServiceCategory {
    /// The readable half of this category's check identifiers.
    const fn tag(self) -> &'static str {
        match self {
            Self::Payment => "payment",
            Self::Email => "email",
            Self::Cloud => "cloud",
        }
    }

    /// The phrase a check's title is built from.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Payment => "project uses an external payment service",
            Self::Email => "project uses an external email service",
            Self::Cloud => "project uses external cloud storage",
        }
    }

    /// The dependencies that signal this category.
    const fn dependencies(self) -> &'static [&'static str] {
        match self {
            Self::Payment => &[
                "stripe",
                "@stripe/stripe-js",
                "paypal-rest-sdk",
                "@paypal/checkout-server-sdk",
                "squareup",
                "square",
            ],
            Self::Email => &[
                "nodemailer",
                "sendgrid",
                "@sendgrid/mail",
                "mailgun-js",
                "mailgun.js",
            ],
            Self::Cloud => &[
                "aws-sdk",
                "@aws-sdk/client-s3",
                "@aws-sdk/s3-request-presigner",
                "@google-cloud/storage",
                "google-cloud/storage",
                "@azure/storage-blob",
                "azure-storage",
            ],
        }
    }

    /// The environment variable prefixes that signal this category.
    const fn env_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::Payment => &["STRIPE_", "PAYPAL_", "SQUARE_"],
            Self::Email => &["SMTP_", "SENDGRID_", "MAILGUN_"],
            Self::Cloud => &["AWS_", "GCP_", "GOOGLE_CLOUD_", "AZURE_", "AZURE_STORAGE_"],
        }
    }

    /// The import patterns that signal this category.
    const fn import_patterns(self) -> &'static [&'static str] {
        match self {
            Self::Payment => &[
                "stripe",
                "paypal-rest-sdk",
                "@paypal/checkout-server-sdk",
                "squareup",
                "square",
            ],
            Self::Email => &[
                "nodemailer",
                "sendgrid",
                "@sendgrid/mail",
                "mailgun-js",
                "mailgun.js",
            ],
            Self::Cloud => &[
                "aws-sdk",
                "@aws-sdk/client-s3",
                "@aws-sdk/s3-request-presigner",
                "@google-cloud/storage",
                "google-cloud/storage",
                "@azure/storage-blob",
                "azure-storage",
            ],
        }
    }
}

/// Where an external service was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Detection {
    category: ServiceCategory,
    file: String,
}

/// The checks SURE proposes for detected external services.
///
/// Built from a discovery result and nothing else — see the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalServiceChecks {
    proposals: Vec<CheckProposal>,
}

impl ExternalServiceChecks {
    /// What SURE would check about external services in this project.
    ///
    /// **One proposal per detected category**: if a project uses both a payment
    /// processor and cloud storage, two checks are proposed. A project that uses
    /// none produces nothing.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        let mut detections: Vec<Detection> = Vec::new();
        let mut seen = HashSet::new();

        // 1. Node project dependencies.
        if let Some(report) = discovery.report(Ecosystem::Node)
            && let crate::discover::Findings::Node(project) = &report.findings
        {
            scan_node_dependencies(project, &mut seen, &mut detections);
        }

        // 2. Source files for env vars and imports.
        scan_source_files(discovery, &mut seen, &mut detections);

        // Build proposals, one per category, using the first detection as the
        // reason's anchor.
        let mut proposals = Vec::new();
        for &(category, row) in CATEGORIES {
            let Some(detection) = detections.iter().find(|d| d.category == category) else {
                continue;
            };
            proposals.push(CheckProposal::new(
                check_id(&detection.file, &format!("external{}", category.tag())),
                category.plain_description().to_owned(),
                row.severity,
                row.critical,
                row.evidence_class,
                CheckReason::FilePresent {
                    path: detection.file.clone(),
                },
                &[ActionKind::ExternalService],
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
    ///
    /// **Each proposal is paired with the operation that would carry it out, and
    /// the operation here is the placeholder `P18-T003` puts beside every check
    /// that is not a declared command.** These checks can only be confirmed
    /// against the real outside service, which has not been contacted at the
    /// moment the plan is made — so the honest observation is the one that claims
    /// neither a pass nor a defect. `P18-T004` replaces it with the observation
    /// this detector actually made, and ADR 0014's decision 11 is where the
    /// separate question of what a *service* check may be given is answered.
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for proposal in &self.proposals {
            let work = PlannedWork::new(proposal.clone(), nothing_observed_yet());
            if let Err(refusal) = builder.propose(work) {
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
    /// produces that is a result.** An external service check can only be
    /// confirmed against the real outside service, which is not available here,
    /// so every proposal becomes a skipped result with
    /// [`NotCheckedReason::ExternalServiceUnavailable`].
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
                    NotCheckedReason::ExternalServiceUnavailable,
                    project_fingerprint.clone(),
                )
            })
            .collect()
    }
}

/// Scan a Node project's dependencies for external service signals.
fn scan_node_dependencies(
    project: &NodeProject,
    seen: &mut HashSet<ServiceCategory>,
    detections: &mut Vec<Detection>,
) {
    let mut check_package = |package: &Package, manifest_path: &str| {
        for dependency in &package.dependencies {
            for &(category, _) in CATEGORIES {
                if seen.contains(&category) {
                    continue;
                }
                for &name in category.dependencies() {
                    if dependency.name == name {
                        seen.insert(category);
                        detections.push(Detection {
                            category,
                            file: manifest_path.to_owned(),
                        });
                        break;
                    }
                }
            }
        }
    };

    if let Some(package) = project.package() {
        check_package(package, MANIFEST);
    }
    for member in project.workspaces.readable_members() {
        if let Some(package) = member.package.as_deref() {
            let manifest = display_path(&member.path.join(MANIFEST));
            check_package(package, &manifest);
        }
    }
}

/// Scan source files for env var references and import patterns.
fn scan_source_files(
    discovery: &Discovery,
    seen: &mut HashSet<ServiceCategory>,
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
        for line in text.lines() {
            for &(category, _) in CATEGORIES {
                if seen.contains(&category) {
                    continue;
                }
                if line_has_env_prefix(line, category.env_prefixes())
                    || line_has_import(line, category.import_patterns())
                {
                    seen.insert(category);
                    detections.push(Detection {
                        category,
                        file: file.clone(),
                    });
                }
            }
        }
    }
}

/// Whether a line contains an environment variable prefix in an env access.
fn line_has_env_prefix(line: &str, prefixes: &[&str]) -> bool {
    // Quick reject: none of the prefixes are present at all.
    if !prefixes.iter().any(|prefix| line.contains(prefix)) {
        return false;
    }

    // Must look like an env access in at least one language.
    let looks_like_env = line.contains("process.env.")
        || line.contains("process.env[")
        || line.contains("os.environ[")
        || line.contains("os.getenv(")
        || line.contains("env::var(")
        || line.contains("std::env::var(");

    if !looks_like_env {
        return false;
    }

    prefixes.iter().any(|prefix| line.contains(prefix))
}

/// Whether a line contains an import or require of any of the patterns.
fn line_has_import(line: &str, patterns: &[&str]) -> bool {
    for pattern in patterns {
        let quoted_single = format!("'{pattern}'");
        let quoted_double = format!("\"{pattern}\"");
        if line.contains(&format!("require({quoted_single})"))
            || line.contains(&format!("require({quoted_double})"))
            || line.contains(&format!("from {quoted_single}"))
            || line.contains(&format!("from {quoted_double}"))
            || line.contains(&format!("import({quoted_single})"))
            || line.contains(&format!("import({quoted_double})"))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::discover::node::{Managers, ManifestState, Package, Workspaces};
    use serde_json::json;
    use sure_domain::status::{CheckStatus, NotCheckedReason};

    fn package(value: serde_json::Value) -> Package {
        Package::from_json(&value).expect("this fixture is a manifest")
    }

    fn node_project_with_deps(dependencies: serde_json::Value) -> NodeProject {
        NodeProject {
            manifest: ManifestState::Read(Box::new(package(json!({
                "dependencies": dependencies
            })))),
            managers: Managers::default(),
            workspaces: Workspaces::default(),
            tooling: Vec::new(),
            typescript: Default::default(),
        }
    }

    #[test]
    fn the_table_covers_every_category_exactly_once() {
        let listed: Vec<ServiceCategory> = CATEGORIES.iter().map(|(c, _)| *c).collect();
        assert_eq!(
            listed,
            vec![
                ServiceCategory::Payment,
                ServiceCategory::Email,
                ServiceCategory::Cloud,
            ]
        );
    }

    #[test]
    fn payment_is_detected_from_stripe_dependency() {
        let project = node_project_with_deps(json!({ "stripe": "^12.0.0" }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].category, ServiceCategory::Payment);
    }

    #[test]
    fn email_is_detected_from_nodemailer_dependency() {
        let project = node_project_with_deps(json!({ "nodemailer": "^6.9.0" }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].category, ServiceCategory::Email);
    }

    #[test]
    fn cloud_is_detected_from_aws_sdk_dependency() {
        let project = node_project_with_deps(json!({ "aws-sdk": "^2.0.0" }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].category, ServiceCategory::Cloud);
    }

    #[test]
    fn scoped_cloud_dependency_is_detected() {
        let project = node_project_with_deps(json!({ "@aws-sdk/client-s3": "^3.0.0" }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].category, ServiceCategory::Cloud);
    }

    #[test]
    fn a_project_with_no_external_services_produces_no_detections() {
        let project = node_project_with_deps(json!({ "lodash": "^4.0.0" }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert!(detections.is_empty());
    }

    #[test]
    fn multiple_categories_are_detected() {
        let project = node_project_with_deps(json!({
            "stripe": "^12.0.0",
            "nodemailer": "^6.9.0",
            "aws-sdk": "^2.0.0",
        }));
        let mut seen = HashSet::new();
        let mut detections = Vec::new();
        scan_node_dependencies(&project, &mut seen, &mut detections);

        assert_eq!(detections.len(), 3);
        let categories: HashSet<_> = detections.iter().map(|d| d.category).collect();
        assert!(categories.contains(&ServiceCategory::Payment));
        assert!(categories.contains(&ServiceCategory::Email));
        assert!(categories.contains(&ServiceCategory::Cloud));
    }

    #[test]
    fn a_clean_project_produces_no_checks() {
        let temp = std::env::temp_dir().join(format!(
            "sure-external-service-clean-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("package.json"), r#"{"name":"clean"}"#).unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let checks = ExternalServiceChecks::of(&discovery);
        assert!(checks.is_empty());
        assert!(checks.proposed().is_empty());
        assert!(checks.not_checked(&FingerprintId::generate()).is_empty());
    }

    #[test]
    fn env_var_prefix_is_detected_in_node_source() {
        assert!(line_has_env_prefix(
            "const key = process.env.STRIPE_SECRET_KEY;",
            &["STRIPE_"]
        ));
    }

    #[test]
    fn env_var_prefix_is_detected_in_python_source() {
        assert!(line_has_env_prefix(
            "key = os.environ['AWS_ACCESS_KEY_ID']",
            &["AWS_"]
        ));
    }

    #[test]
    fn env_var_prefix_is_detected_in_rust_source() {
        assert!(line_has_env_prefix(
            "let key = env::var(\"STRIPE_SECRET_KEY\");",
            &["STRIPE_"]
        ));
    }

    #[test]
    fn plain_prefix_without_env_accessor_is_not_detected() {
        assert!(!line_has_env_prefix(
            "// TODO: add STRIPE_ support",
            &["STRIPE_"]
        ));
    }

    #[test]
    fn require_is_detected() {
        assert!(line_has_import(
            "const stripe = require('stripe');",
            &["stripe"]
        ));
    }

    #[test]
    fn esm_import_is_detected() {
        assert!(line_has_import("import stripe from 'stripe';", &["stripe"]));
    }

    #[test]
    fn dynamic_import_is_detected() {
        assert!(line_has_import(
            "const mod = await import('stripe');",
            &["stripe"]
        ));
    }

    #[test]
    fn unrelated_string_is_not_detected_as_import() {
        assert!(!line_has_import("const x = 'stripe is cool';", &["stripe"]));
    }

    #[test]
    fn produced_check_does_not_aggregate_to_local_pass() {
        let fingerprint = FingerprintId::generate();
        let temp = std::env::temp_dir().join(format!(
            "sure-external-service-agg-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(
            temp.join("package.json"),
            r#"{"name":"fixture","dependencies":{"stripe":"^12.0.0"}}"#,
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let checks = ExternalServiceChecks::of(&discovery);
        assert!(!checks.is_empty());

        let results = checks.not_checked(&fingerprint);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(!result.status.is_green());
        assert!(
            result.blocks_green(),
            "a critical external service check that did not run must block green"
        );
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::ExternalServiceUnavailable)
        );
        assert_eq!(
            result.reason,
            NotCheckedReason::ExternalServiceUnavailable.plain_explanation()
        );
    }

    #[test]
    fn nothing_this_module_builds_is_refused_and_the_plan_holds_everything() {
        let temp = std::env::temp_dir().join(format!(
            "sure-external-service-plan-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(
            temp.join("package.json"),
            r#"{"name":"fixture","dependencies":{"stripe":"^12.0.0","aws-sdk":"^2.0.0"}}"#,
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let checks = ExternalServiceChecks::of(&discovery);
        assert_eq!(checks.proposed().len(), 2);

        let mut builder = PlanBuilder::new(
            sure_domain::execution::ExecutionMode::HostConfirmed,
            sure_domain::execution::ExecutionPermissions {
                connect_service: true,
                ..sure_domain::execution::ExecutionPermissions::inspect_only()
            },
        );
        checks.add_to(&mut builder);

        assert!(builder.refused().is_empty());
        let schedule = builder.build();
        assert_eq!(schedule.len(), 2);
        assert!(schedule.duplicates().is_empty());
    }

    #[test]
    fn a_blocked_external_service_check_stays_in_the_plan() {
        let temp = std::env::temp_dir().join(format!(
            "sure-external-service-blocked-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(
            temp.join("package.json"),
            r#"{"name":"fixture","dependencies":{"stripe":"^12.0.0"}}"#,
        )
        .unwrap();

        let discovery =
            crate::discover::discover(&temp, &crate::discover::DiscoverOptions::default()).unwrap();
        let checks = ExternalServiceChecks::of(&discovery);

        let mut builder = PlanBuilder::new(
            sure_domain::execution::ExecutionMode::InspectOnly,
            sure_domain::execution::ExecutionPermissions::inspect_only(),
        );
        checks.add_to(&mut builder);

        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.blocked().count(), 1);

        let blocked = schedule.blocked().next().unwrap();
        assert_eq!(
            blocked.proposal().requirements().actions(),
            &[ActionKind::ExternalService]
        );
        assert!(blocked.proposal().requirements().can_touch_network());
    }
}
