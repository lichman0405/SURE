//! The Windows signing and SmartScreen statement, and what it is allowed to say.
//!
//! `P15-T012`'s acceptance is two sentences, and neither of them says "write a
//! paragraph":
//!
//! > *If Authenticode/code-signing credentials are unavailable, mark external
//! > limitation honestly.*
//! > *Do not fake signing or claim SmartScreen reputation.*
//!
//! The first asks for an honest **statement of an external limitation**; the
//! second is a **prohibition on two specific lies** — claiming a signature that
//! does not exist, and claiming a reputation the binary does not have.
//!
//! # What was wrong, and where the guard belongs
//!
//! Five places in this repository said the Windows build is unsigned and **not
//! one of them read the signature**. `scripts/Build-Release.ps1` wrote
//! `THIS BUILD IS UNSIGNED` into the archive's own `RELEASE.txt` while never
//! looking at the file it was talking about. The other two platforms were
//! already held to a different standard: `scripts/Build-Release.sh` runs
//! `codesign -d` on the macOS artifact and prints what it finds, and its Linux
//! branch reports `not read` **and gives the reason**. Windows asserted; the
//! others measured or explained. `grep -rn 'UNSIGNED|unsigned'
//! crates/*/tests/*.rs` returned nothing before this file, so every one of
//! those sentences could have been deleted and every gate would have stayed
//! green.
//!
//! The measurement itself is not here. It is in `scripts/Build-Release.ps1`,
//! which asks Windows its own question — `Get-AuthenticodeSignature` — about the
//! `sure.exe` it is about to zip, takes a second reading of the extracted binary
//! in both phases, and writes the answer into `RELEASE.txt` as a `signature`
//! line. That the reading is *taken* is a property of a run, and this file
//! cannot see a run.
//!
//! # What this file proves, and what it cannot
//!
//! It reads text and asserts two kinds of thing. It proves that the **mechanism
//! is still written down** — that the packager still names the cmdlet, still has
//! an arm for the answer "there is none" and an arm for "the reader did not
//! answer", and still selects the archive's paragraph from the reading rather
//! than writing it flat; and that the installer still reads the archive's own
//! `signature` line rather than making a claim of its own. And it proves that
//! **four families of forbidden phrasing are absent** from the files a reader
//! meets the claim in.
//!
//! It cannot prove any of the following, and no test here should be read as
//! proving them:
//!
//! * that a run took the reading, or that the branch it took was the one the
//!   reading called for. A presence rule is a rule about text. What says the
//!   branch was taken is `P15-T012`'s own run record, which is a log rather than
//!   a test;
//! * that anyone will ever **not** see a SmartScreen prompt. Nothing in this
//!   repository has observed the prompt at all — no test runs an unsigned binary
//!   through a download-marked file, and `docs/development/INSTALL_WINDOWS.md`'s
//!   `## What is not covered` says so in the same words. A rule over a phrase
//!   list cannot enumerate the phrasing nobody thought of;
//! * that the list below is **complete**. A list of strings is a list of the
//!   ways someone thought of, and this file's own `every_forbidden_phrase_is_one
//!   _this_reader_reports` is what keeps a dead entry from sitting in it looking
//!   like a check. The same limit, and the same remedy, as `FORBIDDEN_ACTS` in
//!   `crates/sure-testkit/tests/ci_workflow.rs`.
//!
//! # Why the phrase rules do not scan all five files
//!
//! Two of the five are exempt, each for a reason that is a *finding* rather than
//! a convenience, and `the_two_exemptions_are_still_earning_their_place` fails if
//! either stops being true — so an exemption cannot quietly become permanent.
//!
//! * `docs/development/RELEASE_PROCESS.md` is the authoritative statement and
//!   therefore the file that **lists these phrasings in order to forbid them**.
//!   A substring rule cannot tell naming a lie from telling one: deleting this
//!   exemption would make the prohibition list illegal to write, which is the
//!   opposite of what the acceptance asks for. What guards it instead is a
//!   presence rule — it has to keep carrying the prohibition section and the
//!   boundary sentence.
//! * `.github/workflows/release.yml` carries `Windows will warn about an unknown
//!   publisher` in its release notes, which is a definite claim about a Microsoft
//!   service in the safe direction. **That file is `P15-T011`'s, accepted one
//!   task ago with measurements taken on real runners, and this task may not edit
//!   it.** It is reported in the hand-back rather than asserted here, and the
//!   exemption test fails the moment a later task fixes the sentence, so whoever
//!   fixes it is told to delete the exemption rather than leaving it to rot.
//!
//! # Why the reader is a line reader, and why it reports what it found
//!
//! `docs/development/RELEASE_PROCESS.md` is Markdown, the two scripts are
//! PowerShell and the workflow is YAML, so there is no one parse of all five and
//! none of these properties is about structure. What keeps a text reader honest
//! is that it says what it found: `the_reader_sees_the_mechanism_that_is_there`
//! fails if the anchors were read as absent, and `a_file_that_is_not_there_is_
//! not_a_pass` feeds the rules nothing and requires every one of them to
//! complain.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// The file that asks Windows the question and writes the answer into the
/// archive's `RELEASE.txt`.
const PACKAGER: &str = "scripts/Build-Release.ps1";

/// The file that puts that answer in front of the person who installed it.
const INSTALLER: &str = "scripts/Install-Sure.ps1";

/// The authoritative statement of the limitation.
const RELEASE_PROCESS: &str = "docs/development/RELEASE_PROCESS.md";

/// The statement a person installing meets in the documentation.
const INSTALL_WINDOWS: &str = "docs/development/INSTALL_WINDOWS.md";

/// The release notes — `P15-T011`'s file, which this task may not edit.
const RELEASE_WORKFLOW: &str = ".github/workflows/release.yml";

/// Every file that states the Windows signing limitation.
const STATEMENTS: [&str; 5] = [
    PACKAGER,
    INSTALLER,
    RELEASE_PROCESS,
    INSTALL_WINDOWS,
    RELEASE_WORKFLOW,
];

/// The three files a phrase rule can read without reading a quotation.
///
/// See "Why the phrase rules do not scan all five files" in this file's header.
const SCANNED_FOR_PHRASING: [&str; 3] = [PACKAGER, INSTALLER, INSTALL_WINDOWS];

/// What the packager has to keep saying for the claim to be measured.
///
/// Each entry is a literal substring of `scripts/Build-Release.ps1` as it
/// stands, and the sentence is what its disappearance would mean. These are
/// presence rules and the header says what a presence rule cannot prove.
///
/// Three rounds of `every_rule_is_turned_red_by_an_edit_that_breaks_it` shaped
/// this list. The first version anchored on the bare names — the cmdlet, the
/// status, the state — and the mutation table showed each one surviving the
/// deletion of the code it named, because the same words appear in the file's
/// comments. An anchor that a comment can satisfy is a check on the comment.
/// The third found the same weakness from the other side: every entry deleted
/// code, so none of them held the **wiring**, and renaming the `'not-signed'`
/// case label — which leaves every arm present and one of them unreachable, and
/// therefore makes every archive this repository builds claim its signature
/// state was never established — was green. Each entry below is therefore the
/// **code that does the thing**, which is a string no amount of prose can
/// carry, and the entry that holds the switch is the adjacency between the
/// switch and the arm it selects, because an anchor for either alone survives
/// the other one moving.
const PACKAGER_REQUIRED: &[(&str, &str)] = &[
    (
        "$signature = Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop",
        "the question Windows answers about a file's Authenticode signature is not asked anywhere \
         in the file, so the sentence in the archive's RELEASE.txt is asserted again rather than \
         read",
    ),
    (
        "$reading.Status -eq 'NotSigned'",
        "the reading has no arm for the answer that there is no signature, so 'there is none' and \
         'the reader did not answer' cannot be told apart",
    ),
    (
        "State = 'cannot-confirm'",
        "the third state is gone, so a reading that did not happen would have to be reported as \
         either a signature or an absence — and a reader that failed is not an observation of \
         absence",
    ),
    (
        "Microsoft.PowerShell.Security, which does not load in every host",
        "the module that carries the cmdlet is not named in the archive's own text, so why a \
         reading can fail to be taken is not written down where the person holding the file can \
         read it",
    ),
    (
        "switch ($exeSignature.State) { 'not-signed' {",
        "either the archive's own text is not selected by the reading at all, so it is a constant \
         written beside a reading rather than a function of one, or the switch lost the arm a \
         reading that found nothing takes. The anchor is the adjacency between the switch and its \
         first case on purpose: `switch ($exeSignature.State)` alone survives that case being \
         renamed to a word no reading produces, which leaves the paragraph an unsigned build is \
         supposed to carry unreachable and makes every archive this repository builds say its \
         state was never established. Both halves are compared squashed, like every other anchor",
    ),
    (
        "$signatureToken = $exeSignature.State",
        "the RELEASE.txt header block does not carry the state in one word, so nothing downstream \
         — including scripts/Install-Sure.ps1 — can read it",
    ),
    (
        "RELEASE_PROCESS.md records what that means under",
        "the archive's own text does not point at the authoritative statement, so a reader of it \
         has no way to the reason, the boundary and the prohibition",
    ),
    (
        "Get-SignatureReading -Path $stagedExe",
        "the reading is not taken on the staged binary, so the state in the archive is whatever \
         the verify step happened to read later rather than a reading taken before the bytes were \
         sealed",
    ),
    (
        "Get-StatedSignature -Path $releaseTxtPath",
        "the verify step does not use the shared reader of the archive's own line, so what this \
         run prints about that file comes from other code, and the two are free to disagree",
    ),
];

/// The flat sentence this task removed, which must not come back.
///
/// It is checked as an *absence* rather than by a presence rule for its
/// replacement, because it is the defect itself: a claim of absence written
/// unconditionally is true on the day it is written and silently false from the
/// moment anybody signs a build.
const PACKAGER_FORBIDDEN: &[(&str, &str)] = &[(
    "THIS BUILD IS UNSIGNED",
    "the unconditional unsigned heading is back in scripts/Build-Release.ps1, which is the \
     unmeasured claim P15-T012 removed: the sentence has to be selected by the reading, not \
     written flat",
)];

/// What the installer has to keep doing, and the one thing it must not do.
const INSTALLER_REQUIRED: &[(&str, &str)] = &[
    (
        "Get-StatedSignature -Path (Join-Path $BinDirectory 'RELEASE.txt')",
        "the installer does not read the archive's own `signature` line, so whatever it prints \
         about signing is a sentence of its own rather than the state the machine that built the \
         archive measured",
    ),
    (
        "'cannot-confirm' {",
        "the installer has no arm for an archive whose state was not established, so it would have \
         to answer either way about bytes nobody read",
    ),
    (
        "project can measure; docs/development/RELEASE_PROCESS.md,",
        "the installer reports a state without naming where the reason, the boundary and the \
         prohibition are kept",
    ),
];

/// The installer must not become a second measurer of the same fact.
///
/// Read as an absence on purpose. The installer could ask Windows the same
/// question about the copy it just installed, and that would be a second reading
/// of one fact, on the machine least able to do anything with the answer, with
/// no rule for what to do when the two disagree. What it can honestly do is
/// carry the archive's own sentence to the reader, which `INSTALLER_REQUIRED`
/// makes it do.
const INSTALLER_FORBIDDEN: &[(&str, &str)] = &[
    (
        "Get-AuthenticodeSignature",
        "the installer has become a second measurer of a fact the archive already records, and the \
         two readings have no rule for disagreeing",
    ),
    (
        "this build has no Authenticode signature",
        "the installer asserts the state instead of reporting the one the archive carries, which \
         is the unmeasured sentence P15-T012 removed from the packager one step further down the \
         pipe — in front of the one person who cannot check it",
    ),
];

/// The one reader of `RELEASE.txt`'s `signature` line, which both scripts share.
///
/// It is not a shared module — there is none between these two scripts, and one
/// would be a new file for the sake of one function. It is the same function
/// written twice and held together by
/// `the_two_readers_of_release_txt_are_one_reader` below, because **two copies
/// of a reader are two readers** unless something reddens when they drift. That
/// is the specific thing this file must not allow: the archive's own line is
/// what everything downstream reads, and two scripts that disagreed about how to
/// read it would put two different sentences in front of the same person about
/// the same archive.
const READER_NAME: &str = "Get-StatedSignature";

/// The two scripts that read a `RELEASE.txt`.
const READERS: [&str; 2] = [PACKAGER, INSTALLER];

/// The two readers of `RELEASE.txt` are one reader.
///
/// **Do not create a second, divergent reader for a file an existing reader
/// already reads** is P15-T012's fourth constraint, and this is what makes it
/// more than a resolution. Both scripts read the `signature` line, neither can
/// import the other, and the honest arrangement is one function written twice
/// with a test that fails the moment the two texts differ by a character. Any
/// edit to one is then an edit the other has to make too, and a disagreement is
/// a red build rather than two different sentences in front of the same person.
///
/// Same shape as the rule `docs/development/INSTALL_WINDOWS.md` already
/// records for the known-folder lookup: "a test compares the two scripts' text
/// so that a fix to one is a fix to both."
#[test]
fn the_two_readers_of_release_txt_are_one_reader() {
    let bodies = READERS.map(|path| {
        let text = read(path);
        let body = function_body(&text, READER_NAME).unwrap_or_else(|| {
            panic!("{path} has no `function {READER_NAME} {{`, so it either stopped reading RELEASE.txt's signature line or reads it some other way")
        });
        (path, body)
    });
    let found = reader_divergence(&|wanted: &str| {
        READERS
            .iter()
            .find(|path| **path == wanted)
            .map(|path| read(path))
    });
    assert!(
        found.is_empty(),
        "{}\n\n--- {} ---\n{}\n\n--- {} ---\n{}",
        found.join("\n"),
        bodies[0].0,
        bodies[0].1,
        bodies[1].0,
        bodies[1].1
    );
    assert!(
        bodies[0].1.len() > 100,
        "the reader found only {} bytes of {READER_NAME} in {}, which is not a reader:\n{}",
        bodies[0].1.len(),
        bodies[0].0,
        bodies[0].1
    );
}

/// The text of `function <name> {` through its closing brace at column 0.
fn function_body(text: &str, name: &str) -> Option<String> {
    let opener = format!("function {name} {{");
    let start = text.lines().position(|line| line == opener)?;
    let mut body: Vec<&str> = Vec::new();
    for line in text.lines().skip(start) {
        body.push(line);
        if body.len() > 1 && line == "}" {
            break;
        }
    }
    Some(body.join("\n"))
}

/// What the authoritative statement has to keep carrying.
const AUTHORITATIVE_REQUIRED: &[(&str, &str)] = &[
    (
        "SmartScreen is not measurable here at all",
        "the boundary is not stated: what this project can establish is a fact about bytes, and \
         what Windows does with them afterwards belongs to a service this project cannot observe",
    ),
    (
        "not-signed",
        "the state the reading produces when it answers is not named",
    ),
    (
        "cannot-confirm",
        "the state that exists because a reader that failed is not an observation of absence is \
         not named",
    ),
    (
        "Do not fake signing",
        "the acceptance's own words are gone from the section the other four files point at",
    ),
    (
        "What must not be written",
        "the prohibition is not spelled out, and a prohibition left as a general principle is one \
         a reader has to interpret rather than one a file states",
    ),
];

/// What the install-time statement has to keep carrying.
const INSTALL_WINDOWS_REQUIRED: &[(&str, &str)] = &[
    (
        "## The build is unsigned, and how that is known",
        "the section a reader is sent to for this is gone, and with it the only place a person \
         installing the program is told the state was read rather than assumed",
    ),
    (
        "`RELEASE_PROCESS.md`'s `## Signing` is where the reason, the boundary and the",
        "the statement does not point at the authoritative section",
    ),
    (
        "prompt is **not measurable from here**",
        "the boundary is not restated where a person installing meets the claim, so the document \
         says the build is unsigned and leaves what that does to them to be guessed. The emphasis \
         is part of the anchor: this sentence is the one the file makes impossible to read past, \
         and a rule that held only `not measurable from here` would be satisfied by the milder \
         mention in `## What is not covered` further down",
    ),
];

/// Phrasings that would claim a SmartScreen reputation this project cannot see.
///
/// SmartScreen is a Microsoft service driven by download telemetry this project
/// does not hold and cannot observe. Every string here asserts something about
/// what that service does or will do with this binary, in the reassuring
/// direction — the direction the second acceptance clause is about.
const REPUTATION_CLAIMS: &[(&str, &str)] = &[
    (
        "reputation builds",
        "a promise that SmartScreen reputation accrues to this binary, which is a claim about \
         somebody else's service",
    ),
    ("reputation accrues", "the same promise in different words"),
    ("reputation will", "the same promise in the future tense"),
    (
        "will not see a warning",
        "an assertion that a reader meets no prompt, which nothing in this repository can observe",
    ),
    ("will not be warned", "the same assertion in the passive"),
    ("no warning will", "the same assertion, negated and fronted"),
    (
        "avoids the SmartScreen",
        "a claim that this build escapes SmartScreen, which is a property of the downloading \
         machine and of a Microsoft service rather than of these bytes",
    ),
    (
        "bypass SmartScreen",
        "the same claim written as an instruction",
    ),
    (
        "SmartScreen will trust",
        "a claim about what the reputation service does with this binary",
    ),
];

/// Phrasings that present signing as done, free, or instant.
const SIGNING_CLAIMS: &[(&str, &str)] = &[
    (
        "signing is free",
        "a statement about what a code-signing credential costs, which this project does not hold \
         and has not priced",
    ),
    ("signing is instant", "the same statement about time"),
    ("signing is quick", "the same statement about time"),
    (
        "certificate is configured",
        "a claim that a signing credential exists",
    ),
    ("we hold a certificate", "the same claim"),
    ("signed by SURE", "a claim of a signature by this project"),
    ("this build is signed", "the same claim about the artifact"),
];

/// Phrasings that present the absence of a signature as a neutral fact.
///
/// The acceptance asks for a limitation to be *marked*. A sentence that presents
/// an unsigned binary as a detail with no consequence leaves a reader surprised
/// by a prompt they were never told to expect.
const NEUTRAL_ABSENCE_CLAIMS: &[(&str, &str)] = &[
    (
        "no signature is required",
        "the absence of a signature presented as a rule rather than a limitation a reader may meet",
    ),
    (
        "signature state is informational",
        "the same, presented as a detail rather than something that reaches a person",
    ),
    (
        "does not affect anything",
        "the same, presented as having no consequence",
    ),
];

/// Phrasings that commit to a definite outcome of SmartScreen, in either
/// direction.
///
/// The honest form is **may**. A definite prompt is the same unmeasured claim as
/// a definite absence of one — it is the safe direction rather than the
/// dangerous one, and it is still a claim about a service this project cannot
/// observe.
const DEFINITE_PROMPT_CLAIMS: &[(&str, &str)] = &[
    (
        "will warn",
        "a definite claim about what SmartScreen does, which is a Microsoft service driven by \
         telemetry this project does not hold",
    ),
    ("will show", "the same claim about a prompt"),
    ("will display", "the same claim"),
];

/// The phrase families, each with the acceptance clause it comes from.
fn phrase_families() -> Vec<(&'static str, &'static [(&'static str, &'static str)])> {
    vec![
        ("claim a SmartScreen reputation", REPUTATION_CLAIMS),
        ("claim signing that did not happen", SIGNING_CLAIMS),
        (
            "present the absence of a signature as neutral",
            NEUTRAL_ABSENCE_CLAIMS,
        ),
        (
            "commit to a definite SmartScreen outcome",
            DEFINITE_PROMPT_CLAIMS,
        ),
    ]
}

// --- reading --------------------------------------------------------------

fn repository_root() -> PathBuf {
    sure_testkit::repository_root()
}

/// A tracked file of the checkout, with its line endings normalized.
///
/// `.gitattributes` declares `* text=auto eol=lf`, and this machine's checkout
/// is not the only place these files are read, so a comparison written against
/// LF normalizes rather than assumes — the same reason, and the same shape, as
/// `read` in `crates/sure-testkit/tests/ci_workflow.rs`.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// The five files, as text, in the order `STATEMENTS` names them.
fn statements() -> Vec<(&'static str, String)> {
    STATEMENTS.iter().map(|path| (*path, read(path))).collect()
}

// --- the rules ------------------------------------------------------------

/// The divergence rule on its own, so both the general rules and the test that
/// names it are one implementation.
fn reader_divergence(text_of: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let bodies = READERS
        .iter()
        .filter_map(|path| text_of(path).map(|text| (*path, function_body(&text, READER_NAME))))
        .collect::<Vec<_>>();
    if bodies.len() != READERS.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (path, body) in &bodies {
        if body.is_none() {
            out.push(format!(
                "{path} has no `function {READER_NAME} {{`, so it either stopped reading \
                 RELEASE.txt's signature line or reads it some other way"
            ));
        }
    }
    let (Some((first_path, first)), Some((second_path, second))) = (
        bodies[0].1.as_ref().map(|b| (bodies[0].0, b)),
        bodies[1].1.as_ref().map(|b| (bodies[1].0, b)),
    ) else {
        return out;
    };
    if first != second {
        out.push(format!(
            "the two readers of RELEASE.txt have drifted: {first_path} and {second_path} no longer \
             hold the same function {READER_NAME}, so one of them was edited alone. Two copies of a \
             reader are two readers, and these two would put different states in front of the same \
             person about the same archive"
        ));
        return out;
    }
    for anchor in ["^signature\\s+(\\S+)", "$inBlock"] {
        if !first.contains(anchor) {
            out.push(format!(
                "the shared reader no longer carries {anchor:?}, so what it does is not what this \
                 repository believes it does"
            ));
        }
    }
    out
}

/// Collapse every run of whitespace to a single space.
///
/// A Markdown paragraph or a PowerShell here-string is reflowed by every editor
/// that has ever touched it, so a rule about a *sentence* must not be a rule
/// about where the line breaks happen to fall. Squashing both the text and the
/// anchor makes a rewrap invisible to a rule, and makes a phrase split across a
/// line break visible to one — which is the direction that catches something.
/// The first version of these rules read raw text, and
/// `docs/development/INSTALL_WINDOWS.md` was reported as not carrying
/// `not measurable from here` purely because the line ended between "from" and
/// "here". A rule that goes red on a reflow is a rule that gets deleted.
fn squashed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every anchor that has to be present, checked against one file's text.
fn required_in(path: &str, text: &str, required: &[(&str, &str)]) -> Vec<String> {
    // Settled here rather than at the call sites so that a reader that came back
    // with nothing produces a complaint rather than silence.
    if text.trim().is_empty() {
        return vec![format!(
            "{path} was read as empty, so nothing here says whether it still carries what the \
             signing acceptance asks of it"
        )];
    }
    let haystack = squashed(text);
    required
        .iter()
        .filter(|(anchor, _)| !haystack.contains(&squashed(anchor)))
        .map(|(anchor, why)| format!("{path} does not carry {anchor:?}, and {why}"))
        .collect()
}

/// Every anchor that has to be absent, checked against one file's text.
fn forbidden_in(path: &str, text: &str, forbidden: &[(&str, &str)]) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let haystack = squashed(text).to_lowercase();
    forbidden
        .iter()
        .filter(|(phrase, _)| haystack.contains(&squashed(phrase).to_lowercase()))
        .map(|(phrase, why)| format!("{path} carries {phrase:?}, and {why}"))
        .collect()
}

/// Every rule, over the five files.
///
/// Empty means the files still carry what the two acceptance clauses ask of
/// them. It is a function of the texts so that the ways each rule could go false
/// can be fed to it: the edits in `BREAKS` never touch the real files.
fn signing_violations(files: &[(&str, String)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let text_of = |wanted: &str| -> Option<&str> {
        files
            .iter()
            .find(|(path, _)| *path == wanted)
            .map(|(_, text)| text.as_str())
    };
    // Every file the rules are about has to be there. A file that went missing
    // is a rule passing because it had nothing to read.
    for path in STATEMENTS {
        if text_of(path).is_none() {
            out.push(format!(
                "{path} is one of the files the Windows signing statement lives in and it was not \
                 read at all, so every rule about it passed on nothing"
            ));
        }
    }

    // --- the first clause: the limitation is marked, and it is measured

    if let Some(text) = text_of(PACKAGER) {
        out.extend(required_in(PACKAGER, text, PACKAGER_REQUIRED));
        out.extend(forbidden_in(PACKAGER, text, PACKAGER_FORBIDDEN));
    }
    if let Some(text) = text_of(INSTALLER) {
        out.extend(required_in(INSTALLER, text, INSTALLER_REQUIRED));
        out.extend(forbidden_in(INSTALLER, text, INSTALLER_FORBIDDEN));
    }
    if let Some(text) = text_of(RELEASE_PROCESS) {
        out.extend(required_in(RELEASE_PROCESS, text, AUTHORITATIVE_REQUIRED));
    }
    if let Some(text) = text_of(INSTALL_WINDOWS) {
        out.extend(required_in(INSTALL_WINDOWS, text, INSTALL_WINDOWS_REQUIRED));
    }

    // --- the second clause: no faked signature, no claimed reputation

    for path in SCANNED_FOR_PHRASING {
        let Some(text) = text_of(path) else { continue };
        for (family, phrases) in phrase_families() {
            for violation in forbidden_in(path, text, phrases) {
                out.push(format!("{violation} (this would {family})"));
            }
        }
    }

    // --- and the one reader of the archive's own line, held together

    out.extend(reader_divergence(&|wanted: &str| {
        text_of(wanted).map(str::to_owned)
    }));

    out.dedup();
    out
}

// --- the repository, against every rule -----------------------------------

#[test]
fn the_windows_signing_statement_satisfies_every_rule() {
    let found = signing_violations(&statements());
    assert!(
        found.is_empty(),
        "the Windows signing statement no longer carries what the acceptance asks of it:\n{}",
        found
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The reader found the mechanism rather than inventing a pass.
///
/// Without this, a reader that came back with an empty string would satisfy
/// every "does not carry" rule and the phrase rules would have nothing to scan —
/// the failure mode of a checker that is green because it is blind.
#[test]
fn the_reader_sees_the_mechanism_that_is_there() {
    let files = statements();
    for (path, text) in &files {
        assert!(
            text.len() > 200,
            "{path} was read as {} bytes, which is not a file this repository has",
            text.len()
        );
    }
    let packager = read(PACKAGER);
    for anchor in [
        "Get-AuthenticodeSignature",
        "NotSigned",
        "cannot-confirm",
        "signed",
        "signature",
    ] {
        assert!(
            packager.contains(anchor),
            "the reader did not find {anchor:?} in {PACKAGER}; what it found of the mechanism \
             was {:?}",
            PACKAGER_REQUIRED
                .iter()
                .map(|(wanted, _)| (wanted, packager.contains(wanted)))
                .collect::<Vec<_>>()
        );
    }
    let installer = read(INSTALLER);
    assert!(
        installer.contains("signature"),
        "the reader did not find the word `signature` anywhere in {INSTALLER}, so the rule that \
         the installer reports the archive's own line has nothing to read"
    );
    // And the authoritative statement is the file the prohibition list is in,
    // which is the reason its exemption from the phrase rules is legitimate.
    assert!(
        read(RELEASE_PROCESS).contains("What must not be written"),
        "{RELEASE_PROCESS} no longer carries the prohibition list, and that list is the whole \
         reason it is exempt from the phrase rules"
    );
}

/// An empty corpus is not a pass, and neither is one file missing.
#[test]
fn a_file_that_is_not_there_is_not_a_pass() {
    let found = signing_violations(&[]);
    assert!(
        !found.is_empty(),
        "an empty corpus satisfied every rule, so the rules below prove nothing about the files \
         that are there"
    );
    for wanted in [PACKAGER, INSTALLER, RELEASE_PROCESS] {
        assert!(
            found.iter().any(|violation| violation.contains(wanted)),
            "a corpus with nothing in it produced no complaint about {wanted}: {found:#?}"
        );
    }

    // A packager that says the right words in an empty file, and a file whose
    // text is there but whose mechanism is not.
    let with_flat_sentence = statements()
        .into_iter()
        .map(|(path, text)| {
            if path == PACKAGER {
                (
                    path,
                    format!("{text}\n# THIS BUILD IS UNSIGNED\n# {INSTALLER}\n"),
                )
            } else {
                (path, text)
            }
        })
        .collect::<Vec<_>>();
    let found = signing_violations(&with_flat_sentence);
    assert!(
        found
            .iter()
            .any(|violation| violation.contains("THIS BUILD IS UNSIGNED")),
        "the unconditional unsigned heading was read as acceptable: {found:#?}"
    );
}

/// The two exemptions are findings, not conveniences, and they must keep being.
///
/// `docs/development/RELEASE_PROCESS.md` is exempt because it **lists** these
/// phrasings in order to forbid them, and `.github/workflows/release.yml` is
/// exempt because it carries one of them and this task may not edit that file.
/// Both exemptions stop being legitimate the moment their reason stops holding,
/// so both are asserted here: an exemption that no longer earns its place is an
/// exemption nobody notices is hiding a rule.
#[test]
fn the_two_exemptions_are_still_earning_their_place() {
    // 1. The authoritative statement really does name the forbidden phrasings,
    //    which is why a substring rule cannot be applied to it.
    let authoritative = read(RELEASE_PROCESS);
    let named = phrase_families()
        .iter()
        .flat_map(|(_, phrases)| phrases.iter())
        .filter(|(phrase, _)| {
            authoritative
                .to_lowercase()
                .contains(&phrase.to_lowercase())
        })
        .count();
    assert!(
        named >= 3,
        "only {named} of the forbidden phrasings appear in {RELEASE_PROCESS}, so it is no longer \
         the file that lists them in order to forbid them — and the exemption from the phrase \
         rules should be deleted rather than left standing"
    );

    // 2. The release notes really do carry the definite form this task reported
    //    instead of editing.
    let workflow = read(RELEASE_WORKFLOW);
    assert!(
        workflow.contains("Windows will warn"),
        "{RELEASE_WORKFLOW} no longer carries a definite SmartScreen claim, so the exemption from \
         DEFINITE_PROMPT_CLAIMS is no longer needed: delete it, and put this test with it"
    );
}

// --- the rules against edits that break them ------------------------------

/// An edit to one of the real files that must turn a rule red.
struct Break {
    /// What the edit is, in a sentence.
    what: &'static str,
    /// Which file the edit is made to.
    file: &'static str,
    /// Text taken out of that file as it stands.
    from: &'static str,
    /// What goes in its place.
    to: &'static str,
    /// Text the resulting violation has to carry.
    wanted: &'static str,
}

/// Every rule, broken the way it would really be broken.
///
/// Same discipline as `BREAKS` in `crates/sure-testkit/tests/ci_workflow.rs`:
/// the edits are taken from the files' own bytes rather than written as
/// fixtures, so each one proves the checker reacts to *these* files; and a
/// `replace` that found nothing is a failure rather than a pass, because that is
/// the way this kind of table rots into decoration.
///
/// The phrase entries are the ones a reader would actually write, and the
/// generated test below covers the rest of the lists.
const BREAKS: &[Break] = &[
    Break {
        what: "the packager stops asking Windows the question",
        file: PACKAGER,
        from: "$signature = Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop",
        to: "$signature = $null",
        wanted: "Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop",
    },
    Break {
        what: "the packager loses the arm for the answer that there is no signature",
        file: PACKAGER,
        from: "$reading.Status -eq 'NotSigned'",
        to: "$false",
        wanted: "$reading.Status -eq 'NotSigned'",
    },
    Break {
        what: "the state in which nothing was read is folded into an absence",
        file: PACKAGER,
        from: "State  = 'cannot-confirm'",
        to: "State  = 'not-signed'",
        wanted: "State = 'cannot-confirm'",
    },
    Break {
        what: "the reason a reading can fail is taken out of the archive's own text",
        file: PACKAGER,
        from: "Microsoft.PowerShell.Security, which does not load in every host",
        to: "a PowerShell module, which does not load in every host",
        wanted: "does not carry \"Microsoft.PowerShell.Security, which does not load in every host\"",
    },
    Break {
        what: "the archive's paragraph goes back to being written flat",
        file: PACKAGER,
        from: "switch ($exeSignature.State)",
        to: "switch ('not-signed')",
        wanted: "switch ($exeSignature.State)",
    },
    Break {
        what: "the switch keeps every arm and stops selecting the one a reading that found nothing takes",
        file: PACKAGER,
        from: "'not-signed' {",
        to: "'notsigned' {",
        wanted: "switch ($exeSignature.State) { 'not-signed' {",
    },
    Break {
        what: "the header block stops carrying the state in one word",
        file: PACKAGER,
        from: "$signatureToken = $exeSignature.State",
        to: "$signatureToken = 'not-signed'",
        wanted: "$signatureToken = $exeSignature.State",
    },
    Break {
        what: "the packager stops pointing at the authoritative statement",
        file: PACKAGER,
        from: "RELEASE_PROCESS.md\nrecords what that means under",
        to: "the release process document records what that means under",
        wanted: "RELEASE_PROCESS.md records what that means under",
    },
    Break {
        what: "the reading is no longer taken on the staged binary",
        file: PACKAGER,
        from: "Get-SignatureReading -Path $stagedExe",
        to: "Get-SignatureReading -Path $builtExe",
        wanted: "Get-SignatureReading -Path $stagedExe",
    },
    Break {
        what: "the verify step stops using the shared reader of the archive's own line",
        file: PACKAGER,
        from: "Get-StatedSignature -Path $releaseTxtPath",
        to: "Get-StatedSignature -Path (Join-Path $extractDir 'RELEASE.txt')",
        wanted: "Get-StatedSignature -Path $releaseTxtPath",
    },
    Break {
        what: "the unconditional unsigned heading comes back",
        file: PACKAGER,
        from: "WHAT THIS IS\n",
        to: "WHAT THIS IS\nTHIS BUILD IS UNSIGNED\n",
        wanted: "THIS BUILD IS UNSIGNED",
    },
    Break {
        what: "the installer stops reading the archive's own line",
        file: INSTALLER,
        from: "Get-StatedSignature -Path (Join-Path $BinDirectory 'RELEASE.txt')",
        to: "''",
        wanted: "Get-StatedSignature -Path (Join-Path $BinDirectory 'RELEASE.txt')",
    },
    Break {
        what: "the installer's arm for a state that was not established is folded into an absence",
        file: INSTALLER,
        from: "'cannot-confirm' {",
        to: "'not-signed' {",
        wanted: "'cannot-confirm' {",
    },
    Break {
        what: "the installer stops naming the authoritative statement",
        file: INSTALLER,
        from: "docs/development/RELEASE_PROCESS.md, \"Signing\",",
        to: "the release process document, \"Signing\",",
        wanted: "project can measure; docs/development/RELEASE_PROCESS.md,",
    },
    Break {
        what: "the installer's copy of the reader stops looking only at the header block",
        file: INSTALLER,
        from: "        $inBlock = $true\n        if ($line.Trim() -match '^signature\\s+(\\S+)') { $token = $Matches[1]; break }",
        to: "        if ($line.Trim() -match '^signature\\s+(\\S+)') { $token = $Matches[1]; break }",
        wanted: "drifted",
    },
    Break {
        what: "the installer asserts the state instead of reporting it",
        file: INSTALLER,
        from: "Write-Step 'Done'",
        to: "Write-Step 'Done'\nWrite-Detail 'unsigned     this build has no Authenticode signature'",
        wanted: "this build has no Authenticode signature",
    },
    Break {
        what: "the installer becomes a second measurer of the same fact",
        file: INSTALLER,
        from: "Write-Step 'Done'",
        to: "Write-Step 'Done'\n$exeSig = Get-AuthenticodeSignature -LiteralPath $installedExe",
        wanted: "second measurer",
    },
    Break {
        what: "the authoritative statement stops naming the boundary",
        file: RELEASE_PROCESS,
        from: "SmartScreen is not measurable here at all",
        to: "SmartScreen is a Microsoft service",
        wanted: "not measurable",
    },
    Break {
        what: "the prohibition stops being spelled out",
        file: RELEASE_PROCESS,
        from: "### What must not be written",
        to: "### A note about tone",
        wanted: "What must not be written",
    },
    Break {
        what: "the install-time statement stops pointing at the reason",
        file: INSTALL_WINDOWS,
        from: "`RELEASE_PROCESS.md`'s `## Signing` is where the reason, the boundary and the",
        to: "the release process is where the reason, the boundary and the",
        wanted: "is where the reason, the boundary and the",
    },
    Break {
        what: "the install-time statement stops saying what can be measured",
        file: INSTALL_WINDOWS,
        from: "is **not measurable from\nhere**",
        to: "is **not measured**",
        wanted: "not measurable from here",
    },
    Break {
        what: "the section a reader is sent to is retitled out of existence",
        file: INSTALL_WINDOWS,
        from: "## The build is unsigned, and how that is known",
        to: "## The build carries no signature",
        wanted: "The build is unsigned, and how that is known",
    },
    Break {
        what: "a reader is promised they will not meet a prompt",
        file: INSTALL_WINDOWS,
        from: "runs. That is the consequence of the state above, not a fault in the archive or",
        to: "runs, and you will not see a warning more than once. That is the consequence of the \
             state above, not a fault in the archive or",
        wanted: "will not see a warning",
    },
    Break {
        what: "the packager commits to a definite prompt",
        file: PACKAGER,
        from: "Windows may show a SmartScreen or \"unknown publisher\" prompt the first time the\nprogram is run.",
        to: "Windows will show a SmartScreen or \"unknown publisher\" prompt the first time the\nprogram is run.",
        wanted: "will show",
    },
    Break {
        what: "the installer presents the absence as a neutral fact",
        file: INSTALLER,
        from: "Write-Step 'Done'",
        to: "Write-Step 'Done'\nWrite-Detail 'note         the signature state does not affect anything'",
        wanted: "does not affect anything",
    },
];

#[test]
fn every_rule_is_turned_red_by_an_edit_that_breaks_it() {
    let real = statements();
    assert!(
        signing_violations(&real).is_empty(),
        "the files are already failing a rule, so no edit below can be said to have broken it"
    );
    for edit in BREAKS {
        let mut edited = real.clone();
        let mut applied = false;
        for (path, text) in edited.iter_mut() {
            if *path != edit.file {
                continue;
            }
            let after = text.replace(edit.from, edit.to);
            assert_ne!(
                after, *text,
                "\"{}\" edits text that is not in {} any more ({}), so it would have mutated \
                 nothing and passed for that reason instead. The table has to be re-pointed at what \
                 the file says now.",
                edit.what, edit.file, edit.from
            );
            *text = after;
            applied = true;
        }
        assert!(
            applied,
            "\"{}\" names the file {} and no such file was read",
            edit.what, edit.file
        );
        let found = signing_violations(&edited);
        assert!(
            found
                .iter()
                .any(|violation| violation.contains(edit.wanted)),
            "breaking `{}` produced no violation carrying {:?}:\n{found:#?}",
            edit.what,
            edit.wanted
        );
    }
}

/// Every forbidden phrase is one this reader reports.
///
/// The same test `FORBIDDEN_ACTS` and `PUBLICATION_CHANNELS` have in
/// `ci_workflow.rs`, for the same reason: a list of phrases is decoration unless
/// each entry is one the rule actually finds. Each phrase is injected into a
/// real file as a real sentence and the rules are required to name it, so a
/// typo in the list, or an entry the rule stopped reading, is a red test rather
/// than a line in a table that looks like a check.
#[test]
fn every_forbidden_phrase_is_one_this_reader_reports() {
    let real = statements();
    assert!(signing_violations(&real).is_empty());

    for (family, phrases) in phrase_families() {
        for (phrase, why) in phrases {
            let mut edited = real.clone();
            for (path, text) in edited.iter_mut() {
                if *path == INSTALLER {
                    *text = format!("{text}\nWrite-Detail '{phrase}'\n");
                }
            }
            let found = signing_violations(&edited);
            assert!(
                found.iter().any(|violation| violation.contains(phrase)),
                "injecting {phrase:?} ({why}) into {INSTALLER} produced no violation naming it, so \
                 the entry is a line in a list rather than a check, and it would {family}:\n{found:#?}"
            );
        }
    }
}
