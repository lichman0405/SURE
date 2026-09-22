//! `P3-T011`'s acceptance, checked rather than asserted.
//!
//! The task's two sentences are:
//!
//! > *Browser unavailable => skipped/unknown.*
//! > *Browser automation is isolated from core verdict semantics.*
//!
//! The first is behavioural and the module's own unit tests cover it over
//! `AbsenceReason::ALL`. **The second is an absence, and an absence cannot be
//! demonstrated by running anything** — which is the same problem
//! `spawn_sites.rs` and `fingerprint_git.rs` have, and this file answers it the
//! same way: the fact lives in the source text, so the source text is what is
//! checked.
//!
//! # The five rules
//!
//! **One: the frozen verdict machinery does not know what a browser is.**
//! `sure-domain/src/status.rs` owns [`CheckStatus`], `CriticalState`,
//! `blocks_green` and `aggregate`. It contains none of `browser`, `console`,
//! `page` or `driver`, in code or in prose, and it cannot start: a browser rule
//! appearing there is a browser rule in every verdict SURE produces.
//!
//! **Two: a driver cannot spell a verdict.** The [`BrowserDriver`] trait's own
//! body names neither `CheckStatus` nor `CheckResult`. This is the rule that
//! makes the seam real rather than conventional — a trait method returning a
//! `CheckStatus` would let an adapter decide what its own observations are
//! worth, and no amount of documentation would stop it.
//!
//! **Three: there is one door from a browser's answer to a verdict.**
//! `sure-core/src/browser.rs` contains exactly one `-> CheckResult` and exactly
//! one `-> CheckStatus`. A second one is a second opinion about the same
//! observation, and this is the count that makes adding one a decision.
//!
//! **Four: the page-status window is the local probe's window.** Swept over
//! every status from 100 to 599, the browser mapping and `ProbeOutcome`'s agree.
//! They are two spellings of one rule today rather than one function, so the
//! thing that keeps them together is this test; a change to either fails here
//! with the code that differs.
//!
//! **Five: nothing outside the adapter and the composition root can start a
//! browser** — and this rule has now moved twice, which is what it was written
//! to do. It used to read *no shipped file outside `browser.rs` names
//! [`BrowserDriver`]*, with a note saying that the day an adapter landed it must
//! fail and the commit that lands the adapter edits it. `P5-T004` landed one, it
//! failed, and it became two rules:
//!
//! * **Only the interface and the adapter may name [`BrowserDriver`].** A third
//!   file naming the trait is a file that has an opinion about how a browser is
//!   driven, and that opinion belongs in the adapter.
//! * **Nothing outside the adapter and the composition root may name the
//!   adapter.** This is the rule the old one became: an adapter that cannot be
//!   constructed from anywhere in the product is a browser that cannot be
//!   started from anywhere in the product, and it is checked against
//!   `browser_driver` rather than against the name of the type for the reason
//!   rule three gives — a `use` of a module has to spell the module, and
//!   `Browser` is a word this crate could grow elsewhere.
//!
//! **`P18-T010` made the second rule false as a name, and `P18-T012` is the task
//! that noticed.** That task bound the product's one driver construction at
//! `sure-cli/src/check.rs` — the composition root, the layer allowed to know
//! which implementations exist — and `sure-cli` is a crate the walk below could
//! not see: it started at `crates/sure-core` and filtered to `src`, so the rule
//! passed while the thing it forbids was shipping one crate away. **A check that
//! passes by not looking is the failure shape this repository treats as worse
//! than a visible error.** The walk covers every shipped crate now, matching the
//! scope `spawn_sites.rs` has always used, and the composition root is exempted
//! *by name* rather than by the walk being short — which is a decision a reader
//! can see and argue with, unlike a directory the walk never enters.
//!
//! Both are backed by the behavioural half, which has not moved: the product's
//! default execution mode denies the action a browser probe needs.
//!
//! # What is not claimed here
//!
//! **None of this makes a driver honest.** A driver that reports `complete:
//! true` and no problems about a page that threw has lied, and rules one to five
//! are silent about it. What they hold is the direction that matters — *no
//! driver can be handed, or hand itself, a verdict* — and the observations
//! themselves are checked by a second driver, not by this file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::browser::{BrowserDriver, Observation, Report, Target};
use sure_core::probe::ProbeOutcome;
use sure_core::scan::{ScanOptions, scan};
use sure_domain::execution::{
    ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions, decide,
};
use sure_domain::status::CheckStatus;

/// The file that owns the frozen verdict machinery.
const THE_VERDICT_MACHINERY: &str = "crates/sure-domain/src/status.rs";

/// The one file allowed to turn a browser's answer into a verdict, as a path
/// from the repository root.
const THE_DOOR: &str = "crates/sure-core/src/browser.rs";

/// The same file as [`shipped_sources`] reports it, which is relative to
/// `crates/` because that is the directory that walk starts from.
///
/// Two spellings of one path, because the two checks that name this file reach
/// it by different routes — and `the_two_spellings_of_the_browser_modules_path_agree`
/// is what stops them drifting into two files.
const THE_DOOR_IN_THE_WALK: &str = "sure-core/src/browser.rs";

/// The adapter, as the walk reports it: the directory, so that a new file inside
/// it is covered the day it is created rather than the day somebody remembers to
/// add it to a list.
const THE_ADAPTER_IN_THE_WALK: &str = "sure-core/src/browser_driver/";

/// The one file outside the adapter that may name it, which is the file that
/// declares the module.
///
/// **A declaration is not a caller**, and the distinction is the whole of why
/// this exemption is not a hole: `pub mod browser_driver;` makes the module
/// reachable and reaches nothing itself. Only a file that can name the module
/// can name a type inside it — which is what the rule below is stated against,
/// and what makes this exemption cost the rule nothing.
const THE_MODULE_DECLARATION: &str = "sure-core/src/lib.rs";

/// The second file outside the adapter that may name it, and the one `P18-T010`
/// put the product's only driver construction in.
///
/// `sure-cli/src/check.rs` is `sure check`'s composition root: the layer that
/// assembles `Paths`, the store, the pipeline and the runner, and the one layer
/// allowed to know **which implementations exist**. Binding
/// `browser_driver::Browser::system()` there is what makes a browser check
/// startable at all; `sure-core` cannot do it, because naming an implementation
/// is the composition root's job and not the library's.
///
/// **This exemption is a hole, and it is named as one.** A second construction
/// added to this same file would not be seen by the rule below. What makes that
/// tolerable is that the file is one function performing one assembly, and that
/// `sure_core::ProcessRunner` answers a browser check it holds no driver for
/// with an `Error` — so a *second* construction here cannot make a run greener,
/// only a run that could not have driven a page either way. What is not
/// tolerable, and what `P18-T012` repaired, is the walk stopping one crate short
/// so that this file was never read at all.
const THE_COMPOSITION_ROOT: &str = "sure-cli/src/check.rs";

/// The files that may name [`BrowserDriver`]: the interface that defines it and
/// the adapter that implements it.
///
/// Two entries rather than a predicate, for the reason `spawn_sites.rs` gives
/// about its own lists: a predicate is a rule that grows without anybody reading
/// it, and a third entry is a decision somebody has to write down.
const MAY_NAME_THE_INTERFACE: &[&str] = &[THE_DOOR_IN_THE_WALK, THE_ADAPTER_IN_THE_WALK];

/// Words that mean a browser rule has reached the core verdict machinery.
///
/// Four words rather than a list of type names, because the leak this guards
/// against is not an import — it is a paragraph, a comment and a special case.
/// A core verdict file that has begun talking about pages has stopped being a
/// core verdict file whatever it imports.
const BROWSER_WORDS: &[&str] = &["browser", "console", "page", "driver"];

/// Read a file the rules are stated against, refusing to check a file that
/// could not be read.
///
/// A rule that passes because its subject was not found is the false green this
/// repository is built against, so a missing file is a failure and not a skip.
fn read(relative: &str) -> String {
    let path = sure_testkit::repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// Whether a line is prose rather than code.
///
/// The same helper, and the same reason, as `spawn_sites.rs`': the paragraphs
/// that *explain* a rule have to be able to name the thing the rule is about.
fn is_prose(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Every shipped `.rs` file under `crates/`, with its text.
///
/// # Why the walk covers every crate and not only `sure-core`
///
/// It covered `crates/sure-core` alone until `P18-T012`, and that was a hole
/// rather than a scope: `P18-T010` put the product's one driver construction at
/// `sure-cli/src/check.rs`, and a walk that never enters `sure-cli` cannot see
/// it — so rule five passed while the thing it forbids was shipping. **A rule
/// that passes by not looking is the false green this repository is built
/// against, and it was found by a task that was not looking for it.**
///
/// The scope is now the one `spawn_sites.rs` has always used — every crate under
/// `crates/` — and the filter is that file's too, for its reason: a test file is
/// supposed to name these types, and a rule that counted them would be a rule
/// nobody could satisfy. A path with no `src` in it is a test, a benchmark or a
/// build script, and none of those ships.
///
/// The cost of the wider scope is that `sure-testkit` is walked, and that crate
/// is deliberately not part of the product — `repository_shape.rs`'s
/// `nothing_depends_on_the_testkit_in_production` keeps it out. Including it is
/// stricter than the rule needs and is recorded rather than filtered out: a rule
/// whose exemption list grows a crate name for a reason that is about shipping
/// rather than about browsers would be a second door, and this file already has
/// one. It names none of these tokens today.
fn shipped_sources() -> Vec<(String, String)> {
    let root = sure_testkit::repository_root().join("crates");
    let walked = scan(&root, ScanOptions::default()).expect("crates/ is a directory");
    assert!(
        walked.is_complete(),
        "the source tree could not be read completely, so these rules would be \
         checking an unknown subset of it"
    );

    let shipped: Vec<(String, String)> = walked
        .files()
        .filter(|entry| entry.path.extension().is_some_and(|ext| ext == "rs"))
        // Filtered on the reported path rather than the filesystem one, so that
        // the string the failure messages print and the string the rules are
        // stated against are the same string. `src` alone, without a separator,
        // because [`scan::display_path`] writes `/` on every platform and the
        // first component is a crate name that could contain the word.
        .filter(|entry| entry.display_path().contains("src"))
        .map(|entry| {
            let path = root.join(&entry.path);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (entry.display_path(), text)
        })
        .collect();

    // The vacuity guard, and it is the load-bearing one for the wider walk: a
    // predicate that matched nothing would make every rule below pass over an
    // empty corpus. The number is the one the widening is *for* — a walk that
    // reached only `sure-core` would not reach the composition root — so the
    // check is stated as both halves rather than as a count.
    assert!(
        shipped.len() > 10,
        "the source walk found {} shipped files, which is not this workspace — the \
         filter is matching the wrong thing",
        shipped.len()
    );
    for crate_in_the_product in ["sure-cli/src/", "sure-core/src/"] {
        assert!(
            shipped
                .iter()
                .any(|(path, _)| path.contains(crate_in_the_product)),
            "the walk reaches no file under `{crate_in_the_product}`, so the rules \
             stated over it are stated over a subset of the product: {:?}",
            shipped.iter().map(|(path, _)| path).collect::<Vec<_>>()
        );
    }
    shipped
}

/// The text of `pub trait BrowserDriver { ... }`, to its closing brace.
///
/// Returns `None` rather than an empty string when the trait is not found. **An
/// empty block would satisfy every rule stated against it**, and a parse that
/// fails into a smaller, plausible answer is the defect this repository has hit
/// more than once; the test below refuses the `None` instead of treating it as
/// a block with nothing in it.
fn driver_trait_block(text: &str) -> Option<&str> {
    const OPENS: &str = "pub trait BrowserDriver {";
    let start = text.find(OPENS)?;
    let body = &text[start + OPENS.len()..];
    let end = body.find("\n}")?;
    Some(&body[..end])
}

/// The part of a file above its `#[cfg(test)]` module.
///
/// The door rule is about code that ships, and `browser.rs`'s own tests declare
/// a helper that returns a [`CheckResult`] because that is what testing a
/// verdict mapping looks like. Counting it would make the rule unsatisfiable and
/// the rule that *is* satisfiable — "no test may build a verdict" — is false and
/// should be: a test that could not build one could not check the mapping.
///
/// `None` when there is no test module, which the caller refuses rather than
/// treating as "the whole file ships". The difference matters: with no test
/// module the counts below would silently include anything the file grew later.
fn shipped_part(text: &str) -> Option<&str> {
    text.split_once("\n#[cfg(test)]")
        .map(|(shipped, _)| shipped)
}

#[test]
fn the_frozen_verdict_machinery_does_not_know_what_a_browser_is() {
    let text = read(THE_VERDICT_MACHINERY);
    assert!(
        text.contains("pub fn aggregate"),
        "the file the browser vocabulary is forbidden from is not the verdict \
         machinery any more — it does not define `aggregate`"
    );

    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let lowered = line.to_lowercase();
        for word in BROWSER_WORDS {
            if lowered.contains(word) {
                found.push(format!(
                    "{THE_VERDICT_MACHINERY}:{}: {}",
                    number + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        found.is_empty(),
        "the frozen verdict machinery has begun talking about browsers. That is \
         not a test to widen: a browser rule in this file is a browser rule in \
         every verdict SURE produces, and the acceptance for the browser probe \
         is that it is isolated from these semantics. Found:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn a_component_that_drives_a_browser_cannot_hand_sure_a_verdict() {
    let text = read(THE_DOOR);
    let block = driver_trait_block(&text).expect(
        "`pub trait BrowserDriver {` was not found in the browser module, so this \
         rule checked nothing",
    );

    // The vacuity guard, before the rule itself: a block that was found but is
    // empty would pass the rule below while asserting nothing at all.
    assert!(
        block.contains("fn observe"),
        "the trait block does not contain the one method a driver implements, so \
         the parse found the wrong region: {block:?}"
    );

    for forbidden in ["CheckStatus", "CheckResult"] {
        assert!(
            !block.contains(forbidden),
            "the driver interface names `{forbidden}`, so an adapter can decide \
             what its own observations are worth. The seam is that a driver \
             reports what the page did and SURE decides what that is worth; a \
             signature naming a verdict is the seam gone. Block:\n{block}"
        );
    }
}

#[test]
fn there_is_one_door_from_a_browser_to_a_verdict() {
    let text = read(THE_DOOR);

    // The vacuity guard for the counts below: a file that could not be found or
    // is not the browser module would give zero of both and pass a `<= 1` rule.
    assert!(
        text.contains("pub trait BrowserDriver") && text.contains("pub enum Report"),
        "the file at {THE_DOOR} is not the browser module, so the counts below \
         would be counting nothing"
    );

    let shipped = shipped_part(&text).unwrap_or_else(|| {
        panic!(
            "{THE_DOOR} has no `#[cfg(test)]` module, so this rule cannot tell \
             shipped code from test code and the counts below would include \
             whatever the file grows later"
        )
    });
    assert!(
        shipped.contains("pub trait BrowserDriver"),
        "the text above the test module is not the shipped module — the split \
         found the wrong boundary"
    );

    for (signature, expected, why) in [
        (
            "-> CheckResult",
            1,
            "a second one is a second opinion about the same observation",
        ),
        (
            "-> CheckStatus",
            1,
            "the mapping from an observation to a status is one function, and a \
             second one is a rule that can disagree with the first",
        ),
    ] {
        let found = shipped.matches(signature).count();
        assert_eq!(
            found, expected,
            "the shipped part of {THE_DOOR} contains {found} of `{signature}` and \
             should contain {expected}: {why}"
        );
    }
}

#[test]
fn the_page_status_window_is_the_local_probes_window() {
    // Two spellings of one rule, so the thing that keeps them together is this
    // test rather than a shared function. The disagreement it is looking for is
    // the expensive kind: a page served a 404 renders, reports no errors and
    // looks exactly like a page served a 200 to everything except this number.
    for code in 100_u16..=599 {
        let probe = ProbeOutcome::Answered {
            status: code,
            status_line: format!("HTTP/1.1 {code}"),
            body_bytes: 0,
            truncated: false,
        };
        let browser = Report::observed(Observation {
            landing_url: "http://127.0.0.1:3000/".to_owned(),
            title: "a page".to_owned(),
            document_status: Some(code),
            problems: Vec::new(),
            complete: true,
        });

        assert_eq!(
            browser.status(),
            probe.status(),
            "HTTP {code} is worth {:?} to the local probe and {:?} to the browser \
             probe",
            probe.status(),
            browser.status()
        );
        assert_eq!(
            browser.status().is_green(),
            (200..400).contains(&code),
            "HTTP {code} is green for the wrong half of the range"
        );
    }
}

/// Every line of **code** in a shipped file that names `token`, except in
/// `exempt`, as `path:line: text`.
///
/// The same shape as `spawn_sites.rs`'s `namers_of`, and `is_prose` is the same
/// filter for the same reason: the paragraphs that explain a rule have to be
/// able to name the thing the rule is about, and this file's own subject is
/// named in dozens of them.
fn namers_of(token: &str, exempt: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in shipped_sources() {
        if exempt.iter().any(|allowed| path.starts_with(allowed)) {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if is_prose(line) {
                continue;
            }
            if line.contains(token) {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    found
}

#[test]
fn only_the_interface_and_the_adapter_know_how_a_browser_is_driven() {
    // The first half of rule five, as it stands after the adapter landed. Before
    // `P5-T004` this was "no shipped file outside `browser.rs` names the trait",
    // and the adapter is the file that was always going to break it.
    let found = namers_of("BrowserDriver", MAY_NAME_THE_INTERFACE);

    assert!(
        found.is_empty(),
        "a shipped file outside the interface and its adapter has begun naming \
         `BrowserDriver`, so it has an opinion about how a browser is driven. \
         That opinion belongs in the adapter: add the file to \
         MAY_NAME_THE_INTERFACE and say in the paragraph above what it does \
         there. Found:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn nothing_outside_the_adapter_can_start_a_browser() {
    // The second half, and the one that carries the claim the first half used to:
    // a `Browser` that nothing constructs is a browser that nothing starts.
    //
    // Stated against the module rather than against the type name, because a
    // file that names a type inside a module has to spell the module to reach it
    // — and because `Browser` is an ordinary English word that a crate this size
    // could grow in an unrelated place, which would make the rule fail for a
    // reason that is not about browsers at all.
    let found = namers_of(
        "browser_driver",
        &[
            THE_ADAPTER_IN_THE_WALK,
            THE_MODULE_DECLARATION,
            THE_COMPOSITION_ROOT,
        ],
    );

    assert!(
        found.is_empty(),
        "a shipped file outside the adapter and the composition root has begun \
         naming it, so something in the product can construct a driver and start \
         a browser. That is a real change and not a test to update: it means a \
         path out of the product opens a page.\n\n\
         What it does **not** mean is that `sure_core::support`'s ceiling of \
         *inspect only* has to move, and this message used to say so. The \
         ceiling's reason is written beside the ceiling and was measured by \
         `P18-T012`: it is about which platforms SURE has been shown to run \
         project code on, not about whether a caller exists — one has, since \
         `P18-T007`. Neither this file nor a new name in it is evidence about \
         that, and `docs/adr/0015` is where the measurement is. So: take the \
         name back out, or add the file to the list above with a paragraph \
         saying what it does there and read the ADR before touching the \
         ceiling. Found:\n  {}",
        found.join("\n  ")
    );

    // The exemption for the module declaration is only worth having if the
    // declaration is still there — an entry that exempts a file nobody needs
    // exempted reads to the next person as a decision somebody made.
    let declared = shipped_sources()
        .into_iter()
        .filter(|(path, _)| path == THE_MODULE_DECLARATION)
        .any(|(_, text)| {
            text.lines()
                .any(|line| !is_prose(line) && line.contains("browser_driver"))
        });
    assert!(
        declared,
        "{THE_MODULE_DECLARATION} is exempted from the rule about `browser_driver` \
         and no longer declares the module, so the exemption is covering nothing"
    );
}

#[test]
fn the_default_mode_denies_the_permission_a_browser_probe_needs() {
    // Named for what it checks rather than for what it used to be able to say.
    // It was `nothing_in_the_product_can_drive_a_browser_today` until
    // `P18-T012`, and that name is false since `P18-T010`: a product path does
    // construct a driver. What is true, and what this asserts, is that the mode
    // the product documents as its default grants no `ConnectService`, so that
    // path is never reached without a permission the user gave.
    // Rule five's behavioural half: the mode the product documents as its
    // default grants no `ConnectService`, so a browser probe is denied before
    // any question about a browser is reached.
    assert_eq!(
        decide(
            ActionKind::BrowserProbe,
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only()
        ),
        ExecutionDecision::Denied
    );

    // And a caller that has not asked for the permission explicitly gets
    // `skipped` rather than a green, which is the task's first sentence.
    let absence = sure_core::browser::absence(
        sure_core::config::CheckPreference::Auto,
        ExecutionMode::InspectOnly,
        &ExecutionPermissions::inspect_only(),
    )
    .expect("a browser probe without the connect permission is an absence");
    let verdict = Report::absent(absence.reason, absence.detail).verdict(
        sure_domain::ids::CheckId::generate(),
        &Target::local(3000, "/").expect("a loopback target"),
        sure_domain::severity::Severity::MustFix,
        true,
        sure_domain::ids::FingerprintId::generate(),
    );
    assert_eq!(verdict.status, CheckStatus::Skipped);
    assert!(verdict.blocks_green());
}

#[test]
fn the_two_spellings_of_the_browser_modules_path_agree() {
    // Written because they disagreed once: `src/browser.rs` used as a path from
    // the repository root read nothing, and the rule built on it would have
    // checked an empty file rather than failing. The read refuses a missing file
    // now; this is the cheaper check that catches the mistake before it becomes
    // an exemption that covers nothing.
    assert!(
        THE_DOOR.ends_with(THE_DOOR_IN_THE_WALK),
        "{THE_DOOR} does not end with {THE_DOOR_IN_THE_WALK}, so the walk's \
         spelling of the browser module names a different file — or none"
    );
    assert!(
        THE_DOOR_IN_THE_WALK.starts_with("sure-core/src/"),
        "{THE_DOOR_IN_THE_WALK} is not a path the walk can report, so the entry \
         it is used as exempts a file that is not in the corpus"
    );
}

/// A driver written **outside the crate**, which is the only way to prove the
/// interface is usable at all.
///
/// The module's own unit tests reach a stub through `super::`, which works even
/// for a type that cannot be named from another crate. This one is in an
/// integration test, so it can only compile if `BrowserDriver`, `Target`,
/// `Limits` and `Report` are all public, and only run if the trait is
/// object-safe — a caller holding one from configuration has a
/// `Box<dyn BrowserDriver>` and not a concrete type.
struct Outside(Report);

impl BrowserDriver for Outside {
    fn observe(
        &self,
        _target: &Target,
        _limits: &sure_core::browser::Limits,
        _cancellation: &sure_core::process::Cancellation,
    ) -> Report {
        self.0.clone()
    }
}

#[test]
fn the_interface_is_usable_from_outside_the_crate_and_through_a_boxed_driver() {
    let driver: Box<dyn BrowserDriver> = Box::new(Outside(Report::observed(Observation {
        landing_url: "http://127.0.0.1:3000/".to_owned(),
        title: "a page".to_owned(),
        document_status: Some(200),
        problems: Vec::new(),
        complete: true,
    })));

    let target = Target::local(3000, "/").expect("a loopback target");
    let limits = sure_core::browser::Limits::new(std::time::Duration::from_secs(5), 10)
        .expect("five seconds is a budget");
    let report = driver.observe(
        &target,
        &limits,
        &sure_core::process::Cancellation::default(),
    );
    assert_eq!(report.status(), CheckStatus::Pass);

    // A caller that does not want to build a budget by hand still cannot build
    // one that means nothing.
    assert!(sure_core::browser::Limits::new(std::time::Duration::ZERO, 10).is_err());
}
