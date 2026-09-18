//! The frozen wire contract, pinned.
//!
//! P1-T002 acceptance: "Stable serialization tests exist for IDs/status/
//! evidence/intent/tier enums."
//!
//! "Stable" means pinned, not merely self-consistent. A test that asserts
//! `serde` agrees with `as_str()` still passes if both are renamed together, and
//! together is exactly how a wire name changes by accident. The names below are
//! written out as literals, so changing one is a visible edit to this file
//! rather than a side effect of editing a match arm.
//!
//! Two mechanisms keep this file honest:
//!
//! 1. Every `wire()` match below has no wildcard arm, so adding a variant to a
//!    frozen enum stops this file from compiling until someone decides what the
//!    new variant is called on the wire.
//! 2. `ALL` is declared by `variants!` next to each enum, and each test asserts
//!    the two lists have the same length, so a variant added to one list and not
//!    the other fails rather than going untested.
//!
//! Note what is *not* asserted: the order of variants. Serde is name-based, and
//! a reorder is not a wire change. Asserting order here would invent a
//! constraint the product does not have.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;

use sure_domain::capability::{BlindSpotKind, CapabilityTier, HookFailureBehaviour};
use sure_domain::evidence::{
    AnchorSubject, ClaimAssessment, EvidenceClass, Freshness, StalenessReason,
};
use sure_domain::execution::{
    ActionKind, CommandClass, ConsentGrantor, ExecutionDecision, ExecutionMode, Permission,
};
use sure_domain::finding::FindingStatus;
use sure_domain::ids::{IdKind, ProjectId};
use sure_domain::intent::{IntentSource, RequirementAuthority};
use sure_domain::severity::Severity;
use sure_domain::status::{
    AggregateSeverity, CheckStatus, CriticalState, NotCheckedReason, RequirementClaim,
};
use sure_domain::vocabulary::{FingerprintKind, Project, SupportLevel};

/// Pin the wire name of every variant of a unit-variant enum.
macro_rules! frozen {
    (
        $name:ident,
        test: $test_fn:ident,
        { $($variant:ident => $wire:literal),+ $(,)? }
    ) => {
        #[test]
        fn $test_fn() {
            /// The frozen wire name. No wildcard arm: a new variant must be
            /// named here before the crate compiles.
            fn wire(value: $name) -> &'static str {
                match value {
                    $( $name::$variant => $wire, )+
                }
            }

            let pinned: &[($name, &str)] = &[ $( ($name::$variant, $wire), )+ ];
            assert_eq!(
                $name::ALL.len(),
                pinned.len(),
                "{}::ALL and this table disagree: a variant was added to one and not the other",
                stringify!($name)
            );

            for (variant, expected) in pinned {
                let json = serde_json::to_value(variant).expect("enum variant must serialize");
                assert_eq!(
                    json,
                    Value::String((*expected).to_owned()),
                    "{}::{:?} no longer serializes as {expected:?}",
                    stringify!($name),
                    variant
                );
                assert_eq!(wire(*variant), *expected);
            }

            let mut names: Vec<&str> = pinned.iter().map(|(_, name)| *name).collect();
            names.sort_unstable();
            let total = names.len();
            names.dedup();
            assert_eq!(
                names.len(),
                total,
                "{} has two variants sharing one wire name, so a stored record cannot be read back",
                stringify!($name)
            );
        }
    };
}

frozen!(CapabilityTier, test: capability_tier_wire_names_are_frozen, {
    Snapshot => "snapshot",
    Observed => "observed",
    Protected => "protected",
});

frozen!(BlindSpotKind, test: blind_spot_wire_names_are_frozen, {
    UserGoalNotExposed => "user_goal_not_exposed",
    CompletionClaimNotExposed => "completion_claim_not_exposed",
    ToolCallsNotExposed => "tool_calls_not_exposed",
    FailuresNotExposed => "failures_not_exposed",
    FileEditsNotExposed => "file_edits_not_exposed",
    GitActivityNotExposed => "git_activity_not_exposed",
    NoPreActionControl => "no_pre_action_control",
    NoSessionVisibility => "no_session_visibility",
});

frozen!(HookFailureBehaviour, test: hook_failure_wire_names_are_frozen, {
    FailOpen => "fail_open",
    FailClosed => "fail_closed",
    NotApplicable => "not_applicable",
});

frozen!(EvidenceClass, test: evidence_class_wire_names_are_frozen, {
    ObservedFact => "observed_fact",
    DeterministicCheck => "deterministic_check",
    ModelAssessment => "model_assessment",
    Inference => "inference",
    Unknown => "unknown",
});

frozen!(AnchorSubject, test: anchor_subject_wire_names_are_frozen, {
    File => "file",
    Directory => "directory",
    LineRange => "line_range",
    Command => "command",
    Output => "output",
    Check => "check",
    Event => "event",
    Config => "config",
    Database => "database",
    Documentation => "documentation",
    Git => "git",
    Runtime => "runtime",
    Intent => "intent",
    Claim => "claim",
    Model => "model",
});

frozen!(ClaimAssessment, test: claim_assessment_wire_names_are_frozen, {
    Confirmed => "confirmed",
    Contradicted => "contradicted",
    CannotConfirm => "cannot_confirm",
    NotCheckable => "not_checkable",
});

frozen!(StalenessReason, test: staleness_reason_wire_names_are_frozen, {
    FingerprintChanged => "fingerprint_changed",
    SupersededByLaterChange => "superseded_by_later_change",
    UnknownProvenance => "unknown_provenance",
});

frozen!(ActionKind, test: action_kind_wire_names_are_frozen, {
    ReadFile => "read_file",
    ListDirectory => "list_directory",
    ReadMetadata => "read_metadata",
    StaticAnalysis => "static_analysis",
    RunTests => "run_tests",
    Build => "build",
    TypeCheck => "type_check",
    Lint => "lint",
    StartService => "start_service",
    LocalProbe => "local_probe",
    BrowserProbe => "browser_probe",
    BrowserObservation => "browser_observation",
    InstallDependencies => "install_dependencies",
    NetworkAccess => "network_access",
    WriteProjectFile => "write_project_file",
    DeleteProjectFile => "delete_project_file",
    ArbitraryCommand => "arbitrary_command",
    ExternalService => "external_service",
});

frozen!(Permission, test: permission_wire_names_are_frozen, {
    Inspect => "inspect",
    RunProjectCode => "run_project_code",
    InstallDependencies => "install_dependencies",
    Network => "network",
    WriteProject => "write_project",
    ConnectService => "connect_service",
});

frozen!(ExecutionMode, test: execution_mode_wire_names_are_frozen, {
    InspectOnly => "inspect_only",
    HostConfirmed => "host_confirmed",
    Container => "container",
});

frozen!(ConsentGrantor, test: consent_grantor_wire_names_are_frozen, {
    InteractiveUser => "interactive_user",
    UserConfiguration => "user_configuration",
    OrganizationPolicy => "organization_policy",
    ProjectRequestEscalated => "project_request_escalated",
});

frozen!(ExecutionDecision, test: execution_decision_wire_names_are_frozen, {
    Allowed => "allowed",
    NeedsConsent => "needs_consent",
    Denied => "denied",
});

frozen!(CommandClass, test: command_class_wire_names_are_frozen, {
    Static => "static",
    DynamicHost => "dynamic_host",
    Install => "install",
    Network => "network",
    Destructive => "destructive",
});

frozen!(IdKind, test: id_kind_wire_names_are_frozen, {
    Project => "project",
    Fingerprint => "fingerprint",
    Session => "session",
    Event => "event",
    Run => "run",
    Check => "check",
    Finding => "finding",
    Repair => "repair",
    Claim => "claim",
    Evidence => "evidence",
});

frozen!(IntentSource, test: intent_source_wire_names_are_frozen, {
    ExplicitUserGoal => "explicit_user_goal",
    ObservedUserRequest => "observed_user_request",
    ProjectSpec => "project_spec",
    AgentClaim => "agent_claim",
    Inferred => "inferred",
});

frozen!(RequirementAuthority, test: requirement_authority_wire_names_are_frozen, {
    UserRequirement => "user_requirement",
    DocumentedInstruction => "documented_instruction",
    AgentAssertion => "agent_assertion",
    NotARequirement => "not_a_requirement",
});

frozen!(Severity, test: severity_wire_names_are_frozen, {
    MustFix => "must_fix",
    ShouldFixFirst => "should_fix_first",
    CanFixLater => "can_fix_later",
    Note => "note",
});

frozen!(CheckStatus, test: check_status_wire_names_are_frozen, {
    Pass => "pass",
    Fail => "fail",
    Warning => "warning",
    Skipped => "skipped",
    Error => "error",
    Unknown => "unknown",
});

frozen!(CriticalState, test: critical_state_wire_names_are_frozen, {
    Passed => "passed",
    Failed => "failed",
    NotRun => "not_run",
    CheckerError => "checker_error",
    Uncertain => "uncertain",
});

frozen!(NotCheckedReason, test: not_checked_reason_wire_names_are_frozen, {
    ExecutionNotAuthorized => "execution_not_authorized",
    UserDeclined => "user_declined",
    DependencyInstallNotPermitted => "dependency_install_not_permitted",
    NetworkNotPermitted => "network_not_permitted",
    ToolUnavailable => "tool_unavailable",
    UnsupportedStack => "unsupported_stack",
    NotApplicable => "not_applicable",
    DisabledByConfiguration => "disabled_by_configuration",
    ExternalServiceUnavailable => "external_service_unavailable",
    AnalysisProviderDisabled => "analysis_provider_disabled",
    UnknownReason => "unknown_reason",
});

frozen!(AggregateSeverity, test: aggregate_severity_wire_names_are_frozen, {
    Green => "green",
    NeedsAttention => "needs_attention",
    NotReady => "not_ready",
    NotEnoughChecked => "not_enough_checked",
});

frozen!(RequirementClaim, test: requirement_claim_wire_names_are_frozen, {
    Comparable => "comparable",
    AfterTheFact => "after_the_fact",
});

frozen!(SupportLevel, test: support_level_wire_names_are_frozen, {
    FirstClass => "first_class",
    Generic => "generic",
    InspectOnly => "inspect_only",
});

frozen!(FingerprintKind, test: fingerprint_kind_wire_names_are_frozen, {
    Git => "git",
    Content => "content",
});

frozen!(FindingStatus, test: finding_status_wire_names_are_frozen, {
    Open => "open",
    Resolved => "resolved",
    AcceptedRisk => "accepted_risk",
    CannotConfirm => "cannot_confirm",
});

// --- enums the macro cannot cover ---------------------------------------

#[test]
fn freshness_serializes_a_stale_reason_rather_than_hiding_it() {
    // `Freshness` carries a payload, so it has no complete value list. It is
    // still part of the wire contract, and this match is exhaustive for the
    // same reason the macro's matches are.
    fn tag(value: &Freshness) -> &'static str {
        match value {
            Freshness::Fresh => "fresh",
            Freshness::Stale(_) => "stale",
        }
    }

    assert_eq!(
        serde_json::to_value(Freshness::Fresh).expect("serialize"),
        Value::String("fresh".to_owned()),
        "a unit variant is serialized by serde as a bare string"
    );
    assert_eq!(tag(&Freshness::Fresh), "fresh");

    // The reason is carried on the wire. A stale result that lost its reason
    // would tell a user "I cannot use this" without saying why.
    let stale = Freshness::Stale(StalenessReason::FingerprintChanged);
    let json = serde_json::to_value(stale).expect("serialize");
    assert_eq!(tag(&stale), "stale");
    assert!(
        json.to_string().contains("fingerprint_changed"),
        "the staleness reason must survive serialization, got {json}"
    );
}

// --- identifiers --------------------------------------------------------

#[test]
fn every_id_kind_has_the_frozen_prefix() {
    let pinned: &[(IdKind, &str)] = &[
        (IdKind::Project, "prj"),
        (IdKind::Fingerprint, "fp"),
        (IdKind::Session, "ses"),
        (IdKind::Event, "evt"),
        (IdKind::Run, "run"),
        (IdKind::Check, "chk"),
        (IdKind::Finding, "fnd"),
        (IdKind::Repair, "rep"),
        (IdKind::Claim, "clm"),
        (IdKind::Evidence, "evd"),
    ];
    assert_eq!(IdKind::ALL.len(), pinned.len());
    for (kind, prefix) in pinned {
        assert_eq!(kind.prefix(), *prefix);
        assert_eq!(IdKind::from_prefix(prefix), Some(*kind));
    }
}

#[test]
fn an_id_travels_as_a_plain_string() {
    let id = ProjectId::generate();
    let json = serde_json::to_value(&id).expect("serialize");
    assert_eq!(json, Value::String(id.as_str().to_owned()));
    let back: ProjectId = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, id);
}

#[test]
fn a_stored_id_survives_a_full_json_round_trip() {
    // The property that matters for evidence: an identifier read back from
    // storage is the same identifier, and a malformed one is refused rather
    // than accepted as a different kind of entity.
    let id = ProjectId::generate();
    let text = serde_json::to_string(&id).expect("serialize");
    let back: ProjectId = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(back, id);

    for bad in [
        "\"fnd_abc\"",
        "\"prj_\"",
        "\"prj_ABC\"",
        "\"\"",
        "\"prjabc\"",
        "\"prj_a-b\"",
    ] {
        assert!(
            serde_json::from_str::<ProjectId>(bad).is_err(),
            "{bad} must not deserialize as a project id"
        );
    }
}

#[test]
fn a_project_record_stored_before_support_existed_reads_as_unclassified() {
    // `Project::support` carries `#[serde(default)]`, and this is the claim that
    // makes it safe: a record written by a build that had no such field reads
    // back as **no answer**, not as an answer somebody made up on the way in.
    //
    // The distinction is invisible in `level` alone — `unrecorded` returns a real
    // level, precisely so that a missing answer never reads as the best one — so
    // it is asserted through `is_recorded`, which is the only door to it. The
    // field is removed from a real record rather than hand-written as JSON, so
    // this keeps testing the question if the record's shape changes.
    let project = Project::new(ProjectId::generate(), "shop", "C:\\shop");
    assert!(
        !project.support.is_recorded(),
        "a freshly built project record must carry no classification, or the field \
         removed below is not the only route to `unrecorded`"
    );
    let mut stored = serde_json::to_value(&project).expect("serialize");
    let removed = stored
        .as_object_mut()
        .expect("a project record is a JSON object")
        .remove("support");
    assert!(
        removed.is_some(),
        "`support` was not in the serialized record, so nothing below is being tested"
    );

    let back: Project = serde_json::from_value(stored).expect("deserialize");
    assert!(
        !back.support.is_recorded(),
        "a record with no support field claims a classification: {}",
        back.support.reason
    );
    assert_eq!(
        back.support.level,
        SupportLevel::InspectOnly,
        "and the level it leaves behind must be the weakest one, because a project \
         nobody has classified must never read as the best-supported one"
    );
}

// --- the schemas are the same contract ----------------------------------

fn repository_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("sure-domain lives at <root>/crates/sure-domain");
    assert!(
        root.join("schemas").is_dir(),
        "the schemas directory is the other half of this contract and must be present at {}",
        root.display()
    );
    root.to_path_buf()
}

/// Read the `enum` array at a `/`-separated JSON pointer.
fn schema_enum(file: &str, pointer: &str) -> Vec<String> {
    let path = repository_root().join("schemas").join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut node: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
    for step in pointer.split('/').filter(|s| !s.is_empty()) {
        node = node
            .get(step)
            .unwrap_or_else(|| panic!("{file} has no '{step}' in {pointer}"))
            .clone();
    }
    node.as_array()
        .unwrap_or_else(|| panic!("{file} {pointer} is not an array"))
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("{file} {pointer} holds a non-string"))
                .to_owned()
        })
        .collect()
}

#[test]
fn the_json_schemas_and_the_rust_enums_name_the_same_values() {
    // These two are the same contract written twice: the schemas are what an
    // integration validates against, the enums are what the core produces. A
    // schema that lists a value the core cannot emit, or is missing one it can,
    // is drift that would be discovered by a user rather than by a test.
    let cases: &[(&str, &str, Vec<String>)] = &[
        (
            "check-result.schema.json",
            "/properties/status/enum",
            CheckStatus::ALL
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect(),
        ),
        (
            "check-result.schema.json",
            "/properties/evidence_class/enum",
            EvidenceClass::ALL
                .iter()
                .map(|c| c.as_str().to_owned())
                .collect(),
        ),
        (
            "claim.schema.json",
            "/properties/assessment/enum",
            ClaimAssessment::ALL
                .iter()
                .map(|a| a.as_str().to_owned())
                .collect(),
        ),
        (
            "finding.schema.json",
            "/properties/severity/enum",
            Severity::ALL
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect(),
        ),
        (
            "finding.schema.json",
            "/properties/status/enum",
            FindingStatus::ALL
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect(),
        ),
        (
            "project-intent.schema.json",
            "/properties/source/enum",
            IntentSource::ALL
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect(),
        ),
        (
            "finding.schema.json",
            "/properties/evidence/items/properties/anchor/properties/subject/enum",
            AnchorSubject::ALL
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect(),
        ),
    ];

    for (file, pointer, expected) in cases {
        let mut declared = schema_enum(file, pointer);
        let mut expected = expected.clone();
        declared.sort();
        expected.sort();
        assert_eq!(
            declared, expected,
            "{file} {pointer} and the Rust enum have drifted apart"
        );
    }
}
