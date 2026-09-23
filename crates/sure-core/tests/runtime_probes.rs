//! `P5-T001`'s acceptance, checked from outside the crate.
//!
//! The task's sentence is one sentence:
//!
//! > *Runtime probes specify execution/network requirements and target
//! > component.*
//!
//! Three claims, and this file is arranged so that each is checked where it can
//! actually be false. **The requirements** are checked by asking the domain what
//! they mean — which permission the action costs, whether the check can reach
//! the network, and what [`decide`] says about it under a mode — rather than by
//! comparing them to a list written here, because a list written here would be
//! this module's own opinion read back to it. **The target component** is
//! checked against the [`ComponentGraph`], which is the only thing in the
//! product that says what a component is. And that a probe is **usable** — that
//! it is a check and not a description of one — is checked by handing it to a
//! `PlanBuilder` and reading what came out.
//!
//! # The five claims this file is arranged around
//!
//! **One: a probe says what it would do and where.** Over a real workspace on
//! disk, read by the product's own discovery, every probe's target is a
//! component the graph holds, and the title names it.
//!
//! **Two: the network answer is the domain's and not a pass-through.** This is
//! the finding the task turned up rather than the one it went looking for:
//! [`ActionKind::StartService`] requires
//! [`Permission::RunProjectCode`] and answers `can_touch_network() == true`, so
//! a probe that starts a project is one a user grants *run your own commands* to
//! and which can then reach the network. Both halves are asserted, and so is the
//! consequence — that [`Permission::Network`] is **not** among the permissions a
//! serve probe asks for, because requiring it to talk to `127.0.0.1` would ask
//! for something the check does not need.
//!
//! **Three: `auto` and `always` are not decorative.** The same project under the
//! three preferences, and the same answer to *is there anything here an
//! interface could be checked against* — nothing, which is why `auto` plans one
//! probe and not two, and says so rather than leaving a reader to assume.
//!
//! **Four: nothing is silently dropped.** Every readable manifest is either a
//! probe or a gap, in every configuration, and the two lists together account
//! for the whole project.
//!
//! **Five: the weight a probe carries reaches a verdict, and only one kind of
//! probe can hold a run out of green.** Three of the module's four weights are
//! arguments about consequence rather than about numbers, and the place a
//! consequence can be read is a result built from a schedule — so that is where
//! they are checked. A machine without a browser must be able to say so without
//! refusing a hand-off.
//!
//! # What is not claimed here
//!
//! Nothing here starts a service, opens a port or touches a browser, because
//! nothing in this module does either. What a probe would *do* is checked
//! through the actions it declares; what it would *not do* is that there is no
//! command line anywhere in the plan that did not come out of the project's own
//! manifest.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::checks::MissingKind;
use sure_core::checks::node::NodeChecks;
use sure_core::components::ComponentGraph;
use sure_core::config::{
    CheckPreference, ChecksConfig, Launcher, ScopeReduction, ServiceDeclaration,
};
use sure_core::discover::node::MANIFEST;
use sure_core::discover::{DiscoverOptions, Discovery, Ecosystem, Findings, discover};
use sure_core::planned_work::PlannedWork;
use sure_core::runtime_probes::{
    NotPlanned, NotPlannedBecause, PlanRefused, ProbeKind, ProbePlan, RuntimeProbe,
};
use sure_core::schedule::{CheckSchedule, PlanBuilder};
use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::{ExecutionDecision, ExecutionMode, ExecutionPermissions, Permission};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::CheckStatus;

/// A scratch project that removes itself.
///
/// Under the workspace's own `target/` so the fixture is on the same volume as
/// the checkout, with a space and a non-ASCII character in the path — the
/// cheapest way to make every path below one two platforms disagree about, which
/// is the discipline `CLAUDE.md` asks for.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let project = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure 探针 probes")
            .join(unique);
        std::fs::create_dir_all(&project)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", project.display()));
        Self { project }
    }

    /// Write a file inside the project, creating the directories above it.
    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    fn discover(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// The Node findings, or `None` when this is not a Node project.
    ///
    /// `None` is a value rather than a panic because one test below needs it and
    /// every other test must not accept it: a fixture that quietly stopped being
    /// a Node project would make every assertion after it vacuous.
    fn node_or_none(&self) -> Option<sure_core::discover::NodeProject> {
        node_of(&self.discover())
    }

    /// The Node findings, or a panic naming what was found instead.
    fn node(&self) -> sure_core::discover::NodeProject {
        self.node_or_none()
            .unwrap_or_else(|| panic!("this fixture is a Node project"))
    }

    /// A plan built the way a caller builds one: **one discovery, one graph, one
    /// project**. A fixture that discovered twice would be handing
    /// [`ProbePlan::of`] two readings of one directory and calling the result a
    /// plan, which is the shape the refusal test below is about.
    fn plan(&self, preferences: &ChecksConfig) -> ProbePlan {
        let discovery = self.discover();
        let graph = ComponentGraph::of(&discovery);
        ProbePlan::of(&graph, &self.node(), preferences).unwrap_or_else(|error| panic!("{error}"))
    }
}

/// The Node findings from one discovery result, or `None` if there are none.
fn node_of(found: &Discovery) -> Option<sure_core::discover::NodeProject> {
    let report = found.report(Ecosystem::Node)?;
    match &report.findings {
        Findings::Node(node) => Some((**node).clone()),
        other => panic!("the Node report carried {other:?}"),
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        drop(std::fs::remove_dir_all(&self.project));
    }
}

/// A root manifest that declares a way to start, and a member that declares a
/// different one.
///
/// Two components with two different commands, so that a test about *which*
/// command a probe would run has two answers to tell apart.
fn workspace(fixture: &Fixture) -> &Fixture {
    fixture
        .write(
            MANIFEST,
            r#"{"name":"root","workspaces":["packages/*"],
                "scripts":{"start":"node server.js","build":"tsc -b"}}"#,
        )
        .write(
            "packages/web/package.json",
            r#"{"name":"web","scripts":{"dev":"vite --host"}}"#,
        )
        .write("package-lock.json", "")
}

/// Every preference except the two this file varies.
fn preferences(services: CheckPreference, browser: CheckPreference) -> ChecksConfig {
    ChecksConfig {
        start_local_services: services,
        browser_probe: browser,
        ..ChecksConfig::default()
    }
}

/// The probe for one kind on one component, or a panic naming what is there.
fn probe<'a>(plan: &'a ProbePlan, kind: ProbeKind, target: &str) -> &'a RuntimeProbe {
    plan.probes()
        .iter()
        .find(|probe| probe.kind() == kind && probe.target() == target)
        .unwrap_or_else(|| {
            panic!(
                "no {kind:?} probe for {target}; the plan holds {:?}",
                plan.probes()
                    .iter()
                    .map(|probe| (probe.kind(), probe.target()))
                    .collect::<Vec<_>>()
            )
        })
}

/// The gap for one kind on one component, or a panic naming what is there.
fn gap<'a>(plan: &'a ProbePlan, kind: ProbeKind, target: &str) -> &'a NotPlannedBecause {
    plan.not_planned()
        .iter()
        .find(|gap| gap.kind() == kind && gap.component() == Path::new(target))
        .unwrap_or_else(|| panic!("no {kind:?} gap for {target}"))
        .because()
}

/// Host-confirmed, with the permissions a probe of this kind needs.
fn permitted(kind: ProbeKind) -> ExecutionPermissions {
    match kind {
        ProbeKind::Serve => ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        },
        ProbeKind::Interface => ExecutionPermissions {
            connect_service: true,
            ..ExecutionPermissions::inspect_only()
        },
    }
}

/// A `checks.services` declaration for one directory, as a project writes it.
///
/// The entry is written into the fixture rather than left dangling, so that this
/// file's projects pass the validation `service_plan` applies as well as the
/// rules this file is about.
fn declaring(directory: Option<&str>) -> ServiceDeclaration {
    ServiceDeclaration {
        name: "api".to_owned(),
        directory: directory.map(str::to_owned),
        launcher: Launcher::NodeEntry {
            entry: "server.js".to_owned(),
        },
        port: 4310,
        readiness: "/healthz".to_owned(),
        page: None,
    }
}

#[test]
fn a_probe_names_a_component_the_graph_holds_and_a_command_the_project_declared() {
    // The acceptance's third clause, over two components with two commands. A
    // target that the graph does not hold would be a probe about a place the
    // scan never found, and the assertion is made against the graph rather than
    // against a path written here.
    let fixture = Fixture::new("target");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    let discovery = fixture.discover();
    let graph = ComponentGraph::of(&discovery);
    assert!(!plan.probes().is_empty(), "no probes at all: {plan:?}");
    for probe in plan.probes() {
        assert!(
            graph.get(probe.component()).is_some(),
            "{:?} is not a component of this project, and a probe about it would \
             point at a place the scan did not find",
            probe.component()
        );
    }

    assert_eq!(
        probe(&plan, ProbeKind::Serve, "the project root")
            .proposal()
            .title(),
        "start the project"
    );
    assert_eq!(
        probe(&plan, ProbeKind::Serve, "packages/web")
            .proposal()
            .title(),
        "start the project in packages/web",
        "two components in one workspace printed one title"
    );

    // And the command is the project's own, rendered by the package manager:
    // the root declares `node server.js` and the member declares `vite --host`.
    let root = probe(&plan, ProbeKind::Serve, "the project root");
    let sentence = root.proposal().reason().plain_description();
    assert!(
        sentence.contains("npm start") && sentence.contains(MANIFEST),
        "the reason does not name the command SURE would run and the file it came \
         from: {sentence}"
    );
    let member = probe(&plan, ProbeKind::Serve, "packages/web");
    let sentence = member.proposal().reason().plain_description();
    assert!(
        sentence.contains("npm run dev") && sentence.contains("packages/web/package.json"),
        "the member's probe does not name the member's own command and manifest: \
         {sentence}"
    );
}

#[test]
fn a_probe_specifies_the_requirements_its_action_costs() {
    // The acceptance's first clause. The permission is read out of the domain's
    // own table rather than compared to a list written here: `Permission` is
    // what `decide` reads and what a consent prompt is written from, so the
    // claim that matters is that the probe's requirements carry exactly it.
    let fixture = Fixture::new("requirements");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    for kind in ProbeKind::ALL {
        let found = probe(&plan, *kind, "the project root");
        let requirements = found.proposal().requirements();
        let action = kind.action();
        assert_eq!(
            requirements.actions(),
            &[action],
            "a {kind:?} probe declares more or less than the one action it is"
        );
        assert_eq!(
            requirements.permissions_needed(),
            &[action.required_permission()],
            "a {kind:?} probe asks for a permission its action does not cost"
        );
        assert!(
            requirements.runs_project_code() == action.executes_project_code(),
            "a {kind:?} probe disagrees with the domain about whether it runs \
             project code"
        );
    }

    // The two kinds cost two different things, which is what makes them two
    // kinds rather than one with a flag.
    assert_eq!(
        probe(&plan, ProbeKind::Serve, "the project root")
            .proposal()
            .requirements()
            .permissions_needed(),
        &[Permission::RunProjectCode]
    );
    assert_eq!(
        probe(&plan, ProbeKind::Interface, "the project root")
            .proposal()
            .requirements()
            .permissions_needed(),
        &[Permission::ConnectService]
    );
}

#[test]
fn a_probe_carries_the_weight_it_argues_for_and_nothing_louder() {
    // The module argues three weights, one at a time, and each argument is about
    // what a user sees rather than about the number: a serve probe is `MustFix`
    // and critical because *a project that declares how to start itself and does
    // not start is a project that does not run*; an interface probe is
    // `ShouldFixFirst` and **not** critical because a browser SURE could not
    // drive is not a finding about the project; both are `ObservedFact` because
    // a probe's answer is not a function of the project state.
    //
    // **The middle one is why this reads through a result instead of stopping at
    // the proposal.** `severity == ShouldFixFirst` on the proposal is this file
    // agreeing with a sentence in that file. The claim worth holding is the
    // consequence one level down — a machine with no browser must not be able to
    // hold a hand-off — and that lives in `blocks_green`.
    let fixture = Fixture::new("weight");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    for kind in ProbeKind::ALL {
        assert_eq!(
            probe(&plan, *kind, "the project root")
                .proposal()
                .evidence_class(),
            EvidenceClass::ObservedFact,
            "a {kind:?} probe claims the project state determines its answer, which \
             is what the proposers next door can claim and this cannot: a probe \
             depends on the port, the install and the machine"
        );
    }

    let serve = probe(&plan, ProbeKind::Serve, "the project root").proposal();
    assert_eq!(serve.severity(), Severity::MustFix);
    assert!(
        serve.critical(),
        "a project that declares how to start itself and does not start does not \
         hold a run out of green"
    );

    let interface = probe(&plan, ProbeKind::Interface, "the project root").proposal();
    assert_eq!(interface.severity(), Severity::ShouldFixFirst);
    assert!(
        !interface.critical(),
        "an interface probe is critical, so a machine without a browser could \
         refuse a hand-off over a check SURE was never able to perform"
    );

    // The consequence, read out of the domain. Both probes are stopped by a mode
    // that authorizes nothing and get the same status; only one of them blocks.
    let mut builder = PlanBuilder::new(
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    );
    plan.add_to(&mut builder);
    let stopped: CheckSchedule = builder.build();

    let fingerprint = FingerprintId::generate();
    let blocked: Vec<bool> = ProbeKind::ALL
        .iter()
        .map(|kind| {
            let found = probe(&plan, *kind, "the project root");
            let scheduled = stopped
                .get(found.proposal().id())
                .expect("every probe reached the schedule");
            let result = scheduled
                .not_run(&fingerprint)
                .expect("an inspect-only mode stops every probe");
            assert_eq!(result.status, CheckStatus::Skipped);
            result.blocks_green()
        })
        .collect();
    assert_eq!(
        blocked,
        vec![true, false],
        "a probe's weight does not reach a verdict: {blocked:?}"
    );
}

#[test]
fn a_probe_declares_the_network_it_can_touch_and_not_the_permission_it_does_not_need() {
    // **The finding this task turned up.** `StartService` requires
    // `RunProjectCode` and answers `can_touch_network() == true` — cost and
    // capability are two questions and the domain answers them separately. Both
    // halves are asserted, because the temptation is to smooth one into the
    // other: a serve probe that reported `Permission::Network` would ask a user
    // to let a check use the internet when what it needs is to run their own
    // command, and one that reported no network capability would understate what
    // that command can do.
    let fixture = Fixture::new("network");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    for kind in ProbeKind::ALL {
        let requirements = probe(&plan, *kind, "the project root")
            .proposal()
            .requirements();
        assert!(
            requirements.can_touch_network(),
            "a {kind:?} probe says it cannot reach the network, and the domain says \
             {:?} can",
            kind.action()
        );
        assert!(
            !requirements
                .permissions_needed()
                .contains(&Permission::Network),
            "a {kind:?} probe asks for the internet in order to talk to this machine"
        );
    }

    // The serve probe's capability does not come from its permission, and this
    // is the pair of facts in one assertion so that a change to either fails
    // here rather than in a report.
    let serve = probe(&plan, ProbeKind::Serve, "the project root");
    assert_eq!(
        serve.proposal().requirements().permissions_needed(),
        &[Permission::RunProjectCode],
    );
    assert!(serve.proposal().requirements().can_touch_network());
}

#[test]
fn the_mode_and_the_permissions_decide_a_probe_and_this_module_does_not() {
    // The gating is `decide`'s and the schedule's, exactly as it is for the four
    // declared checks. Three situations, and the middle one is the one a
    // permission-only reading of the answer would get wrong: a check whose
    // permission has been granted and whose mode still refuses it comes back as
    // `NeedsConsent`, so a user who has already agreed to run project code is
    // told something true about what is stopping them.
    let fixture = Fixture::new("gating");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    let serve = probe(&plan, ProbeKind::Serve, "the project root")
        .proposal()
        .requirements()
        .clone();
    assert_eq!(
        serve.decision(
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only()
        ),
        ExecutionDecision::Denied
    );
    assert_eq!(
        serve.decision(ExecutionMode::InspectOnly, &permitted(ProbeKind::Serve)),
        ExecutionDecision::NeedsConsent,
        "a serve probe with its permission granted does not report that the mode \
         is what stops it"
    );
    assert_eq!(
        serve.decision(ExecutionMode::HostConfirmed, &permitted(ProbeKind::Serve)),
        ExecutionDecision::Allowed
    );

    // **The browser probe's middle answer is different, and the difference is
    // the two kinds being two kinds.** `decide` refuses an action under
    // inspect-only mode when that action executes project code, and
    // `BrowserProbe` does not — it drives a browser, which is not the project's
    // own code. So the mode does not stop an interface probe and the permission
    // is the whole of what does; a report that showed the two probes as stopped
    // by one thing would be telling a user to change the wrong setting.
    let interface = probe(&plan, ProbeKind::Interface, "the project root")
        .proposal()
        .requirements()
        .clone();
    assert_eq!(
        interface.decision(
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only()
        ),
        ExecutionDecision::Denied
    );
    assert_eq!(
        interface.decision(ExecutionMode::InspectOnly, &permitted(ProbeKind::Interface)),
        ExecutionDecision::Allowed,
        "an interface probe says the mode stops it, and the mode does not refuse \
         an action that runs none of the project's code"
    );
    assert_eq!(
        interface.decision(
            ExecutionMode::HostConfirmed,
            &permitted(ProbeKind::Interface)
        ),
        ExecutionDecision::Allowed
    );

    // The distinction in one place: one kind runs project code and one does not.
    assert!(serve.runs_project_code());
    assert!(!interface.runs_project_code());
}

#[test]
fn every_component_is_either_probed_or_explained_under_every_preference() {
    // The cover statement: a component in neither list is a component the report
    // says nothing about, and a reader takes that for the absence of a problem.
    // Checked under all six combinations of the two preferences rather than the
    // one somebody thought of.
    for services in CheckPreference::ALL {
        for browser in CheckPreference::ALL {
            let fixture = Fixture::new("cover");
            workspace(&fixture)
                .write("packages/api/package.json", r#"{"name":"api"}"#)
                .write("README.md", "a project\n");
            let plan = fixture.plan(&preferences(*services, *browser));

            let probed: Vec<(ProbeKind, String)> = plan
                .probes()
                .iter()
                .map(|probe| (probe.kind(), probe.target()))
                .collect();
            let explained: Vec<(ProbeKind, String)> = plan
                .not_planned()
                .iter()
                .map(|gap| (gap.kind(), gap.component().display().to_string()))
                .collect();

            for kind in ProbeKind::ALL {
                let enabled = !kind
                    .preference(&preferences(*services, *browser))
                    .disables();
                for target in ["", "packages/api", "packages/web"] {
                    let named = if target.is_empty() {
                        "the project root".to_owned()
                    } else {
                        target.to_owned()
                    };
                    let is_probed = probed
                        .iter()
                        .any(|(found, at)| found == kind && at.as_str() == named);
                    let is_explained = explained.iter().any(|(found, at)| {
                        found == kind && at.replace('\\', "/") == target.replace('\\', "/")
                    });
                    assert_eq!(
                        is_probed || is_explained,
                        enabled,
                        "{kind:?} on {named} under {services:?}/{browser:?}: probed \
                         {is_probed}, explained {is_explained}. Probed so far \
                         {probed:?}, explained so far {explained:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn auto_plans_a_service_and_says_why_it_plans_no_browser() {
    // The third clause this file is arranged around. `auto` is *run it when the
    // discovered project shape suggests it is worth running*, and the two kinds
    // can see different amounts of that shape: a `start` script is a fact about
    // the project, and *this project has a user interface* is not in any
    // manifest SURE reads.
    let fixture = Fixture::new("auto");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(CheckPreference::Auto, CheckPreference::Auto));

    assert!(
        plan.probes()
            .iter()
            .any(|probe| probe.kind() == ProbeKind::Serve),
        "`auto` did not plan a component that declares how to start"
    );
    assert!(
        !plan
            .probes()
            .iter()
            .any(|probe| probe.kind() == ProbeKind::Interface),
        "`auto` planned a browser probe, and no manifest says a project has an \
         interface"
    );

    // And the absence is a sentence rather than a hole.
    let because = gap(&plan, ProbeKind::Interface, "");
    assert_eq!(because, &NotPlannedBecause::AutoCannotSeeAnInterface);
    assert!(
        because
            .plain_description()
            .contains("checks.browser_probe: always"),
        "the gap does not say how to ask for the check: {}",
        because.plain_description()
    );
}

#[test]
fn always_is_the_only_way_to_ask_for_a_browser_and_never_is_not_a_silence() {
    let fixture = Fixture::new("preferences");
    workspace(&fixture);

    let asked_for = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));
    assert_eq!(
        asked_for
            .probes()
            .iter()
            .filter(|probe| probe.kind() == ProbeKind::Interface)
            .count(),
        2,
        "`always` did not plan a browser probe for both components"
    );

    let switched_off = fixture.plan(&preferences(CheckPreference::Never, CheckPreference::Never));
    assert!(switched_off.probes().is_empty());
    assert_eq!(
        switched_off.reductions(),
        &[
            ScopeReduction::LocalServicesDisabled,
            ScopeReduction::BrowserProbeDisabled,
        ],
        "a plan that probed nothing did not report both settings that stopped it"
    );
    assert!(
        switched_off.not_planned().is_empty(),
        "a check the user switched off was also reported as a gap, which reads as \
         something SURE could not do: {:?}",
        switched_off.not_planned()
    );

    // A reduction about the four declared checks is not this plan's to report:
    // one report would state the same reduction twice, in two layers' words.
    let also_no_existing_tests = fixture.plan(&ChecksConfig {
        existing_tests: false,
        ..preferences(CheckPreference::Never, CheckPreference::Auto)
    });
    assert!(
        !also_no_existing_tests
            .reductions()
            .contains(&ScopeReduction::ExistingTestsDisabled),
        "this plan reported a setting about the declared checks"
    );
}

#[test]
fn a_component_a_project_declares_a_service_for_is_left_to_the_declaration() {
    // The coverage rule, and it is about **where the rows come from** rather
    // than whether they exist: a declaration names a launcher, a port and a
    // readiness path, so the checks about the component it covers are planned
    // from that by `service_plan`, and planning them here as well would propose
    // one service twice. What must not happen is a reader taking this plan's
    // silence for a project SURE never looks at, which is why the absence is a
    // sentence.
    let fixture = Fixture::new("declared-service");
    workspace(&fixture).write("server.js", "// the service the declaration names\n");
    let both = preferences(CheckPreference::Always, CheckPreference::Always);

    let root_spoken_for = fixture.plan(&ChecksConfig {
        services: vec![declaring(None)],
        ..both.clone()
    });
    assert_eq!(
        root_spoken_for
            .probes()
            .iter()
            .map(|probe| (probe.kind(), probe.target()))
            .collect::<Vec<_>>(),
        vec![
            (ProbeKind::Serve, "packages/web".to_owned()),
            (ProbeKind::Interface, "packages/web".to_owned()),
        ],
        "a declaration for the project itself must take the root out of this plan and \
         leave the member where it was"
    );
    for kind in [ProbeKind::Serve, ProbeKind::Interface] {
        let because = gap(&root_spoken_for, kind, "");
        assert_eq!(
            because,
            &NotPlannedBecause::DeclaredAsAService,
            "the root's missing {kind:?} row was recorded as something else"
        );
        assert!(
            because.plain_description().contains("checks.services"),
            "the gap does not say where the rows went: {}",
            because.plain_description()
        );
    }
    assert!(
        root_spoken_for.reductions().is_empty(),
        "a declaration is not a setting this module reports as a reduction, which is \
         `Config::scope_reductions`'s to report: {:?}",
        root_spoken_for.reductions()
    );

    // And a declaration for a member is a statement about that member. The root
    // is the project itself, and a member's service says nothing about it.
    let member_spoken_for = fixture.plan(&ChecksConfig {
        services: vec![declaring(Some("packages/web"))],
        ..both
    });
    assert_eq!(
        member_spoken_for
            .probes()
            .iter()
            .map(|probe| (probe.kind(), probe.target()))
            .collect::<Vec<_>>(),
        vec![
            (ProbeKind::Serve, "the project root".to_owned()),
            (ProbeKind::Interface, "the project root".to_owned()),
        ],
        "a declaration for a member must not take the root out of this plan"
    );
    assert_eq!(
        gap(&member_spoken_for, ProbeKind::Serve, "packages/web"),
        &NotPlannedBecause::DeclaredAsAService,
        "the member's own rows are the declaration's to plan"
    );
}

#[test]
fn a_component_nothing_starts_is_a_gap_and_not_a_missing_row() {
    // `MissingCommand`'s shape one level up, and the same failure it guards
    // against: a report built from the plan would say nothing at all about a
    // component SURE cannot start, and a reader takes that for the absence of a
    // problem.
    let fixture = Fixture::new("nothing-starts-it");
    workspace(&fixture)
        .write(
            "packages/api/package.json",
            r#"{"name":"api","scripts":{"build":"tsc"}}"#,
        )
        .write(
            "packages/broken/package.json",
            r#"{"name":"broken","scripts":{"start":["node","server.js"]}}"#,
        );

    let plan = fixture.plan(&preferences(CheckPreference::Auto, CheckPreference::Auto));

    assert_eq!(
        gap(&plan, ProbeKind::Serve, "packages/api"),
        &NotPlannedBecause::NoCommand(MissingKind::NotDeclared),
        "a member that declares nothing to start it was not explained"
    );
    assert_eq!(
        gap(&plan, ProbeKind::Serve, "packages/broken"),
        &NotPlannedBecause::NoCommand(MissingKind::NotACommand),
        "a member that declared something unusable was described as one that \
         declared nothing"
    );
    assert!(
        plan.not_planned()
            .iter()
            .all(|gap| !gap.plain_description().trim().is_empty()),
        "a gap with no sentence is a row a report would print empty"
    );
    // `packages/api` has a `build` script and no way to start: the two roles are
    // different questions, and the gap is about starting.
    assert!(
        NodeChecks::of(&fixture.node(), &fixture.project)
            .planned()
            .iter()
            .map(PlannedWork::proposal)
            .any(|proposal| proposal.title() == "build the project in packages/api"),
        "the fixture's build script is not a declared check, so the gap above is \
         not evidence that starting and checking are different questions"
    );
}

#[test]
fn the_plan_reads_what_it_would_do_before_what_it_would_not() {
    // **A gap's line is a caveat and a probe's line is work**, and a reader who
    // skims the top of a report is reading whichever one comes first. This is the
    // one place in the module where the order of two lists is a decision rather
    // than an accident of a `Vec`, and the decision is written in
    // `ProbePlan::plain_description`'s own documentation.
    //
    // The fixture has both kinds of line on purpose: two components declare a way
    // to start and one declares a `build` script and nothing to serve with, so a
    // list that came back with either half first would still be a list.
    let fixture = Fixture::new("line-order");
    workspace(&fixture).write(
        "packages/api/package.json",
        r#"{"name":"api","scripts":{"build":"tsc"}}"#,
    );
    let plan = fixture.plan(&preferences(CheckPreference::Auto, CheckPreference::Auto));

    let would_do: Vec<String> = plan
        .probes()
        .iter()
        .map(RuntimeProbe::plain_description)
        .collect();
    let would_not: Vec<String> = plan
        .not_planned()
        .iter()
        .map(NotPlanned::plain_description)
        .collect();
    assert!(
        !would_do.is_empty() && !would_not.is_empty(),
        "this fixture does not have both kinds of line, so the order below is not \
         a claim about anything: {would_do:?} / {would_not:?}"
    );

    let lines = plan.plain_description();
    assert_eq!(
        lines.len(),
        would_do.len() + would_not.len(),
        "the plan's own list does not account for every probe and every gap"
    );
    let boundary = lines
        .iter()
        .position(|line| would_not.contains(line))
        .expect("the plan has gaps, so its list holds one");
    assert_eq!(
        boundary,
        would_do.len(),
        "a report states what SURE would not do before what it would do, so the \
         first thing a reader sees is an absence: {lines:?}"
    );
    assert!(
        lines[..boundary].iter().all(|line| would_do.contains(line)),
        "a line above the gaps is neither a probe's nor a gap's: {lines:?}"
    );
}

#[test]
fn a_probe_cannot_be_about_a_component_the_scan_did_not_find() {
    // Two values that came from one discovery answer; two that did not are
    // refused. **The refusal is checked rather than described**, because a rule
    // that passes only when nothing exercises it is the false green this
    // repository ranks above a visible error — and nothing in the type system
    // ties the graph to the project.
    let fixture = Fixture::new("disagreement");
    workspace(&fixture);

    let elsewhere = Fixture::new("elsewhere");
    elsewhere.write("README.md", "an empty project\n");
    let other = elsewhere.discover();

    let error = ProbePlan::of(
        &ComponentGraph::of(&other),
        &fixture.node(),
        &ChecksConfig::default(),
    )
    .expect_err("a graph from another project did not refuse this one's manifests");

    // The member rather than the root, and that is not an accident of this
    // fixture: **every component graph has a root**, so the root's directory is
    // always found and the first disagreement between two projects is always a
    // member. Asserting the root here would have been a test that passes for the
    // wrong reason.
    match &error {
        PlanRefused::NoSuchComponent {
            manifest,
            component,
        } => {
            assert_eq!(manifest, "packages/web/package.json");
            assert_eq!(component, "packages/web");
        }
    }
    let sentence = error.to_string();
    assert!(
        sentence.contains("packages/web/package.json") && sentence.contains("packages/web"),
        "the refusal does not name both halves of the disagreement: {sentence}"
    );
}

#[test]
fn nothing_this_module_builds_is_refused_and_every_identifier_is_distinct() {
    // The seam, from outside: a probe is a check, so it goes into a plan with
    // nothing refused and nothing merged away as a duplicate. If this module had
    // grown its own idea of what a check is, this is where it would show.
    let fixture = Fixture::new("seam");
    workspace(&fixture).write(
        "packages/api/package.json",
        r#"{"name":"api","scripts":{"start":"node api.js"}}"#,
    );
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Always,
    ));

    let mut builder = PlanBuilder::new(
        ExecutionMode::HostConfirmed,
        ExecutionPermissions {
            run_project_code: true,
            connect_service: true,
            ..ExecutionPermissions::inspect_only()
        },
    );
    plan.add_to(&mut builder);
    assert!(
        builder.refused().is_empty(),
        "the probes layer proposed something the builder refused: {:?}",
        builder.refused()
    );
    let schedule: CheckSchedule = builder.build();
    assert_eq!(schedule.len(), plan.probes().len());
    assert!(
        schedule.duplicates().is_empty(),
        "{:?}",
        schedule.duplicates()
    );

    // Distinct identifiers, which is what stops a member's probe being dropped
    // as a duplicate of the root's.
    let mut ids: Vec<&str> = plan
        .probes()
        .iter()
        .map(|probe| probe.proposal().id().as_str())
        .collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "two probes share an identifier: {ids:?}");

    for probe in plan.probes() {
        assert!(
            schedule.get(probe.proposal().id()).is_some(),
            "a probe did not reach the schedule"
        );
    }

    // **Every probe the mode stops comes back as the one status this crate can
    // produce for a check that did not run**, and never as a pass: the status is
    // `CheckResult::not_run`'s and `ScheduledCheck::not_run` answers `None` for a
    // check that would run, so there is no path from this module to a verdict.
    let mut stopped = PlanBuilder::new(
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    );
    plan.add_to(&mut stopped);
    let stopped = stopped.build();
    let fingerprint = FingerprintId::generate();
    let mut skipped = 0;
    for probe in plan.probes() {
        let scheduled = stopped
            .get(probe.proposal().id())
            .expect("every probe reached the schedule");
        let result = scheduled
            .not_run(&fingerprint)
            .expect("the inspect-only mode stops every probe");
        assert_eq!(result.status, CheckStatus::Skipped);
        skipped += 1;
    }
    assert!(skipped > 0, "the plan held no probes to gate");
}

#[test]
fn the_same_project_is_probed_under_the_same_identifiers_every_time() {
    // The precondition of the repair contract in `docs/architecture/CHECK_PIPELINE.md`:
    // a result from one run is joined to the plan of another by identity. Read
    // twice from disk rather than compared against a constant, so the claim is
    // about the recipe rather than about one run of it.
    let fixture = Fixture::new("identity");
    workspace(&fixture);

    // `auto` for both readings, so that the two plans hold the same kinds.
    let auto = preferences(CheckPreference::Auto, CheckPreference::Auto);
    let first = fixture.plan(&auto);
    let second = fixture.plan(&auto);

    let ids = |plan: &ProbePlan| -> Vec<String> {
        plan.probes()
            .iter()
            .map(|probe| probe.proposal().id().as_str().to_owned())
            .collect()
    };
    assert_eq!(ids(&first), ids(&second));
    assert!(!ids(&first).is_empty());

    // And a renamed role does not rename the check: `start` and `dev` answer the
    // same question about a component, so a project that switched from one to
    // the other has not changed which check it is.
    let renamed = Fixture::new("renamed");
    renamed
        .write(
            MANIFEST,
            r#"{"name":"root","workspaces":["packages/*"],"scripts":{"dev":"node server.js"}}"#,
        )
        .write(
            "packages/web/package.json",
            r#"{"name":"web","scripts":{"start":"vite --host"}}"#,
        )
        .write("package-lock.json", "");
    let renamed = renamed.plan(&auto);

    assert!(
        !renamed.probes().is_empty(),
        "the renamed fixture planned nothing, so the comparison below is vacuous"
    );
    assert_eq!(
        ids(&renamed),
        ids(&first),
        "which role supplied the command changed which check it is, so a stored \
         result would be orphaned by a rename"
    );
    // The commands themselves are different, which is what makes the identity
    // claim above a claim about something: the root switched from `start` to
    // `dev` and the member the other way.
    let root = probe(&renamed, ProbeKind::Serve, "the project root");
    assert!(
        root.proposal()
            .reason()
            .plain_description()
            .contains("npm run dev"),
        "the renamed root does not run the command it now declares: {}",
        root.proposal().reason().plain_description()
    );
}

#[test]
fn a_project_that_is_not_a_node_project_gets_no_probes_and_neither_do_its_gaps() {
    // The empty case, and it is two different things: a project with no
    // `package.json` at all has nothing to say here, and so does one whose
    // manifest SURE could not read — a manifest that could not be read is not a
    // manifest that is absent, and neither is a manifest that declared nothing.
    let empty = Fixture::new("not-node");
    empty.write("README.md", "a project with no manifest\n");
    // The caller has no `NodeProject` to hand `ProbePlan::of` at all, which is
    // the honest shape of it: discovery looked, found no `package.json`, and a
    // probe planner is not something this project needs. Asserted as `None`
    // rather than reached through a plan, because "there is nothing to plan" and
    // "a plan that came out empty" are different claims and only the first is
    // true here.
    assert!(
        empty.node_or_none().is_none(),
        "a project with no `package.json` was reported as a Node project"
    );

    let unreadable = Fixture::new("unreadable");
    unreadable.write(MANIFEST, "{ this is not json");
    let plan = unreadable.plan(&ChecksConfig::default());
    assert!(
        plan.is_empty(),
        "a manifest SURE could not read produced probes or gaps: {plan:?}"
    );

    let declares_nothing = Fixture::new("declares-nothing");
    declares_nothing.write(MANIFEST, r#"{"name":"quiet"}"#);
    let plan = declares_nothing.plan(&ChecksConfig::default());
    assert!(
        !plan.is_empty(),
        "a readable manifest produced nothing at all"
    );
    assert!(plan.probes().is_empty());

    // **Two gaps, and the second one is not the `auto` gap.** A project SURE has
    // no way to start cannot be browsed either — SURE does not browse an app
    // nobody is serving — so both kinds report the missing command, and
    // *nothing here starts it* is the stronger and more useful fact. The `auto`
    // sentence about interfaces only reaches a component that serves, which is
    // why this fixture has two `NoCommand` gaps rather than one of each.
    assert_eq!(plan.not_planned().len(), 2, "{plan:?}");
    for gap in plan.not_planned() {
        assert_eq!(
            gap.because(),
            &NotPlannedBecause::NoCommand(MissingKind::NotDeclared),
            "{:?} reported something other than the missing command",
            gap.kind()
        );
    }
}

#[test]
fn a_status_can_only_be_produced_by_the_domain_and_not_by_this_module() {
    // A source rule over the module this task added, because the claim is about
    // what the file does *not* contain: nothing here can turn a probe into a
    // verdict. Every status a probe could have is produced by
    // `CheckResult::not_run` through `ScheduledCheck::not_run`, one level down.
    let text = std::fs::read_to_string(
        sure_testkit::repository_root().join("crates/sure-core/src/runtime_probes.rs"),
    )
    .expect("the module this task added is in the tree");

    let shipped = text
        .split_once("\n#[cfg(test)]")
        .map(|(shipped, _)| shipped)
        .expect("the module has a test module, so the split is a fact about the file");

    // The forbidden names are the outcome vocabulary and one constructor. **The
    // "runs nothing" half is deliberately not restated here**: `tests/spawn_sites.rs`
    // counts the callers of `Command::new` over the whole workspace and this file
    // is inside that walk, so a second rule would be a second answer to a
    // question that already has one.
    for forbidden in ["CheckStatus", "CheckResult", "CheckId::generate"] {
        assert!(
            !shipped.contains(forbidden),
            "the shipped half of `runtime_probes.rs` names {forbidden}, which means \
             something here can decide an outcome or invent an identity rather than \
             describe a check"
        );
    }
    // And the two names it *does* have to reach for, so that the rule above is
    // not passing because the file was split at the wrong place.
    for required in ["CheckProposal", "ActionKind"] {
        assert!(
            shipped.contains(required),
            "the shipped half of `runtime_probes.rs` does not name {required}, so \
             the rule above is checking the wrong text"
        );
    }

    // The `CheckStatus` vocabulary the rule is about, named so that the rule
    // above cannot go stale if the enum is renamed.
    assert!(CheckStatus::ALL.len() > 1);
}
