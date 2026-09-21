//! The macOS signing and notarization statement, and what it is allowed to say.
//!
//! `P15-T013`'s acceptance is two sentences, and neither of them says "write a
//! paragraph":
//!
//! > *If Apple credentials are unavailable, mark external limitation honestly.*
//! > *Do not fake signing/notarization.*
//!
//! The first asks for an honest **statement of an external limitation**; the
//! second is a **prohibition on a specific lie** — presenting a signature as a
//! notarization, or presenting either as something this project did.
//!
//! # The distinction this file exists to hold
//!
//! `P15-T012` documented and measured the Windows **signing** half, and its
//! file, `crates/sure-testkit/tests/signing_status.rs`, is the design this one
//! extends. `docs/development/RELEASE_PROCESS.md` handed the rest of the macOS
//! half here by name: `scripts/Build-Release.sh` reads the macOS artifact's
//! signature with `codesign -d` and reports it — and **notarization is not read
//! by that step at all**.
//!
//! Those are two different facts and they are easy to run together:
//!
//! * a **signature** is bytes inside the binary, applied by `codesign --sign`,
//!   and the field a reading prints for it is `Authority=` and `Signature=`;
//! * a **notarization** is a ticket Apple holds, produced by `xcrun notarytool
//!   submit`, and what a reader meets is a **staple** that `xcrun stapler`
//!   attaches — which is not a field `codesign` prints and not a question
//!   `codesign -d` was asked.
//!
//! So an `Authority=` line being absent is not a notarization result, and a
//! sentence that reads one as the other is the exact failure this task exists
//! to prevent. `pgrep -f notarytool` on any machine in this repository returns
//! nothing because there is nothing to find: **no Apple Developer ID
//! certificate and no notarization credential are configured for this
//! project**, and the acceptance's answer to that is to say so, not to pretend
//! otherwise.
//!
//! # What was wrong, and where the two guards belong
//!
//! Five files said the macOS artifacts were not signed and not notarized —
//! `docs/development/RELEASE_PROCESS.md` at several points,
//! `docs/development/MACOS.md`, `scripts/Build-Release.sh` and the two
//! workflows — and **the word "staple" appeared in none of them**, in any
//! spelling: `git grep -i stapl` over the tracked tree returned nothing before
//! this task. Notarization and stapling are one state reported twice, so a
//! statement that names only the first leaves the reader to supply the second.
//! And not one of those sentences was held by anything:
//! `git grep -i notariz -- 'crates/*/tests/*.rs'` returned nothing, so every
//! one of them could have been deleted, or inverted, with every gate staying
//! green.
//!
//! There are therefore **two guards rather than one**, and they point in
//! opposite directions on purpose:
//!
//! * the **phrase rules** below read the files a person meets the claim in and
//!   fail on the forms that would overclaim it;
//! * the **build-path rules** read the files that *do* the building and fail
//!   when a notarization step or an Apple credential appears in one — because
//!   the moment a step exists, every sentence saying no step exists becomes the
//!   defect instead.
//!
//! Neither guard can be satisfied by the other. A phrase rule is blind to a
//! step nobody has written a sentence about yet; a structural rule is blind to
//! a sentence that describes a step that is not there.
//!
//! # What this file proves, and what it cannot
//!
//! It reads text. It proves that the **statement of the limitation is still
//! written down** — that the authoritative section still names the three
//! missing things, still says who would have to provide them, still states what
//! a downloader meets, and still keeps *signed*, *notarized* and *stapled*
//! apart; and that the four other files still carry the sentence each of them
//! is the reader's first sight of. And it proves that **four families of
//! forbidden phrasing are absent** from those files.
//!
//! It cannot prove any of the following, and no test here should be read as
//! proving them:
//!
//! * that any of these sentences is true of a **run**. What the build does with
//!   a notarization is measured by there being no such step, which is what the
//!   build-path rules check as far as text can — but a step added in a file
//!   nobody listed here would be invisible, and a step invoked from outside
//!   this repository entirely would be invisible to every rule in it;
//! * that **Apple's answer** would be what this statement says. Nothing here
//!   can submit an archive to Apple, hold a ticket, or observe a staple. This
//!   repository holds no Apple account, which is the limitation being marked;
//! * that **a person will meet a Gatekeeper prompt**, or will not. Nothing in
//!   this repository has observed Gatekeeper at all — no test downloads an
//!   archive on a Mac and launches it, and `docs/development/RELEASE_PROCESS.md`
//!   says so in the same words. A rule over a phrase list cannot enumerate the
//!   phrasing nobody thought of.
//!
//! # Why one file is exempt from the phrase rules
//!
//! `docs/development/RELEASE_PROCESS.md` is the authoritative statement and
//! therefore the file that **lists these phrasings in order to forbid them**. A
//! substring rule cannot tell naming a lie from telling one, so deleting this
//! exemption would make the prohibition list illegal to write, which is the
//! opposite of what the acceptance asks for. What guards it instead is a
//! presence rule plus
//! `the_exemption_is_still_earning_its_place` below, which fails the moment it
//! stops being the file that lists them — so an exemption cannot quietly become
//! permanent. This is the same exemption, for the same reason, that
//! `signing_status.rs` makes for the Windows half.
//!
//! # Why the reader is a line reader, and why it reports what it found
//!
//! `docs/development/RELEASE_PROCESS.md` and `docs/development/MACOS.md` are
//! Markdown, `scripts/Build-Release.sh` is POSIX shell and the three workflows
//! are YAML, so there is no one parse of all of them and none of these
//! properties is about structure. What keeps a text reader honest is that it
//! says what it found: `the_reader_sees_the_mechanism_that_is_there` fails if
//! the anchors were read as absent, and `a_file_that_is_not_there_is_not_a_pass`
//! feeds the rules nothing and requires every one of them to complain.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// The file that writes the macOS archive's own paragraph and reads the
/// binary's signature.
const BUILD_SCRIPT: &str = "scripts/Build-Release.sh";

/// The Windows packager. It builds no macOS artifact, and it is on the build
/// path for that reason: a notarization step here would be as wrong as one in
/// the shell script.
const WINDOWS_PACKAGER: &str = "scripts/Build-Release.ps1";

/// The file that verifies the four archives and writes the checksum file.
const ASSEMBLER: &str = "scripts/Assemble-Release.sh";

/// The workflow that attaches the archives to a draft release.
const RELEASE_WORKFLOW: &str = ".github/workflows/release.yml";

/// The workflow whose three jobs actually build the macOS artifacts.
const DRY_RUN_WORKFLOW: &str = ".github/workflows/release-dry-run.yml";

/// The platform's own CI workflow.
const CI_WORKFLOW: &str = ".github/workflows/ci.yml";

/// The authoritative statement of the limitation.
const AUTHORITATIVE: &str = "docs/development/RELEASE_PROCESS.md";

/// The statement a macOS reader meets in the documentation.
const MACOS_DOC: &str = "docs/development/MACOS.md";

/// Every file that states the macOS signing/notarization limitation.
const STATEMENTS: [&str; 5] = [
    BUILD_SCRIPT,
    AUTHORITATIVE,
    MACOS_DOC,
    RELEASE_WORKFLOW,
    DRY_RUN_WORKFLOW,
];

/// The four files a phrase rule can read without reading a quotation.
///
/// See "Why one file is exempt from the phrase rules" in this file's header.
const SCANNED_FOR_PHRASING: [&str; 4] =
    [BUILD_SCRIPT, MACOS_DOC, RELEASE_WORKFLOW, DRY_RUN_WORKFLOW];

/// Every file that builds, packages, assembles or releases an archive.
///
/// These are the files the sentence *"no step of this build notarizes"* is
/// about, and the structural rules below read them for the opposite of what a
/// presence rule reads for: a notarization tool or an Apple credential
/// appearing in one of them means the sentence has stopped being true and the
/// documentation has to change with it.
///
/// The set is listed rather than globbed on purpose. A glob over
/// `.github/workflows/*.yml` would silently grow to cover a new workflow, which
/// sounds like a feature and is not one: a new workflow is a new place a
/// notarization step could live, and the honest thing is for a person adding
/// one to be told to decide whether it belongs here rather than for a rule to
/// have already decided.
const BUILD_PATH: [&str; 6] = [
    BUILD_SCRIPT,
    WINDOWS_PACKAGER,
    ASSEMBLER,
    CI_WORKFLOW,
    RELEASE_WORKFLOW,
    DRY_RUN_WORKFLOW,
];

/// Every file any rule in this file reads.
///
/// The corpus is the statements plus the build path rather than either alone,
/// so that a file that went missing is a complaint from whichever rule was
/// supposed to hold it, instead of a rule that passed because it had nothing to
/// read.
const CORPUS: [&str; 8] = [
    BUILD_SCRIPT,
    WINDOWS_PACKAGER,
    ASSEMBLER,
    CI_WORKFLOW,
    RELEASE_WORKFLOW,
    DRY_RUN_WORKFLOW,
    AUTHORITATIVE,
    MACOS_DOC,
];

/// What the macOS packager has to keep saying for the claim to be measured.
///
/// Each entry is a literal substring of `scripts/Build-Release.sh` as it
/// stands, and the sentence is what its disappearance would mean. These are
/// presence rules and the header says what a presence rule cannot prove.
///
/// Each entry is anchored on **the code that produces the value** rather than
/// on a name a comment could satisfy, which is the rule `signing_status.rs`
/// arrived at over three rounds of mutation testing and which
/// `every_rule_is_turned_red_by_an_edit_that_breaks_it` below re-checks here.
/// The one entry that is a comment is a comment deliberately: *"notarization is
/// not read here at all"* is a claim about an **absence**, so the code it names
/// is code that is not written — the sibling rules that read `BUILD_PATH` for a
/// notarization tool are what make it checkable, and the anchor holds the two
/// halves together so neither can be deleted alone.
const BUILD_SCRIPT_REQUIRED: &[(&str, &str)] = &[
    (
        "RELEASE_SIGNATURE_SECTION=\"THIS BUILD CARRIES NO APPLE DEVELOPER ID SIGNATURE AND NOTHING NOTARIZES IT",
        "the archive's own text stops saying that nothing notarizes it, so the person holding the \
         file is told about the signature half of a two-part state and left to infer the other — \
         which is the state the acceptance asks to be marked rather than implied. The anchor is the \
         assignment and not the heading, because the assignment is what puts the words in the \
         archive",
    ),
    (
        "Developer ID signing and notarization as an external credential this project does not",
        "the archive's own text stops naming notarization as an external credential this project \
         does not have, so what is missing is not written where the person holding the file can \
         read it",
    ),
    (
        "Notarization is not read here at all**: nothing in this build notarizes",
        "either the boundary is gone or the reason is. The two are held as one anchor on purpose: \
         *not read here* without *nothing notarizes* is a sentence a reader can take as an \
         oversight to be fixed, and *nothing notarizes* without *not read here* is a claim with no \
         statement of which half is measured. `codesign -d` reads a signature; a deletion of either \
         half is what turns this step's output into a notarization reading in a reader's mind",
    ),
    (
        "grep -q '^Authority=' \"$sign_log\"",
        "the failure stops being keyed on a **signing** field. `Authority=` is what a certificate \
         produces and is not a notarization field; the day the branch keys on anything else, what \
         the step refuses is no longer the thing the archive's text contradicts",
    ),
    (
        "run_captured codesign \"$sign_out\" \"$sign_err\" -d --verbose=2 \"$extracted_binary\"",
        "the one reading this project takes stops being taken, so the archive's paragraph about its \
         own signature is asserted rather than read — which is the standard the Windows half was \
         brought up to, going back down",
    ),
    (
        "Nothing in SURE asks you to turn Gatekeeper off",
        "the archive stops telling the person holding it that this project is not asking them to \
         weaken their own machine to run it. That sentence is the consequence of the unsigned state \
         stated as a limit on what this project asks for, and without it the paragraph reads as a \
         description of a difficulty rather than as a boundary on SURE",
    ),
];

/// What the authoritative statement has to keep carrying.
///
/// The three missing things, the person who would have to provide them, the
/// consequence for a downloader, the boundary on what has been observed, and —
/// separately — the distinction between the two facts, which is the part a
/// later edit is most likely to fold away as redundant.
const AUTHORITATIVE_REQUIRED: &[(&str, &str)] = &[
    (
        "### macOS: no Developer ID, no notarization and no staple",
        "the section a macOS reader is sent to is gone or retitled, and with it the only place that \
         states the notarization half as its own fact rather than as an aside to the Windows one",
    ),
    (
        "**Developer ID Application certificate**",
        "the first of the three missing things is not named, so \"no credential\" stops saying which \
         credential and becomes a reader's guess",
    ),
    (
        "**notarization credential**",
        "the second of the three missing things is not named. It is a separate item from the \
         certificate on purpose: a certificate signs and a notarization credential submits, and a \
         project can hold either without the other",
    ),
    (
        "**Apple Developer Program membership**",
        "the third of the three missing things is not named, so who would have to provide the other \
         two is not written down",
    ),
    (
        "An Apple Developer Program member with an Apple account",
        "the statement stops saying who would have to provide what is missing. A limitation whose \
         holder is unnamed is one a reader can only wait out",
    ),
    (
        "a person who downloads one meets Gatekeeper",
        "the consequence is gone, so the section describes what this project does not have without \
         saying what that does to the person on the other end of it",
    ),
    (
        "Nothing in this repository asks anyone to turn Gatekeeper off",
        "the boundary on what this project asks of a reader is gone from the authoritative \
         statement, leaving the Gatekeeper sentence to be read as advice rather than as a limit",
    ),
    (
        "It is not a notarization field",
        "the distinction between a signing field and a notarization field is gone, so `Authority=` \
         is free to be read as a notarization result — which is the conflation this task exists to \
         prevent",
    ),
    (
        "Notarization is not read by that step at all",
        "the sentence `P15-T013` was handed is gone, and with it the statement that the notarization \
         half rests on there being no step rather than on the `codesign` reading",
    ),
    (
        "has no staple",
        "the stapling half is gone, so an artifact that was never notarized is described without \
         the word for what a reader will not find",
    ),
    (
        "unobserved here and stays unobserved",
        "the boundary on what has been observed is gone, so the section reads as a description of \
         what a Mac does rather than as a statement of what this project has and has not seen",
    ),
];

/// What the macOS platform document has to keep carrying.
const MACOS_DOC_REQUIRED: &[(&str, &str)] = &[
    (
        "No Apple Developer ID certificate and no notarization credential are configured",
        "the platform document stops saying the credentials are missing, so a reader who starts \
         here leaves with \"an optional enhancement\" and not with \"one this project does not \
         have\"",
    ),
    (
        "a person who downloads one meets Gatekeeper",
        "the document states a state without its consequence, which is the shape the acceptance's \
         second clause is about — a reader surprised by a prompt they were never told to expect",
    ),
    (
        "no notarization and no staple",
        "the document stops pointing at the authoritative section, so the reader has the conclusion \
         and no way to the reason, the missing list or the prohibition",
    ),
    (
        "has observed what Gatekeeper does with either archive on a Mac",
        "the document stops saying that Gatekeeper is unobserved, so its Gatekeeper sentence reads \
         as a reading rather than as the general shape of what Apple's software does",
    ),
];

/// What the release notes have to keep carrying.
const RELEASE_WORKFLOW_REQUIRED: &[(&str, &str)] = &[
    (
        "Nothing here is signed or notarized",
        "the release notes stop saying it, so the one place a person meets all four archives at \
         once says nothing about the two macOS ones",
    ),
    (
        "no Apple Developer ID is configured",
        "the notes stop naming the missing Apple credential while still naming the missing \
         Authenticode one, so the two halves of \"not signed\" are stated as one",
    ),
    (
        "a macOS download is subject to Gatekeeper",
        "the notes stop naming what a macOS downloader meets, so the Windows warning is stated and \
         the macOS one is not",
    ),
];

/// What the dry-run workflow's own header has to keep carrying.
const DRY_RUN_WORKFLOW_REQUIRED: &[(&str, &str)] = &[
    (
        "No signing, no notarization, no release, no tag and no",
        "the workflow that builds both macOS artifacts stops saying what it does not do, so the job \
         that produces the bytes reads as though the signing and notarization halves were somewhere \
         else",
    ),
    (
        "Apple Developer ID credentials are an external blocker",
        "the workflow stops naming the blocker for what it does not do, and names the \
         `## Signing` section of the release process without saying which part of it applies here",
    ),
    (
        "the binary is unsigned and unnotarized",
        "the Intel job's own record of what it built stops saying it, so the run that produced the \
         artifact and read its bytes ends without the state of the thing it read",
    ),
];

/// Tools that would mean the build has grown a notarization step.
///
/// The claim these are the counter-evidence for is *"no step of this build
/// notarizes"*, which is what every notarization sentence in this repository
/// rests on. It is a claim about an absence, so it can be checked only by
/// looking for what would end it — and the honest unit is the **tool that would
/// be invoked**, not the word "notarize", because a comment saying a thing is
/// not done is not the thing being done.
///
/// `stapler` is here and not in a family of its own because a staple is what a
/// notarization leaves behind: an artifact cannot be stapled without having been
/// notarized first, so a stapling step is a notarization step that starts
/// halfway through.
const NOTARIZATION_TOOLS: &[(&str, &str)] = &[
    (
        "notarytool",
        "the build path names Apple's notarization client, so the archive's own sentence about \
         notarization has stopped being a fact about the build procedure and has become a claim \
         about a step that now exists",
    ),
    (
        "stapler",
        "the build path names the tool that attaches a notarization ticket, so this build is being \
         made to carry a ticket this project holds no credential to obtain",
    ),
    (
        "spctl",
        "the build path names the Gatekeeper assessment tool, so a sentence about what a downloader \
         meets is being turned into a reading taken on somebody else's machine",
    ),
    (
        "altool",
        "the build path names Apple's previous notarization upload client, which is the same step \
         under its earlier name",
    ),
];

/// Credential names that would mean an Apple notarization account is configured.
///
/// This is the other half of *"if Apple credentials are unavailable"*: the
/// acceptance's clause is about what to write when they are unavailable, and
/// that clause is only honest if the unavailability is a property of the tree
/// rather than an assumption about it. A credential cannot be used by a step
/// that does not exist, which is why `NOTARIZATION_TOOLS` is the rule that
/// matters — and a credential can be **configured** anyway, which is what these
/// catch: a secret passed into a job that does nothing with it is the first
/// commit of a notarization step, not the absence of one.
const APPLE_CREDENTIALS: &[(&str, &str)] = &[
    (
        "APPLE_ID",
        "an Apple account identifier is in the build path, so the notarization credentials this \
         statement says are unavailable are being configured",
    ),
    (
        "APPLE_TEAM_ID",
        "an Apple team identifier is in the build path, which is half of a notarization credential",
    ),
    (
        "APPLE_APP_SPECIFIC_PASSWORD",
        "an app-specific password is named in the build path, which is the credential `notarytool` \
         authenticates with",
    ),
    (
        "APPLE_CERTIFICATE",
        "a Developer ID certificate is named in the build path, so the archive's \"no Apple \
         Developer ID signature\" sentence has a credential behind it to contradict",
    ),
    (
        "APP_STORE_CONNECT",
        "an App Store Connect reference is in the build path, which is the API-key route to the \
         same notarization credential",
    ),
    (
        "NOTARYTOOL_API_KEY",
        "a notarization API key is named in the build path by the name Apple gives it",
    ),
    (
        "AC_PASSWORD",
        "the legacy notarization password variable is in the build path",
    ),
];

/// Phrasings that present a notarization as having happened.
///
/// This is the family the acceptance's second clause is aimed at. Every string
/// here asserts a ticket, a review by Apple or a value Apple issued, none of
/// which this project has ever obtained — and each is written so that the
/// **negative** form does not satisfy it. `is notarized` is not a substring of
/// *is not notarized*, of *is unnotarized* or of *is unsigned or notarized*,
/// which is what keeps this family from failing the sentences it is supposed to
/// protect.
const NOTARIZATION_CLAIMS: &[(&str, &str)] = &[
    (
        "has been notarized",
        "a claim that Apple reviewed these bytes, which is a step and a credential this project \
         does not have",
    ),
    ("have been notarized", "the same claim about the archives"),
    ("was notarized", "the same claim about one archive"),
    ("were notarized", "the same claim in the plural past"),
    ("notarized by Apple", "the same claim naming the party"),
    (
        "Apple has notarized",
        "the same claim with Apple as the subject",
    ),
    (
        "Apple has vetted",
        "a claim that Apple reviewed the artifact and approved of it, which is broader than \
         notarization and no more true",
    ),
    (
        "blessed by Apple",
        "the same claim as a phrase that says nothing checkable at all",
    ),
    (
        "notarization succeeded",
        "a report of an outcome of a step this build does not run",
    ),
    (
        "notarization is complete",
        "the same report in the state form",
    ),
];

/// Phrasings that present the artifact as stapled.
///
/// A staple is what `xcrun stapler` attaches to an artifact Apple has already
/// notarized, so this family is the first one one step further downstream: a
/// stapled archive is an archive a notarization has already happened to, and
/// one that Gatekeeper can check without a network call.
const STAPLED_CLAIMS: &[(&str, &str)] = &[
    (
        "is stapled",
        "a claim that a notarization ticket is attached to these bytes, which cannot be true of an \
         artifact this project has never submitted",
    ),
    ("are stapled", "the same claim about the archives"),
    (
        "has a stapled ticket",
        "the same claim written as a possession",
    ),
    (
        "the ticket is attached",
        "the same claim without the word for what a ticket is attached with",
    ),
    (
        "stapler validate passed",
        "a report of an outcome of a tool this build never invokes",
    ),
    (
        "the staple validates",
        "the same report in the passive, which is the form a summary would use",
    ),
];

/// Phrasings that promise what Gatekeeper does with a download.
///
/// Gatekeeper is a check Apple's software performs on the downloading person's
/// machine, on an artifact whose signature and ticket this project cannot read
/// either way. Every string here asserts the outcome, in the reassuring
/// direction.
const GATEKEEPER_CLAIMS: &[(&str, &str)] = &[
    (
        "opens without a warning",
        "an assertion that a reader meets no prompt, which nothing in this repository can observe",
    ),
    (
        "will open without a warning",
        "the same assertion in the future tense",
    ),
    (
        "will pass Gatekeeper",
        "the same assertion as an outcome of a check Apple's software runs on somebody else's \
         machine",
    ),
    (
        "Gatekeeper will allow",
        "the same assertion with Gatekeeper as the subject",
    ),
    (
        "no Gatekeeper prompt",
        "the absence presented as the reader's experience, which is a fact about their machine",
    ),
    (
        "Gatekeeper-clean",
        "the same claim as a property of the artifact, which is the shape that reads as a \
         specification rather than as a promise",
    ),
];

/// Phrasings that present the absence of a notarization as a neutral fact.
///
/// The acceptance asks for a limitation to be *marked*. A sentence that presents
/// an unnotarized archive as a detail with no consequence leaves a reader
/// surprised by a refusal or a prompt they were never told to expect.
const NEUTRAL_ABSENCE_CLAIMS: &[(&str, &str)] = &[
    (
        "notarization is not required",
        "the absence of a notarization presented as a rule rather than as a limitation a reader \
         may meet",
    ),
    (
        "an unnotarized build is normal",
        "the same, presented as a norm, which leaves a reader who has never met one without a \
         reason to expect anything",
    ),
    (
        "notarization does not affect anything",
        "the same, presented as having no consequence",
    ),
    (
        "unnotarized is fine",
        "the same, presented as an assurance this project is not in a position to give",
    ),
    (
        "notarization does not matter",
        "the same, presented as a judgement rather than as a state",
    ),
];

/// The phrase families, each with the acceptance clause it comes from.
fn phrase_families() -> Vec<(&'static str, &'static [(&'static str, &'static str)])> {
    vec![
        (
            "claim a notarization that never happened",
            NOTARIZATION_CLAIMS,
        ),
        ("claim a staple that was never attached", STAPLED_CLAIMS),
        ("promise a Gatekeeper outcome", GATEKEEPER_CLAIMS),
        (
            "present the absence of a notarization as neutral",
            NEUTRAL_ABSENCE_CLAIMS,
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
/// `read` in `crates/sure-testkit/tests/signing_status.rs`.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// Every file of the corpus, as text, in the order `CORPUS` names them.
fn corpus_texts() -> Vec<(&'static str, String)> {
    CORPUS.iter().map(|path| (*path, read(path))).collect()
}

// --- the rules ------------------------------------------------------------

/// Collapse every run of whitespace to a single space, and drop the `#` that a
/// comment's line break puts in the middle of a sentence.
///
/// A Markdown paragraph or a shell comment is reflowed by every editor that has
/// ever touched it, so a rule about a *sentence* must not be a rule about where
/// the line breaks happen to fall. Squashing both the text and the anchor makes
/// a rewrap invisible to a rule, and makes a phrase split across a line break
/// visible to one — which is the direction that catches something. The same
/// helper, for the same reason, as `squashed` in `signing_status.rs`.
///
/// **The comment marker is the part `signing_status.rs` did not need and this
/// file does.** Three of the five files here are YAML, where a paragraph is a
/// run of `#`-prefixed lines, so a line break does not merely move whitespace:
/// it inserts a `#` into the middle of the sentence. `Apple Developer ID
/// credentials` / `are an external blocker` squashes to *credentials # are an
/// external blocker*, whose anchor would have to spell a `#` the sentence does
/// not contain — and a rule that goes red on a reflowed comment is a rule
/// somebody deletes. So an indented line's leading `#` is stripped before the
/// squash, and a Markdown heading's is not: a heading's `#` is at column 0 and
/// is punctuation the sentence really has. This was found rather than foreseen:
/// the first version of this file anchored on the two-line sentence and was
/// reported as not carrying it.
fn squashed(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.len() == line.len() {
                return line;
            }
            trimmed
                .strip_prefix("# ")
                .or_else(|| trimmed.strip_prefix("#"))
                .unwrap_or(trimmed)
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every anchor that has to be present, checked against one file's text.
fn required_in(path: &str, text: &str, required: &[(&str, &str)]) -> Vec<String> {
    // Settled here rather than at the call sites so that a reader that came back
    // with nothing produces a complaint rather than silence.
    if text.trim().is_empty() {
        return vec![format!(
            "{path} was read as empty, so nothing here says whether it still carries what the \
             macOS signing and notarization acceptance asks of it"
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

/// Every rule, over the corpus.
///
/// Empty means the files still carry what the two acceptance clauses ask of
/// them. It is a function of the texts so that the ways each rule could go
/// false can be fed to it: the edits in `BREAKS` never touch the real files.
fn notarization_violations(files: &[(&str, String)]) -> Vec<String> {
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
                "{path} is one of the files the macOS signing and notarization statement lives in \
                 and it was not read at all, so every rule about it passed on nothing"
            ));
        }
    }
    for path in BUILD_PATH {
        if text_of(path).is_none() {
            out.push(format!(
                "{path} is on the build path, so the claim that no step of this build notarizes \
                 was checked against a file that was not read"
            ));
        }
    }

    // --- the first clause: the limitation is marked

    if let Some(text) = text_of(BUILD_SCRIPT) {
        out.extend(required_in(BUILD_SCRIPT, text, BUILD_SCRIPT_REQUIRED));
    }
    if let Some(text) = text_of(AUTHORITATIVE) {
        out.extend(required_in(AUTHORITATIVE, text, AUTHORITATIVE_REQUIRED));
    }
    if let Some(text) = text_of(MACOS_DOC) {
        out.extend(required_in(MACOS_DOC, text, MACOS_DOC_REQUIRED));
    }
    if let Some(text) = text_of(RELEASE_WORKFLOW) {
        out.extend(required_in(
            RELEASE_WORKFLOW,
            text,
            RELEASE_WORKFLOW_REQUIRED,
        ));
    }
    if let Some(text) = text_of(DRY_RUN_WORKFLOW) {
        out.extend(required_in(
            DRY_RUN_WORKFLOW,
            text,
            DRY_RUN_WORKFLOW_REQUIRED,
        ));
    }

    // --- the second clause: no faked signature, no faked notarization

    for path in SCANNED_FOR_PHRASING {
        let Some(text) = text_of(path) else { continue };
        for (family, phrases) in phrase_families() {
            for violation in forbidden_in(path, text, phrases) {
                out.push(format!("{violation} (this would {family})"));
            }
        }
    }

    // --- and the claim the sentences rest on: no step of this build notarizes

    for path in BUILD_PATH {
        let Some(text) = text_of(path) else { continue };
        out.extend(forbidden_in(path, text, NOTARIZATION_TOOLS));
        out.extend(forbidden_in(path, text, APPLE_CREDENTIALS));
    }

    out.dedup();
    out
}

// --- the repository, against every rule -----------------------------------

/// The focused measure of the claim every notarization sentence rests on.
///
/// `notarization_violations` folds this in, so this test is not a second rule:
/// it is the same rule with a name that says what it is, over the real files
/// rather than over a corpus, so that the one thing this repository can say
/// about notarization is said in a place a reader can find.
///
/// What it establishes: no file of the build path names a notarization tool or
/// an Apple credential. What it therefore establishes: *"no step of this build
/// notarizes"* is a property of the code rather than of anybody's intention.
/// What it does not: that a step could not be added tomorrow, or that a step
/// invoked from outside this repository does not exist.
#[test]
fn nothing_in_the_build_path_notarizes_or_holds_an_apple_credential() {
    let files = corpus_texts();
    let mut found: Vec<String> = Vec::new();
    for path in BUILD_PATH {
        let (_, text) = files
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .unwrap_or_else(|| panic!("{path} is on the build path and was not read"));
        found.extend(forbidden_in(path, text, NOTARIZATION_TOOLS));
        found.extend(forbidden_in(path, text, APPLE_CREDENTIALS));
    }
    assert!(
        found.is_empty(),
        "the build path has grown a notarization step or an Apple credential, and every sentence \
         in this repository that says it has neither is now the defect rather than the statement. \
         Either remove what was added, or change the documentation in the same commit — \
         docs/development/RELEASE_PROCESS.md's `### macOS: no Developer ID, nothing notarizes, and \
         nothing is stapled` and docs/development/MACOS.md both rest on this:\n{}",
        found
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_macos_notarization_statement_satisfies_every_rule() {
    let found = notarization_violations(&corpus_texts());
    assert!(
        found.is_empty(),
        "the macOS signing and notarization statement no longer carries what the acceptance asks \
         of it:\n{}",
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
    let files = corpus_texts();
    for (path, text) in &files {
        assert!(
            text.len() > 200,
            "{path} was read as {} bytes, which is not a file this repository has",
            text.len()
        );
    }
    let script = read(BUILD_SCRIPT);
    for anchor in [
        "Notarization",
        "NOTHING NOTARIZES",
        "Authority=",
        "codesign",
    ] {
        assert!(
            script.contains(anchor),
            "the reader did not find {anchor:?} in {BUILD_SCRIPT}; what it found of the mechanism \
             was {:?}",
            BUILD_SCRIPT_REQUIRED
                .iter()
                .map(|(wanted, _)| (wanted, script.contains(wanted)))
                .collect::<Vec<_>>()
        );
    }
    let authoritative = read(AUTHORITATIVE);
    for anchor in ["staple", "Gatekeeper", "notarization credential"] {
        assert!(
            authoritative.contains(anchor),
            "the reader did not find {anchor:?} in {AUTHORITATIVE}, and this task's whole \
             deliverable is that word being there; what it found was {:?}",
            AUTHORITATIVE_REQUIRED
                .iter()
                .map(|(wanted, _)| (wanted, authoritative.contains(wanted)))
                .collect::<Vec<_>>()
        );
    }
    // And the authoritative statement is the file the prohibition list is in,
    // which is the reason its exemption from the phrase rules is legitimate.
    assert!(
        authoritative.contains("### What must not be claimed about macOS"),
        "{AUTHORITATIVE} no longer carries the macOS prohibition list, and that list is the whole \
         reason it is exempt from the phrase rules"
    );
}

/// An empty corpus is not a pass, and neither is one file missing.
#[test]
fn a_file_that_is_not_there_is_not_a_pass() {
    let found = notarization_violations(&[]);
    assert!(
        !found.is_empty(),
        "an empty corpus satisfied every rule, so the rules here prove nothing about the files \
         that are there"
    );
    for wanted in [BUILD_SCRIPT, AUTHORITATIVE, MACOS_DOC] {
        assert!(
            found.iter().any(|violation| violation.contains(wanted)),
            "a corpus with nothing in it produced no complaint about {wanted}: {found:#?}"
        );
    }

    // A build path that says the right words in a file whose step is there, and
    // a corpus whose texts are read but whose paragraphs are not.
    let with_notarization_step = corpus_texts()
        .into_iter()
        .map(|(path, text)| {
            if path == BUILD_SCRIPT {
                (
                    path,
                    format!("{text}\nxcrun notarytool submit \"$extracted_binary\" --wait\n"),
                )
            } else {
                (path, text)
            }
        })
        .collect::<Vec<_>>();
    let found = notarization_violations(&with_notarization_step);
    assert!(
        found
            .iter()
            .any(|violation| violation.contains("notarytool")),
        "a notarization step added to the build path was read as acceptable: {found:#?}"
    );

    let with_flat_claim = corpus_texts()
        .into_iter()
        .map(|(path, text)| {
            if path == MACOS_DOC {
                (
                    path,
                    format!("{text}\nmacOS downloads: the archive has been notarized.\n"),
                )
            } else {
                (path, text)
            }
        })
        .collect::<Vec<_>>();
    let found = notarization_violations(&with_flat_claim);
    assert!(
        found
            .iter()
            .any(|violation| violation.contains("has been notarized")),
        "an assertion that the archive was notarized was read as acceptable: {found:#?}"
    );
}

/// The one exemption is a finding, not a convenience, and it must keep being.
///
/// `docs/development/RELEASE_PROCESS.md` is exempt because it **lists** these
/// phrasings in order to forbid them. The exemption stops being legitimate the
/// moment that stops holding, so it is asserted here: an exemption that no
/// longer earns its place is an exemption nobody notices is hiding a rule.
#[test]
fn the_exemption_is_still_earning_its_place() {
    let authoritative = read(AUTHORITATIVE).to_lowercase();
    let named = phrase_families()
        .iter()
        .flat_map(|(_, phrases)| phrases.iter())
        .filter(|(phrase, _)| authoritative.contains(&phrase.to_lowercase()))
        .count();
    assert!(
        named >= 3,
        "only {named} of the forbidden phrasings appear in {AUTHORITATIVE}, so it is no longer the \
         file that lists them in order to forbid them — and the exemption from the phrase rules \
         should be deleted rather than left standing"
    );
    // And the count is not reached by accident: each family has to be named,
    // because a family nobody lists is a family a reader has no reason to know
    // the rule is about.
    for (family, phrases) in phrase_families() {
        assert!(
            phrases
                .iter()
                .any(|(phrase, _)| authoritative.contains(&phrase.to_lowercase())),
            "no phrasing from the family that would {family} appears in {AUTHORITATIVE}, so the \
             prohibition list has stopped covering it"
        );
    }
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
/// Same discipline as `BREAKS` in `crates/sure-testkit/tests/signing_status.rs`
/// and `crates/sure-testkit/tests/ci_workflow.rs`: the edits are taken from the
/// files' own bytes rather than written as fixtures, so each one proves the
/// checker reacts to *these* files; and a `replace` that found nothing is a
/// failure rather than a pass, because that is the way this kind of table rots
/// into decoration.
///
/// The phrase entries are the ones a reader would actually write, and the
/// generated test below covers the rest of the lists.
const BREAKS: &[Break] = &[
    Break {
        what: "the archive stops saying that nothing notarizes it",
        file: BUILD_SCRIPT,
        from: "SIGNATURE AND NOTHING NOTARIZES IT",
        to: "SIGNATURE",
        wanted: "NOTHING NOTARIZES IT",
    },
    Break {
        what: "the archive stops naming notarization as an external credential",
        file: BUILD_SCRIPT,
        from: "ID signing and notarization as an external credential this project does not",
        to: "ID signing as an external credential this project does not",
        wanted: "notarization as an external credential this project does not",
    },
    Break {
        what: "the Signature step stops saying why no notarization is read",
        file: BUILD_SCRIPT,
        from: "**Notarization is not read here at all**: nothing in this build notarizes",
        to: "**Notarization is not read here**: nothing in this build notarizes",
        wanted: "Notarization is not read here at all",
    },
    Break {
        what: "the Signature step's failure stops being keyed on a signing field",
        file: BUILD_SCRIPT,
        from: "if grep -q '^Authority=' \"$sign_log\"; then",
        to: "if false; then",
        wanted: "grep -q '^Authority='",
    },
    Break {
        what: "the one reading this project takes stops being taken",
        file: BUILD_SCRIPT,
        from: "run_captured codesign \"$sign_out\" \"$sign_err\" -d --verbose=2 \"$extracted_binary\"",
        to: ": # the signature is not read on this path",
        wanted: "-d --verbose=2",
    },
    Break {
        what: "the archive stops telling the reader not to turn Gatekeeper off",
        file: BUILD_SCRIPT,
        from: "Nothing in SURE asks you to turn\nGatekeeper off.",
        to: "Turning Gatekeeper off is up to you.",
        wanted: "Nothing in SURE asks you to turn Gatekeeper off",
    },
    Break {
        what: "a notarization step is added to the build",
        file: BUILD_SCRIPT,
        from: "step 'Signature'",
        to: "xcrun notarytool submit \"$extracted_binary\" --wait\nstep 'Signature'",
        wanted: "notarytool",
    },
    Break {
        what: "an Apple notarization credential is configured for the build",
        file: BUILD_SCRIPT,
        from: "step 'Signature'",
        to: "APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}\nstep 'Signature'",
        wanted: "APPLE_TEAM_ID",
    },
    Break {
        what: "a notarization step is added to the Windows packager",
        file: WINDOWS_PACKAGER,
        from: "$signature = Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop",
        to: "xcrun stapler validate $Path\n$signature = Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop",
        wanted: "stapler",
    },
    Break {
        what: "an Apple credential appears in a third workflow",
        file: CI_WORKFLOW,
        from: "name: ci",
        to: "name: ci\nenv:\n  APPLE_APP_SPECIFIC_PASSWORD: ${{ secrets.APPLE_APP_SPECIFIC_PASSWORD }}",
        wanted: "APPLE_APP_SPECIFIC_PASSWORD",
    },
    Break {
        what: "a notarization gate is added to the release workflow",
        file: RELEASE_WORKFLOW,
        from: "      - name: Create the draft release",
        to: "      - name: Notarize the macOS archives\n        run: xcrun spctl --assess --type execute target/tmp/release/sure\n      - name: Create the draft release",
        wanted: "spctl",
    },
    Break {
        what: "an Apple credential is passed into the release workflow",
        file: RELEASE_WORKFLOW,
        from: "env:\n          GH_TOKEN: ${{ github.token }}\n          TAG: ${{ inputs.tag }}\n          RUNNER_TEMP: ${{ runner.temp }}",
        to: "env:\n          GH_TOKEN: ${{ github.token }}\n          TAG: ${{ inputs.tag }}\n          RUNNER_TEMP: ${{ runner.temp }}\n          APPLE_ID: ${{ secrets.APPLE_ID }}",
        wanted: "APPLE_ID",
    },
    Break {
        what: "the authoritative section is retitled out of existence",
        file: AUTHORITATIVE,
        from: "### macOS: no Developer ID, no notarization and no staple",
        to: "### macOS",
        wanted: "macOS: no Developer ID, no notarization and no staple",
    },
    Break {
        what: "the first missing thing stops being named",
        file: AUTHORITATIVE,
        from: "**Developer ID Application certificate**",
        to: "**signing certificate**",
        wanted: "**Developer ID Application certificate**",
    },
    Break {
        what: "the second missing thing stops being named",
        file: AUTHORITATIVE,
        from: "**notarization credential**",
        to: "**notarization step**",
        wanted: "**notarization credential**",
    },
    Break {
        what: "the third missing thing stops being named",
        file: AUTHORITATIVE,
        from: "**Apple Developer Program membership**",
        to: "**Apple account**",
        wanted: "**Apple Developer Program membership**",
    },
    Break {
        what: "the statement stops saying who would have to provide what is missing",
        file: AUTHORITATIVE,
        from: "An Apple Developer Program member with an",
        to: "An Apple Developer account holder with an",
        wanted: "An Apple Developer Program member with an Apple account",
    },
    Break {
        what: "the consequence stops being stated",
        file: AUTHORITATIVE,
        from: "a person who downloads one meets Gatekeeper",
        to: "a person who downloads one gets a normal macOS experience",
        wanted: "a person who downloads one meets Gatekeeper",
    },
    Break {
        what: "the boundary on what this project asks of a reader is dropped",
        file: AUTHORITATIVE,
        from: "Nothing in this repository asks anyone to turn Gatekeeper",
        to: "Nothing in this repository prevents anyone from turning Gatekeeper",
        wanted: "Nothing in this repository asks anyone to turn Gatekeeper off",
    },
    Break {
        what: "a signing field is described as a notarization field",
        file: AUTHORITATIVE,
        from: "It is not a notarization field",
        to: "It is the field this section is about",
        wanted: "It is not a notarization field",
    },
    Break {
        what: "the notarization half stops saying it rests on there being no step",
        file: AUTHORITATIVE,
        from: "Notarization is not read by that\nstep at all",
        to: "Notarization is not measured by that step",
        wanted: "Notarization is not read by that step at all",
    },
    Break {
        what: "the stapling half is dropped",
        file: AUTHORITATIVE,
        from: "never notarized has no staple",
        to: "never notarized carries no ticket",
        wanted: "has no staple",
    },
    Break {
        what: "the boundary on what has been observed is dropped",
        file: AUTHORITATIVE,
        from: "either is unobserved here and stays unobserved",
        to: "either is documented by Apple",
        wanted: "unobserved here and stays unobserved",
    },
    Break {
        what: "the platform document stops saying the credentials are missing",
        file: MACOS_DOC,
        from: "No Apple Developer ID certificate and no notarization credential are configured",
        to: "Apple Developer ID signing and notarization are configured",
        wanted: "No Apple Developer ID certificate and no notarization credential are configured",
    },
    Break {
        what: "the platform document states the state without its consequence",
        file: MACOS_DOC,
        from: "a person who downloads one meets Gatekeeper",
        to: "a person who downloads one meets a normal macOS experience",
        wanted: "a person who downloads one meets Gatekeeper",
    },
    Break {
        what: "the platform document stops pointing at the authoritative section",
        file: MACOS_DOC,
        from: "no notarization and no staple",
        to: "the macOS signing situation",
        wanted: "no notarization and no staple",
    },
    Break {
        what: "the platform document stops saying Gatekeeper is unobserved",
        file: MACOS_DOC,
        from: "has observed what Gatekeeper does with either archive on a Mac",
        to: "has documented what Gatekeeper does with either archive on a Mac",
        wanted: "has observed what Gatekeeper does with either archive on a Mac",
    },
    Break {
        what: "the release notes stop saying nothing here is notarized",
        file: RELEASE_WORKFLOW,
        from: "Nothing here is signed or notarized.",
        to: "Nothing here is signed.",
        wanted: "Nothing here is signed or notarized",
    },
    Break {
        what: "the release notes stop naming the missing Apple credential",
        file: RELEASE_WORKFLOW,
        from: "Apple Developer ID is configured",
        to: "Apple certificate is configured",
        wanted: "no Apple Developer ID is configured",
    },
    Break {
        what: "the release notes stop naming what a macOS downloader meets",
        file: RELEASE_WORKFLOW,
        from: "a macOS download is subject to Gatekeeper",
        to: "a macOS download works as any other does",
        wanted: "a macOS download is subject to Gatekeeper",
    },
    Break {
        what: "the dry-run workflow stops saying what it does not do",
        file: DRY_RUN_WORKFLOW,
        from: "No signing, no notarization, no release, no tag and no",
        to: "No release, no tag and no",
        wanted: "No signing, no notarization",
    },
    Break {
        what: "the dry-run workflow stops naming the blocker",
        file: DRY_RUN_WORKFLOW,
        from: "Apple Developer ID credentials",
        to: "Apple credentials",
        wanted: "Apple Developer ID credentials are an external blocker",
    },
    Break {
        what: "the Intel job's record stops saying what it built",
        file: DRY_RUN_WORKFLOW,
        from: "the binary is unsigned and unnotarized",
        to: "the binary is unsigned",
        wanted: "the binary is unsigned and unnotarized",
    },
    Break {
        what: "a reader is told the archive was notarized",
        file: MACOS_DOC,
        from: "and nothing in it asks anyone to turn Gatekeeper off.",
        to: "and nothing in it asks anyone to turn Gatekeeper off. The archive has been notarized.",
        wanted: "has been notarized",
    },
    Break {
        what: "a reader is told a notarization ticket is attached",
        file: MACOS_DOC,
        from: "and nothing in it asks anyone to turn Gatekeeper off.",
        to: "and nothing in it asks anyone to turn Gatekeeper off. The ticket is attached.",
        wanted: "the ticket is attached",
    },
    Break {
        what: "a reader is promised they will not meet a prompt",
        file: MACOS_DOC,
        from: "and nothing in it asks anyone to turn Gatekeeper off.",
        to: "and nothing in it asks anyone to turn Gatekeeper off. It opens without a warning.",
        wanted: "opens without a warning",
    },
    Break {
        what: "the absence is presented as a neutral fact",
        file: MACOS_DOC,
        from: "and nothing in it asks anyone to turn Gatekeeper off.",
        to: "and nothing in it asks anyone to turn Gatekeeper off. Notarization does not affect anything.",
        wanted: "notarization does not affect anything",
    },
];

#[test]
fn every_rule_is_turned_red_by_an_edit_that_breaks_it() {
    let real = corpus_texts();
    assert!(
        notarization_violations(&real).is_empty(),
        "the files are already failing a rule, so no edit below can be said to have broken it:\n{:#?}",
        notarization_violations(&real)
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
        let found = notarization_violations(&edited);
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
/// `ci_workflow.rs`, and `every_forbidden_phrase_is_one_this_reader_reports` has
/// in `signing_status.rs`, for the same reason: a list of phrases is decoration
/// unless each entry is one the rule actually finds. Each phrase is injected
/// into a real file as a real sentence and the rules are required to name it,
/// so a typo in the list, or an entry the rule stopped reading, is a red test
/// rather than a line in a table that looks like a check.
#[test]
fn every_forbidden_phrase_is_one_this_reader_reports() {
    let real = corpus_texts();
    assert!(notarization_violations(&real).is_empty());

    for (family, phrases) in phrase_families() {
        for (phrase, why) in phrases {
            let mut edited = real.clone();
            for (path, text) in edited.iter_mut() {
                if *path == MACOS_DOC {
                    *text = format!("{text}\nmacOS downloads: {phrase}.\n");
                }
            }
            let found = notarization_violations(&edited);
            assert!(
                found.iter().any(|violation| violation.contains(phrase)),
                "injecting {phrase:?} ({why}) into {MACOS_DOC} produced no violation naming it, so \
                 the entry is a line in a list rather than a check, and it would {family}:\n{found:#?}"
            );
        }
    }

    // And the structural lists are held to the same standard: an entry the rule
    // stopped reading is a line in a table, not a check.
    for (token, why) in NOTARIZATION_TOOLS.iter().chain(APPLE_CREDENTIALS.iter()) {
        let mut edited = real.clone();
        for (path, text) in edited.iter_mut() {
            if *path == ASSEMBLER {
                *text = format!("{text}\n# {token}\n");
            }
        }
        let found = notarization_violations(&edited);
        assert!(
            found.iter().any(|violation| violation.contains(token)),
            "injecting {token:?} ({why}) into {ASSEMBLER} produced no violation naming it, so the \
             entry is a line in a list rather than a check:\n{found:#?}"
        );
    }

    // And a phrase from a family is only ever a violation in a file the phrase
    // rules read: the authoritative statement lists these phrasings in order to
    // forbid them, and a rule that fired there would make the list illegal to
    // write.
    let mut edited = real.clone();
    for (path, text) in edited.iter_mut() {
        if *path == AUTHORITATIVE {
            *text = format!("{text}\n\n### A note\n\nThe archive has been notarized.\n");
        }
    }
    let found = notarization_violations(&edited);
    assert!(
        found
            .iter()
            .all(|violation| !violation.starts_with(AUTHORITATIVE)),
        "the phrase rules fired on {AUTHORITATIVE}, which is the file that lists these phrasings in \
         order to forbid them: {found:#?}"
    );
}
