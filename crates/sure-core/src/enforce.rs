//! Applying the execution mode to the checks a run intends to perform.
//!
//! # Why this module exists
//!
//! `inspect_only` has been true of SURE so far for a reason that will not last:
//! **nothing in this repository launches a project process.** The census in
//! `tests/spawn_sites.rs` counts three places that build a
//! [`Command`](std::process::Command), all of them inside the runner's own
//! machinery, and `support::CEILING` is justified by *no project code running*.
//! That is a fact about this build, not a property of the mode, and the day the
//! first check is wired to the runner the fact stops being true. The census test
//! says in its own documentation that it is written to fail on that day.
//!
//! This module is what has to be in place before that day. It turns "nothing
//! runs, because there is nothing to run it" into **"nothing runs, because the
//! plan says so"** — a value that a runner has to consult rather than an absence
//! it happens to fall into.
//!
//! # What it decides
//!
//! Given a [`PermissionPlan`] and the list of checks a run intends to perform,
//! it produces a [`CheckPlan`] whose three lists are the answer:
//!
//! - A check every one of whose commands may run, and none of which runs project
//!   code, goes to `static_checks`.
//! - A check every one of whose commands may run, where at least one of them
//!   runs project code, goes to `dynamic_checks`.
//! - A check with **any** command that may not run is *excluded*, with that
//!   command's own reason, and a [`CheckResult`] is produced for it. It appears
//!   in neither list.
//!
//! A check is a unit, and the third rule is why: a check whose static half may
//! run and whose dynamic half may not is not half-checkable. Admitting it would
//! produce a result for a check that did not happen, which is the one outcome
//! this repository is built to avoid — the allowed half contributes no verdict,
//! and running it would be work done for a result nobody will read.
//!
//! # The classification is made before the refusal, and the order is the point
//!
//! A check is put in `static_checks` or `dynamic_checks` according to what its
//! commands **would** use, and only then removed if one of them will not run. So
//! a check that would run the project's code in a mode that runs nothing lands
//! in `dynamic_checks` and is *immediately* excluded from it, with the reason
//! the command gave. Doing it the other way round — deciding the refusal first —
//! would leave a report unable to distinguish "this check reads files" from
//! "this check would have run your code and was stopped", and the second is the
//! sentence a user most needs to see.
//!
//! # The question nobody is asked
//!
//! [`ExecutionDecision::NeedsConsent`](sure_domain::execution::ExecutionDecision)
//! means there is a question worth asking: the mode is too cautious rather than a
//! permission being missing. **This module does not ask it.** It has no prompt, no
//! user and no way to wait, so a command in that state is stopped exactly like a
//! denied one, under the reason the command already carries —
//! [`NotCheckedReason::ExecutionNotAuthorized`], which is the vocabulary's own
//! answer for "the mode did not allow this". Turning a question into a stop is the
//! cautious direction, and the alternative — carrying it forward as a check that
//! might run — would put a check in a plan that nothing will ever run.
//!
//! # What this does not do
//!
//! **It cannot stop code that does not ask it.** A caller that reaches the runner
//! by some other route is not constrained by anything here, and the enforcement
//! that matters is therefore a rule about the *caller*: the first check to run
//! project code must take its command lines from [`Enforcement::admitted`] and
//! from nowhere else, and the census in `tests/spawn_sites.rs` is where that
//! becomes testable. Until then this module is a gate with no road through it,
//! which is the same position [`crate::approval`] is in and for the same reason.
//!
//! **It says nothing about what a command does when it runs.** The classification
//! is `safety::classify`'s, and its own caveat — SURE reads command lines, not
//! processes — is unchanged by being consulted here.
//!
//! **It does not make `inspect_only` a sandbox.** The mode is a promise about
//! what SURE will start, not a boundary around what a started process can reach.
//! `docs/architecture/EXECUTION_SAFETY.md` is where that distinction is stated.

use sure_domain::execution::ExecutionMode;
use sure_domain::ids::CheckId;
use sure_domain::status::CheckResult;
use sure_domain::vocabulary::CheckPlan;

use crate::consent::{self, PermissionPlan, PlannedCheck, PlannedCommand};

/// A permission plan with the execution mode already applied to it.
///
/// Built by [`Enforcement::of`], which is the only constructor: the three lists
/// of the check plan, the results for the checks that were stopped, and the
/// commands that may run are three views of one decision, and a type that let
/// them be assembled separately would be a type that let them disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enforcement {
    /// Held rather than borrowed, so that the commands `admitted` indexes into
    /// cannot be added to after the decision was made.
    permissions: PermissionPlan,
    checks: CheckPlan,
    stopped: Vec<CheckResult>,
    /// Indices into `permissions.commands()`, in that order.
    admitted: Vec<usize>,
    unscheduled: Vec<CheckId>,
}

impl Enforcement {
    /// Apply `permissions`' mode to the checks a run intends to perform.
    ///
    /// `scheduled` is the caller's list of checks, in the order a report should
    /// show them — and it is the caller's because a check that runs nothing has
    /// no command to be planned and so cannot be discovered from the plan. A
    /// check scheduled twice is scheduled once; the second occurrence would
    /// otherwise put the same id in a list twice and produce a second result for
    /// one check.
    ///
    /// # A command for a check that was not scheduled never runs
    ///
    /// The plan and the schedule are built from two different places, so they can
    /// disagree, and the disagreement is reported by [`Self::unscheduled`] rather
    /// than absorbed. Such a command is not admitted: running it would be work
    /// done for a check that is not in the plan, and so for a result no report
    /// will ever read. **The second acceptance sentence asks that dynamic checks
    /// remain not-run, and a command that runs is one that ran whatever the
    /// plan's lists say** — this is the one path by which a plan could otherwise
    /// have run something it never listed.
    #[must_use]
    pub fn of(
        id: impl Into<String>,
        permissions: PermissionPlan,
        scheduled: &[PlannedCheck],
    ) -> Self {
        let fingerprint = permissions.fingerprint().clone();
        let mut checks = CheckPlan::new(id, fingerprint.clone(), permissions.mode());
        let mut stopped = Vec::new();
        let mut admitted_checks: Vec<CheckId> = Vec::new();
        let mut scheduled_ids: Vec<CheckId> = Vec::new();

        for check in scheduled {
            if scheduled_ids.contains(check.id()) {
                continue;
            }
            scheduled_ids.push(check.id().clone());
            let commands = commands_of(&permissions, check.id());

            // Where the check lands if it runs at all: by what its commands
            // would use, and not by whether they may. A check that would run the
            // project's code belongs in `dynamic_checks` even in a mode that
            // will not let it, because that is what the report has to be able to
            // say about it once it is stopped.
            if commands
                .iter()
                .any(|command| consent::runs_project_code(command.effects()))
            {
                checks.dynamic_checks.push(check.id().clone());
            } else {
                checks.static_checks.push(check.id().clone());
            }

            // And then, if any of its commands will not run, it leaves that list
            // with the reason the command already carried — no reason is made up
            // here. `exclude` returning `false` would mean the check was never in
            // the plan, which cannot happen for an id pushed two lines above; the
            // result is pushed only when the reason was, so the two lists stay in
            // step by construction rather than by a rule to remember.
            let refusal = commands.iter().find_map(|command| {
                let reason = command.reason()?;
                Some((reason, command.refusal(&fingerprint)?))
            });
            match refusal {
                Some((reason, result)) => {
                    if checks.exclude(check.id(), reason) {
                        stopped.push(result);
                    }
                }
                None => admitted_checks.push(check.id().clone()),
            }
        }

        let admitted = permissions
            .commands()
            .iter()
            .enumerate()
            .filter(|(_, command)| admitted_checks.contains(command.check().id()))
            .map(|(index, _)| index)
            .collect();

        let mut unscheduled = Vec::new();
        for command in permissions.commands() {
            let check = command.check().id();
            if !scheduled_ids.contains(check) && !unscheduled.contains(check) {
                unscheduled.push(check.clone());
            }
        }

        Self {
            permissions,
            checks,
            stopped,
            admitted,
            unscheduled,
        }
    }

    /// The mode this enforcement was built under.
    ///
    /// Read from the plan rather than passed alongside it, because a mode that
    /// could differ from the one the commands were decided under is a mode that
    /// could disagree with every decision below it.
    #[must_use]
    pub const fn mode(&self) -> ExecutionMode {
        self.permissions.mode()
    }

    /// The plan this enforcement was built from.
    #[must_use]
    pub const fn permission_plan(&self) -> &PermissionPlan {
        &self.permissions
    }

    /// The plan a report is built from.
    #[must_use]
    pub const fn check_plan(&self) -> &CheckPlan {
        &self.checks
    }

    /// A result for every check the mode stopped, in the order it was scheduled.
    ///
    /// Every one is `Skipped` with a reason, and `checks.excluded` holds the same
    /// reasons in the same order. **This is the second acceptance sentence's
    /// evidence**: a check that a mode stopped is not a check that ran, and
    /// nothing here can produce a passing result — the only constructor reached
    /// is [`CheckResult::not_run`], through [`PlannedCommand::refusal`].
    #[must_use]
    pub fn stopped(&self) -> &[CheckResult] {
        &self.stopped
    }

    /// The commands that may run, in the order they were planned.
    ///
    /// **This is the only door.** A runner wired into this product must take what
    /// it launches from here rather than from [`PermissionPlan::commands`], which
    /// is every command the plan considered — including the ones this decision
    /// stopped. A command that is not in this iterator has not been admitted, and
    /// the difference between the two lists is the whole of the enforcement.
    pub fn admitted(&self) -> impl Iterator<Item = &PlannedCommand> {
        self.admitted
            .iter()
            .filter_map(|index| self.permissions.commands().get(*index))
    }

    /// Checks with a planned command that was not scheduled, if any.
    ///
    /// An anomaly rather than a state to live with: a command planned against a
    /// check that was not scheduled is a plan and a schedule built from two
    /// different check sets. Empty is the expected answer, and it is the caller's
    /// to report — which is what [`PermissionPlan::exclude_refused_from`] does
    /// with its own version of the same anomaly.
    #[must_use]
    pub fn unscheduled(&self) -> &[CheckId] {
        &self.unscheduled
    }

    /// Whether any admitted command runs code the project controls.
    ///
    /// The predicate is [`consent::runs_project_code`]'s and not a second copy of
    /// it: the mode rule that decides a command needs asking and the rule that
    /// decides a check is dynamic have to be about the same set of categories, and
    /// a copy here is where they would drift. **This is the property
    /// `support::CEILING` will be able to rest on** once a runner exists — a value
    /// saying that nothing admitted runs project code, rather than the absence of
    /// a runner saying it.
    #[must_use]
    pub fn runs_project_code(&self) -> bool {
        self.admitted()
            .any(|command| consent::runs_project_code(command.effects()))
    }

    /// What a report should say about this decision.
    #[must_use]
    pub fn explain(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Execution mode: {}. {}",
            self.mode().as_str(),
            self.mode().plain_description()
        )];
        lines.push(format!(
            "Checks: {} read only, {} would run your project's code, {} stopped by the mode.",
            self.checks.static_checks.len(),
            self.checks.dynamic_checks.len(),
            self.stopped.len()
        ));
        lines.push(format!(
            "Commands: {} may run out of the {} this plan considered.",
            self.admitted.len(),
            self.permissions.commands().len()
        ));
        for result in &self.stopped {
            lines.push(format!("Not checked: {}. {}", result.title, result.reason));
        }
        for check in &self.unscheduled {
            lines.push(format!(
                "A command was planned for check {check}, which is not in this plan's \
                 schedule. It will not run and no result will be reported for it."
            ));
        }
        lines
    }
}

/// The planned commands that belong to one check, in plan order.
fn commands_of<'a>(plan: &'a PermissionPlan, check: &CheckId) -> Vec<&'a PlannedCommand> {
    plan.commands()
        .iter()
        .filter(|command| command.check().id() == check)
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::execution::{CommandClass, ExecutionPermissions, Permission};
    use sure_domain::ids::FingerprintId;
    use sure_domain::severity::Severity;
    use sure_domain::status::{CheckStatus, NotCheckedReason};

    /// One command per category, with the set it is expected to classify as.
    ///
    /// The expectations are written out rather than derived, so that a change in
    /// `safety.rs` shows up here as a failure instead of silently changing what
    /// the matrix below is a matrix *over*. `CommandClass::ALL`'s own order.
    const FIXTURES: &[(&str, &[&str], &[CommandClass])] = &[
        ("git", &["status"], &[CommandClass::Static]),
        ("python", &["-m", "pytest"], &[CommandClass::DynamicHost]),
        (
            "cargo",
            &["add", "serde"],
            &[CommandClass::Install, CommandClass::Network],
        ),
        ("git", &["fetch"], &[CommandClass::Network]),
        (
            "git",
            &["push", "--force", "origin", "main"],
            &[CommandClass::Network, CommandClass::Destructive],
        ),
        (
            "frobnicate",
            &["--everything"],
            &[
                CommandClass::DynamicHost,
                CommandClass::Install,
                CommandClass::Network,
                CommandClass::Destructive,
            ],
        ),
    ];

    const COMMANDS: &[(&str, &[&str])] = &[
        ("git", &["status"]),
        ("python", &["-m", "pytest"]),
        ("cargo", &["add", "serde"]),
        ("git", &["fetch"]),
        ("git", &["push", "--force", "origin", "main"]),
        ("frobnicate", &["--everything"]),
    ];

    fn check(title: &str) -> PlannedCheck {
        PlannedCheck::new(CheckId::generate(), title, Severity::MustFix, true)
    }

    fn everything() -> ExecutionPermissions {
        let mut permissions = ExecutionPermissions::inspect_only();
        for permission in Permission::ALL {
            permissions.set(*permission, true);
        }
        permissions
    }

    /// Every permission set the matrix runs over, named for its failures.
    fn permission_sets() -> Vec<(String, ExecutionPermissions)> {
        let mut sets = vec![
            (
                "inspect_only".to_owned(),
                ExecutionPermissions::inspect_only(),
            ),
            (
                "host_confirmed baseline".to_owned(),
                ExecutionMode::HostConfirmed.baseline_permissions(),
            ),
            ("everything granted".to_owned(), everything()),
        ];
        for permission in Permission::ALL {
            let mut permissions = ExecutionPermissions::inspect_only();
            permissions.set(*permission, true);
            sets.push((
                format!("inspect_only plus {}", permission.as_str()),
                permissions,
            ));
        }
        sets
    }

    fn enforcement_of(
        mode: ExecutionMode,
        permissions: ExecutionPermissions,
        commands: &[(&str, &[&str])],
    ) -> (Enforcement, Vec<CheckId>) {
        let mut plan = PermissionPlan::new(mode, FingerprintId::generate(), permissions);
        let mut scheduled = Vec::new();
        for (program, arguments) in commands {
            let scheduled_check = check(program);
            plan.add(scheduled_check.clone(), *program, arguments.to_vec());
            scheduled.push(scheduled_check);
        }
        let ids = scheduled.iter().map(|c| c.id().clone()).collect();
        (Enforcement::of("plan", plan, &scheduled), ids)
    }

    #[test]
    fn each_fixture_is_the_category_set_it_claims() {
        for (program, arguments, expected) in FIXTURES {
            let classification = crate::safety::classify(*program, arguments.to_vec());
            assert_eq!(
                classification.effects().classes(),
                *expected,
                "{program} {arguments:?} is not what this table says it is"
            );
        }
    }

    #[test]
    fn every_category_the_vocabulary_has_has_a_fixture() {
        // The matrix below is a matrix over these commands. A sixth category
        // added to `CommandClass::ALL` would otherwise be a category the matrix
        // silently stopped covering, which is how a category ends up with no
        // test that it can be refused.
        let mut seen: Vec<CommandClass> = Vec::new();
        for (_, _, classes) in FIXTURES {
            for class in *classes {
                if !seen.contains(class) {
                    seen.push(*class);
                }
            }
        }
        for class in CommandClass::ALL {
            assert!(
                seen.contains(class),
                "no fixture classifies as {}",
                class.as_str()
            );
        }
    }

    #[test]
    fn a_check_that_runs_no_project_code_is_static_and_its_command_may_run() {
        let (enforcement, ids) = enforcement_of(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
            &[("git", &["status"])],
        );
        assert_eq!(enforcement.check_plan().static_checks, ids);
        assert!(enforcement.check_plan().dynamic_checks.is_empty());
        assert!(enforcement.check_plan().excluded.is_empty());
        assert!(enforcement.stopped().is_empty());
        assert_eq!(enforcement.admitted().count(), 1);
        assert!(!enforcement.runs_project_code());
        assert!(enforcement.unscheduled().is_empty());
    }

    #[test]
    fn a_mode_that_runs_nothing_stops_a_dynamic_check_whatever_the_permissions_say() {
        // The second acceptance sentence, and the one matrix that has to hold for
        // it to mean anything: no permission set reaches it, because the rule that
        // stops it is about the mode and is consulted after the permissions.
        for (name, permissions) in permission_sets() {
            let (enforcement, ids) = enforcement_of(
                ExecutionMode::InspectOnly,
                permissions,
                &[("python", &["-m", "pytest"])],
            );
            let plan = enforcement.check_plan();
            assert!(
                plan.dynamic_checks.is_empty(),
                "{name}: a dynamic check survived a mode that runs nothing"
            );
            assert!(plan.static_checks.is_empty(), "{name}");
            assert_eq!(
                plan.excluded,
                vec![NotCheckedReason::ExecutionNotAuthorized],
                "{name}"
            );
            assert_eq!(enforcement.admitted().count(), 0, "{name}");
            assert!(!enforcement.runs_project_code(), "{name}");
            let stopped = enforcement.stopped();
            assert_eq!(stopped.len(), 1, "{name}");
            assert_eq!(stopped[0].status, CheckStatus::Skipped, "{name}");
            assert_eq!(
                stopped[0].not_checked_reason,
                Some(NotCheckedReason::ExecutionNotAuthorized),
                "{name}"
            );
            assert_eq!(stopped[0].id, ids[0], "{name}");
            assert!(
                stopped[0].reason.contains("running your project's code"),
                "{name}: the reason is the vocabulary's own sentence: {}",
                stopped[0].reason
            );
        }
    }

    #[test]
    fn even_a_mode_that_runs_nothing_admits_only_commands_that_run_no_project_code() {
        // The strongest form, and it is not the same claim as the one above: the
        // checks that survive an inspect-only mode with everything granted are the
        // ones `decide` itself says are allowed there — a read and a fetch — and
        // what has to be true of the survivors is that none of them runs code the
        // project controls. A `git fetch` is admitted in a mode that runs nothing,
        // which is the domain's own documented answer and not a hole here.
        let mut ever_ran_code = false;
        for mode in ExecutionMode::ALL {
            for (name, permissions) in permission_sets() {
                let (enforcement, _) = enforcement_of(*mode, permissions, COMMANDS);
                for command in enforcement.admitted() {
                    assert!(
                        !consent::runs_project_code(command.effects()) || mode.runs_project_code(),
                        "{} / {name}: {} was admitted and runs the project's code",
                        mode.as_str(),
                        command.display()
                    );
                    ever_ran_code |= consent::runs_project_code(command.effects());
                }
                if !mode.runs_project_code() {
                    assert!(
                        enforcement.check_plan().dynamic_checks.is_empty(),
                        "{} / {name}",
                        mode.as_str()
                    );
                    assert!(
                        !enforcement.runs_project_code(),
                        "{} / {name}",
                        mode.as_str()
                    );
                }
            }
        }
        // Without this the loop above is a loop over an iterator that might
        // always be empty, and a matrix that admits nothing anywhere satisfies
        // every assertion in it. The grant that must admit is a mode that runs
        // project code with every permission granted, and the fixtures are what
        // make that reachable.
        assert!(
            ever_ran_code,
            "no mode and no permission set admitted a command that runs project code, so this \
             matrix has never reached the rule it tests"
        );
    }

    #[test]
    fn a_check_with_no_commands_is_static_because_nothing_is_launched_for_it() {
        let plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            FingerprintId::generate(),
            ExecutionPermissions::inspect_only(),
        );
        let scheduled = check("the manifest parses");
        let enforcement = Enforcement::of("plan", plan, std::slice::from_ref(&scheduled));
        assert_eq!(
            enforcement.check_plan().static_checks,
            vec![scheduled.id().clone()]
        );
        assert!(enforcement.stopped().is_empty());
        assert!(enforcement.admitted().next().is_none());
        assert!(!enforcement.runs_project_code());
    }

    #[test]
    fn a_check_with_one_allowed_command_and_one_refused_one_is_stopped_rather_than_half_run() {
        let mut plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            FingerprintId::generate(),
            ExecutionPermissions::inspect_only(),
        );
        let scheduled = check("the suite passes");
        plan.add(scheduled.clone(), "git", ["status"]);
        plan.add(scheduled.clone(), "python", ["-m", "pytest"]);
        let enforcement = Enforcement::of("plan", plan, std::slice::from_ref(&scheduled));

        assert!(enforcement.check_plan().static_checks.is_empty());
        assert!(enforcement.check_plan().dynamic_checks.is_empty());
        assert_eq!(
            enforcement.check_plan().excluded,
            vec![NotCheckedReason::ExecutionNotAuthorized]
        );
        assert_eq!(enforcement.stopped().len(), 1, "one check, one result");
        assert_eq!(
            enforcement.admitted().count(),
            0,
            "the half that could have run does not: a result for half a check is a result for a \
             check that did not happen"
        );
    }

    #[test]
    fn a_missing_permission_stops_the_check_and_the_reason_names_that_permission() {
        let (stopped, _) = enforcement_of(
            ExecutionMode::HostConfirmed,
            ExecutionMode::HostConfirmed.baseline_permissions(),
            &[("cargo", &["add", "serde"])],
        );
        assert_eq!(
            stopped.check_plan().excluded,
            vec![NotCheckedReason::DependencyInstallNotPermitted]
        );
        assert_eq!(stopped.admitted().count(), 0);

        // The same plan, granted: the check becomes a dynamic one rather than
        // disappearing, which is what makes the reason above a statement about
        // the permission rather than about the check.
        let (granted, _) = enforcement_of(
            ExecutionMode::HostConfirmed,
            everything(),
            &[("cargo", &["add", "serde"])],
        );
        assert!(granted.check_plan().excluded.is_empty());
        assert_eq!(granted.check_plan().dynamic_checks.len(), 1);
        assert_eq!(granted.admitted().count(), 1);
        assert!(granted.runs_project_code());
    }

    #[test]
    fn a_command_no_permission_covers_is_stopped_in_every_mode_and_never_admitted() {
        // `Destructive` has no permission, so no set of grants reaches it. Under
        // the modes that run project code the domain answers `NeedsConsent` —
        // there is a question worth asking — and **this module does not ask it**,
        // so the reason is the vocabulary's "the mode did not allow this" rather
        // than a declined prompt. A stop is the cautious direction, and reporting
        // it as a check that might still run would put a check in a plan that
        // nothing will ever run.
        for mode in ExecutionMode::ALL {
            for (name, permissions) in permission_sets() {
                let (enforcement, _) = enforcement_of(
                    *mode,
                    permissions,
                    &[("git", &["push", "--force", "origin", "main"])],
                );
                assert_eq!(
                    enforcement.admitted().count(),
                    0,
                    "{} / {name}: a command no permission covers was admitted",
                    mode.as_str()
                );
                assert_eq!(enforcement.stopped().len(), 1, "{} / {name}", mode.as_str());
            }
            let (granted, _) = enforcement_of(
                *mode,
                everything(),
                &[("git", &["push", "--force", "origin", "main"])],
            );
            assert_eq!(
                granted.check_plan().excluded,
                vec![NotCheckedReason::ExecutionNotAuthorized],
                "{}: every permission granted, and it is still not this module's to grant",
                mode.as_str()
            );
        }
    }

    #[test]
    fn the_stopped_results_and_the_excluded_reasons_are_in_step() {
        // `CheckPlan::excluded` is a list of reasons with no ids, so the only
        // thing tying a reason to a check is that the two lists were built in the
        // same pass. This is that, over every mode and every permission set.
        for mode in ExecutionMode::ALL {
            for (name, permissions) in permission_sets() {
                let (enforcement, _) = enforcement_of(*mode, permissions, COMMANDS);
                let plan = enforcement.check_plan();
                assert_eq!(
                    enforcement.stopped().len(),
                    plan.excluded.len(),
                    "{} / {name}",
                    mode.as_str()
                );
                for (result, reason) in enforcement.stopped().iter().zip(&plan.excluded) {
                    assert_eq!(result.not_checked_reason, Some(*reason), "{}", result.title);
                    assert_eq!(result.status, CheckStatus::Skipped, "{}", result.title);
                }
            }
        }
    }

    #[test]
    fn a_command_planned_for_a_check_that_was_not_scheduled_is_reported_and_never_admitted() {
        let mut plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            FingerprintId::generate(),
            ExecutionPermissions::inspect_only(),
        );
        let scheduled = check("the scheduled one");
        let forgotten = check("the forgotten one");
        plan.add(scheduled.clone(), "git", ["status"]);
        plan.add(forgotten.clone(), "git", ["status"]);
        let enforcement = Enforcement::of("plan", plan, std::slice::from_ref(&scheduled));

        assert_eq!(enforcement.unscheduled(), &[forgotten.id().clone()]);
        assert_eq!(
            enforcement.admitted().count(),
            1,
            "the forgotten check's command is not admitted, even though it would have been allowed"
        );
        assert!(
            enforcement
                .explain()
                .iter()
                .any(|line| line.contains("not in this plan's schedule")),
            "{:?}",
            enforcement.explain()
        );
    }

    #[test]
    fn a_check_scheduled_twice_is_scheduled_once() {
        let mut plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            FingerprintId::generate(),
            ExecutionPermissions::inspect_only(),
        );
        let scheduled = check("the only one");
        plan.add(scheduled.clone(), "git", ["status"]);
        let twice = vec![scheduled.clone(), scheduled.clone()];
        let enforcement = Enforcement::of("plan", plan, &twice);

        assert_eq!(enforcement.check_plan().all_checks().len(), 1);
        assert_eq!(enforcement.stopped().len(), 0);
        assert_eq!(enforcement.admitted().count(), 1);
    }

    #[test]
    fn a_refused_command_always_has_a_reason_and_an_allowed_one_never_does() {
        // The dependency the classification rests on: this module reads a check's
        // refusal out of `reason()`, and `find_map` over it is exhaustive only
        // because the two agree. A command in the fourth state — refused with no
        // reason — would be admitted by the `None` arm, which is the direction
        // that must not happen.
        for mode in ExecutionMode::ALL {
            for (name, permissions) in permission_sets() {
                let (enforcement, _) = enforcement_of(*mode, permissions, COMMANDS);
                for command in enforcement.permission_plan().commands() {
                    assert_eq!(
                        command.is_allowed(),
                        command.reason().is_none(),
                        "{} / {name}: {} is allowed={} but reason={:?}",
                        mode.as_str(),
                        command.display(),
                        command.is_allowed(),
                        command.reason()
                    );
                }
            }
        }
    }

    #[test]
    fn a_network_command_is_static_because_static_here_means_no_project_code_runs() {
        // Not an oversight. `CheckPlan::static_checks` is documented as "checks
        // that do not execute any project code", and a fetch does not — so a
        // check whose only command fetches is a static check by the field's own
        // definition, whatever else it does. Reading `static` as "harmless" is
        // the mistake this test exists to make impossible.
        let (enforcement, _) = enforcement_of(
            ExecutionMode::HostConfirmed,
            everything(),
            &[("git", &["fetch"])],
        );
        assert_eq!(enforcement.check_plan().static_checks.len(), 1);
        assert!(enforcement.check_plan().dynamic_checks.is_empty());
        assert_eq!(enforcement.admitted().count(), 1);
        assert!(!enforcement.runs_project_code());
    }
}
