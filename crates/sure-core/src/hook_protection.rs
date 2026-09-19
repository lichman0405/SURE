//! Protection decision adapter for harness pre-action hooks.
//!
//! Maps harness tool requests to [`ExecutionDecision`] using the existing
//! domain machinery rather than inventing a new rule engine.
//!
//! P11-T006: Cursor protection where supported.
//!
//! # Where the protection mode's rule lives
//!
//! `docs/security/PROTECTION_MODE.md` gives the user three modes and one
//! difference between the first two: **strict** additionally asks before
//! migrations, CI/CD configuration, secret/config areas and broad filesystem
//! modifications. Until `P13-T004` the mode was parsed, validated and ranked
//! (`crate::config::ProtectionMode`, `Authority::protection`) and read by
//! nothing, so `standard` and `strict` were two names for one behaviour.
//!
//! The rule is **here**, beside the engine that already turns a request into an
//! action, and it is a second question asked of the same decision rather than a
//! second decision path: [`decide`] answers first with the execution mode and
//! permissions, and only an [`ExecutionDecision::Allowed`] is then put to the
//! mode. A refusal the domain reached is never diluted by a mode — strict adds
//! a question, it does not replace an answer.
//!
//! What the second question may look at is deliberately narrow, and since
//! `P13-T005` it is two things rather than one.
//!
//! What is classified is the path the normalised event carries — the argument
//! the harness itself says the tool is aimed at — and only for the three action
//! kinds that name one file. That rule is `P13-T004`'s and it is unchanged.
//!
//! Beside it, a **command line the harness claims** is read, for one purpose
//! and with one reader: [`crate::safety::read_simple_command`], which refuses
//! any text whose meaning is not the characters in it and answers `None` rather
//! than a guess. This module reads the harness's own `args.command` — the
//! string the harness says it is about to run — and puts it to
//! [`crate::safety::classify`], the one classifier, exactly as `P3-T004` puts a
//! program and an argument vector to it. Nothing here is a second table of
//! dangerous commands and nothing here decides anything: the reading can only
//! *name* a danger in a request that was already held, and a name never lets
//! one through. The one thing a name changes is that an allowance the user
//! recorded can be spent on it (see below).
//!
//! A command line SURE declines to read is not a gap to be filled: the request
//! keeps the answer it would have had before, which for a shell tool is a
//! refusal, and the sentence says nothing about what the command would do.
//!
//! # The three dangers, and the one-time allowance
//!
//! Three dangerous acts are named: a **broad delete**, a **force push** and a
//! **sensitive read** ([`Danger`]). Each is read off an answer the code below
//! already reaches — the classifier's own `Source::Rule` and `Destructive`
//! class for the first two, and strict mode's credential rule for the third —
//! so there is one answer to *what would this do* and the danger is a reading
//! of it rather than a rule beside it.
//!
//! A user who has been shown one of these can record, with
//! `sure hook allow-once`, that SURE would let **one** matching request
//! through, once. The record is the durable half: a hook is a fresh process per
//! event, so a one-time grant cannot live in memory, and
//! `crate::allowance` states what is stored and [`Store::spend_allowance`]
//! states how it is spent atomically.
//!
//! [`Store::spend_allowance`]: crate::store::Store::spend_allowance
//!
//! **This is a statement about what SURE would do.** Both integrations are
//! Observed (Tier 1), so nothing here establishes that a harness honours an
//! allowance any more than it establishes that a harness honours a block. The
//! sentence an allowance produces says *SURE would let this through*, and the
//! limit is the same one every other answer in this module has.
//!
//! Since `P13-T010` the writer reads the settings as well as the words, because
//! a grant is only worth recording where an act can be named at all: an act is
//! named for a request SURE *holds*, a hold depends on the execution and
//! protection settings in force, and a grant recorded under settings that hold
//! nothing is one no request can ever spend. [`acts_a_request_could_be_held_for`]
//! answers which acts are reachable and
//! [`allowance_could_not_be_spent_reason`] is the sentence the writer refuses
//! with when the answer is none. Both are questions put to [`assess_request`]
//! rather than a second rule beside it, so what a user is told cannot drift from
//! the rule that would answer their request. The *subject* is still unread at
//! write time, for the reason `PROTECTION_MODE.md` gives.
//!
//! # Capability tier honesty
//!
//! Both integrations are **Observed** (Tier 1) and neither can confirm the
//! harness honours the answer, so what SURE has is a decision about what it
//! would do, not a record of what the harness did. That is why a reason says
//! *SURE does not allow it* rather than *it did not happen*: nothing in this
//! build can establish the second sentence.

use serde::{Deserialize, Serialize};
use sure_domain::execution::{
    ActionKind, CommandClass, ExecutionDecision, ExecutionMode, ExecutionPermissions, decide,
};
use sure_domain::variants::variants;

use crate::config::{CUSTOM_PROTECTION_EXPLANATION, CUSTOM_PROTECTION_INSTEAD, ProtectionMode};
use crate::safety::{self, Source};

/// What the protection adapter decided about a tool request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionDecisionKind {
    /// The tool may proceed.
    Allow,
    /// The tool may proceed, but SURE advises caution.
    Warn,
    /// The tool should not proceed.
    Block,
}

impl ProtectionDecisionKind {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Block => "block",
        }
    }
}

/// A protection decision with an optional human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionDecision {
    /// The decision itself.
    pub decision: ProtectionDecisionKind,
    /// Why the decision was made, when it is not obvious.
    pub reason: Option<String>,
}

impl ProtectionDecision {
    /// Allow with nothing to explain.
    ///
    /// `reason: None` on an allow means one thing in this module: **no request
    /// was put to the rule at all** — a lifecycle event that is not a tool
    /// request. Every decision the rule itself reaches carries a sentence,
    /// allows included ([`ProtectionDecision::allow_with`]), so a caller that
    /// sees `None` here is looking at an event rather than at a verdict.
    #[must_use]
    pub fn allow() -> Self {
        Self {
            decision: ProtectionDecisionKind::Allow,
            reason: None,
        }
    }

    /// Allow, and say what the mode looked at before letting it through.
    ///
    /// The criterion this module is held to is that each mode returns an action
    /// **and** a plain-language reason, and an allow is an action: a user who
    /// reads "allow" and nothing else cannot tell a request SURE examined and
    /// found nothing in from one it never saw.
    #[must_use]
    pub fn allow_with(reason: impl Into<String>) -> Self {
        Self {
            decision: ProtectionDecisionKind::Allow,
            reason: Some(reason.into()),
        }
    }

    /// Warn with a reason.
    #[must_use]
    pub fn warn(reason: impl Into<String>) -> Self {
        Self {
            decision: ProtectionDecisionKind::Warn,
            reason: Some(reason.into()),
        }
    }

    /// Block with a reason.
    #[must_use]
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            decision: ProtectionDecisionKind::Block,
            reason: Some(reason.into()),
        }
    }
}

/// What a harness says it is about to do.
///
/// Every field is the request's own words: the tool name exactly as it was
/// sent, the path the normalised event carries when it carries one, and the
/// command line the harness claims it is about to run when it is a shell tool.
/// They are **data** — another program's claim about itself — so nothing here
/// treats any of them as evidence that the tool would do what it says, and no
/// field of the request is echoed back inside a sentence SURE writes.
///
/// A request carries a path or a command, and which one it carries is the
/// harness's own vocabulary: `Read` and `Write` name files, `Bash` and `Shell`
/// carry a command line, and neither vocabulary has both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRequest<'a> {
    /// The tool name, exactly as the harness sent it.
    pub tool: &'a str,
    /// The path the normalised event names, when it names one.
    pub path: Option<&'a str>,
    /// The command line the harness claims, when it claims one.
    ///
    /// Read only through [`crate::safety::read_simple_command`], only to name a
    /// danger, and never echoed.
    pub command: Option<&'a str>,
}

impl<'a> ToolRequest<'a> {
    /// A request for a tool that names no path and no command.
    #[must_use]
    pub const fn of(tool: &'a str) -> Self {
        Self {
            tool,
            path: None,
            command: None,
        }
    }

    /// The same request, with the path the harness named.
    #[must_use]
    pub const fn at(tool: &'a str, path: &'a str) -> Self {
        Self {
            tool,
            path: Some(path),
            command: None,
        }
    }

    /// A request for a tool that claims a command line.
    #[must_use]
    pub const fn running(tool: &'a str, command: &'a str) -> Self {
        Self {
            tool,
            path: None,
            command: Some(command),
        }
    }

    /// The same request, with the path the harness named, when it named one.
    ///
    /// A builder rather than a fourth argument to [`ToolRequest::of`], because
    /// a payload SURE could not read a field out of and a payload that did not
    /// carry the field are the same request: nothing was named.
    #[must_use]
    pub const fn and_path(self, path: Option<&'a str>) -> Self {
        Self { path, ..self }
    }

    /// The same request, with the command line the harness claimed, if it
    /// claimed one.
    #[must_use]
    pub const fn and_command(self, command: Option<&'a str>) -> Self {
        Self { command, ..self }
    }

    /// What the request is aimed at, in the harness's own words.
    ///
    /// One string rather than two, and that is what a one-time allowance is
    /// recorded against: the exact command line for a shell tool, the exact
    /// path for a tool that names one. It is `None` only for a request that
    /// names neither, which is a request no allowance can cover.
    #[must_use]
    pub fn subject(&self) -> Option<&'a str> {
        self.command.or(self.path)
    }
}

/// Map a Cursor tool name to the [`ActionKind`] the domain understands.
fn cursor_tool_to_action_kind(tool: &str) -> ActionKind {
    match tool {
        "Shell" => ActionKind::ArbitraryCommand,
        "Read" => ActionKind::ReadFile,
        "Write" => ActionKind::WriteProjectFile,
        "Delete" => ActionKind::DeleteProjectFile,
        _ => ActionKind::ArbitraryCommand,
    }
}

/// Decide whether a Cursor `preToolUse` request should be allowed.
///
/// Uses the existing domain [`decide`] function with the current execution
/// mode and permissions, and then asks the protection mode's question. No new
/// rule engine is invented.
///
/// This is [`assess_cursor_tool`] without the danger, for a caller that wants
/// the answer alone.
///
/// # Capability tier honesty
///
/// Cursor remains **Observed** (Tier 1). The hook manifest does not confirm
/// Cursor interprets the response, so protection is advisory/warn-only from
/// the integration's point of view. The decision still uses the real mode and
/// permissions so that the answer is truthful about what SURE would do.
#[must_use]
pub fn decide_cursor_tool(
    request: &ToolRequest<'_>,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> ProtectionDecision {
    assess_cursor_tool(request, mode, permissions, protection).decision
}

/// [`decide_cursor_tool`], and which danger it named on the way.
#[must_use]
pub fn assess_cursor_tool(
    request: &ToolRequest<'_>,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> Assessment {
    assess_request(
        cursor_tool_to_action_kind(request.tool),
        request,
        mode,
        permissions,
        protection,
    )
}

/// Map a Claude Code tool name to the [`ActionKind`] the domain understands.
fn claude_code_tool_to_action_kind(tool: &str) -> ActionKind {
    match tool {
        "Bash" => ActionKind::ArbitraryCommand,
        "Read" => ActionKind::ReadFile,
        "Write" | "Edit" => ActionKind::WriteProjectFile,
        "Delete" => ActionKind::DeleteProjectFile,
        _ => ActionKind::ArbitraryCommand,
    }
}

/// Decide whether a Claude Code `PreToolUse` request should be allowed.
///
/// Uses the existing domain [`decide`] function with the current execution mode
/// and permissions, and then asks the protection mode's question. No new rule
/// engine is invented.
///
/// This is [`assess_claude_code_tool`] without the danger, for a caller that
/// wants the answer alone.
///
/// # Capability tier honesty
///
/// Claude Code remains **Observed** (Tier 1). The hook manifest wires
/// `PreToolUse`, but it does not confirm that Claude Code interprets or enforces
/// the response, so the integration cannot honestly claim Protected (Tier 2).
/// The decision still uses the real mode and permissions so that the answer is
/// truthful about what SURE would do.
#[must_use]
pub fn decide_claude_code_tool(
    request: &ToolRequest<'_>,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> ProtectionDecision {
    assess_claude_code_tool(request, mode, permissions, protection).decision
}

/// [`decide_claude_code_tool`], and which danger it named on the way.
#[must_use]
pub fn assess_claude_code_tool(
    request: &ToolRequest<'_>,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> Assessment {
    assess_request(
        claude_code_tool_to_action_kind(request.tool),
        request,
        mode,
        permissions,
        protection,
    )
}

/// What the rule answered, and which of the three dangers it named on the way.
///
/// The danger is carried beside the decision rather than inside it because
/// [`ProtectionDecision`] is what a harness reads: a field added there would be
/// a change to the integration's response made for a caller's benefit, and this
/// module's answers are a contract with two harnesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// What SURE would do about the request.
    pub decision: ProtectionDecision,
    /// Which danger was named, when one was.
    ///
    /// `Some` only where [`decision`](Self::decision) is a block, and a block
    /// with `None` is a request held for a reason no allowance may be spent on:
    /// a mode that does not permit it, the one mode this release refuses, a
    /// whole-location change strict holds, a migration, CI configuration. The
    /// danger is *the* list of what a recorded allowance can let through, so
    /// this field is also that list.
    pub danger: Option<Danger>,
}

/// The one decision path both integrations share.
///
/// The order is the whole of the design: the existing engine answers, a refusal
/// it reached is returned unchanged, and only an allowed action is put to the
/// protection mode. A mode can therefore make an answer firmer and can never
/// make one weaker — the same rule `Authority::protection` applies to the two
/// configuration layers.
///
/// `P13-T005` adds one thing to that order and changes none of it: the danger a
/// request was held for is *recorded* beside the answer. Recording is the whole
/// of it — the answer itself is the one this function would have given before,
/// except that a shell request held for a named danger carries a sentence about
/// what the command would do instead of the generic one about consent, which is
/// more information and not a weaker refusal.
fn assess_request(
    action_kind: ActionKind,
    request: &ToolRequest<'_>,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> Assessment {
    let base = decide(action_kind, mode, permissions);

    // The mode this release does not implement is answered first and alone.
    // Refusing is the direction that fails closed, and it is the same answer
    // the configuration reader gives the same value in a file: a mode SURE
    // cannot apply must not be quietly answered as though it were standard.
    if protection == ProtectionMode::Custom {
        return Assessment {
            decision: ProtectionDecision::block(custom_reason()),
            danger: None,
        };
    }

    if !base.is_allowed() {
        // The danger is read here too, and it is read off a refusal rather than
        // off an allow because that is where a shell request always lands: the
        // domain holds every arbitrary command for consent, and a hook cannot
        // ask for consent.
        //
        // It is read for `NeedsConsent` and for nothing else. `Denied` is the
        // execution mode or the permissions refusing, and a one-time allowance
        // is not a way to run under settings the user did not change.
        let danger = match base {
            ExecutionDecision::NeedsConsent => claimed_danger(action_kind, request),
            ExecutionDecision::Allowed | ExecutionDecision::Denied => None,
        };
        let decision = match danger {
            Some(danger) => ProtectionDecision::block(danger_reason(danger)),
            None => base_decision(base),
        };
        return Assessment { decision, danger };
    }

    match protection {
        // `Custom` returned above; the arm is here because a value this release
        // cannot produce from a file is still a value this function is total
        // over, and `unreachable!` in a decision path would be a panic where an
        // answer was asked for.
        ProtectionMode::Custom => Assessment {
            decision: ProtectionDecision::block(custom_reason()),
            danger: None,
        },
        ProtectionMode::Standard => Assessment {
            decision: ProtectionDecision::allow_with(STANDARD_ALLOWS),
            danger: None,
        },
        ProtectionMode::Strict => match sensitive_area(action_kind, request.path) {
            Some((area, role)) => Assessment {
                // The sentence is `P13-T004`'s and is not rewritten here: it
                // already names the consequence for both of the categories a
                // danger is read from, and two sentences for one hold would be
                // two chances to disagree.
                decision: ProtectionDecision::block(strict_reason(area, role)),
                danger: danger_of(area, role),
            },
            None => Assessment {
                decision: ProtectionDecision::allow_with(STRICT_ALLOWS),
                danger: None,
            },
        },
    }
}

/// A dangerous act SURE can name.
///
/// The three `P13-T005` is about, and the list is deliberately closed: a danger
/// is the *only* thing a recorded one-time allowance can let through, so a
/// fourth reader here would widen the override as well as the vocabulary.
///
/// Nothing here is a rule of its own. Each danger is read off an answer the
/// code already reaches — [`safety::classify`]'s own `Source::Rule` and its
/// `Destructive` class for the first two, strict mode's credential rule for the
/// third — and a danger is only ever attached to a request that was held.
///
/// The serde name is the one a stored decision records
/// ([`crate::protection_history`]), and it is not [`Danger::as_str`]: that is
/// the sentence a person reads, and a record keeps the name that does not change
/// when the sentence is reworded. There is one name and one place it is defined,
/// and `every_danger_has_one_stored_name` holds the two spellings together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Danger {
    /// The request would delete across a whole location rather than the files
    /// it names: a command the classifier calls destructive whose operands name
    /// a location, or a change strict holds for the same reason.
    BroadDelete,
    /// The request would replace commits that were already published.
    ForcePush,
    /// The request would read a file where credentials or keys live.
    SensitiveRead,
}

variants!(
    /// Every danger a one-time allowance can be spent on.
    Danger {
        BroadDelete,
        ForcePush,
        SensitiveRead
    }
);

impl Danger {
    /// The danger in the words a user reads.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BroadDelete => "delete a whole location",
            Self::ForcePush => "overwrite published commits",
            Self::SensitiveRead => "read a file of credentials",
        }
    }

    /// The name a stored decision gives this danger.
    ///
    /// [`Danger::as_str`] is a sentence a person reads and may be reworded; this
    /// is the value written into a record and read back by a script, so it is
    /// the one that must not change. The two are not two vocabularies: the
    /// serde name is this same enum's, declared by the attribute on it, and
    /// `every_danger_has_one_stored_name` is what keeps this accessor and that
    /// attribute the same string.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::BroadDelete => "broad_delete",
            Self::ForcePush => "force_push",
            Self::SensitiveRead => "sensitive_read",
        }
    }

    /// What this danger means for the user's own work.
    ///
    /// Two of the three are sentences that already exist: a whole-location
    /// change and a read of credentials are categories
    /// `docs/security/PROTECTION_MODE.md` names for strict mode, and the
    /// sentence a user reads for them is [`SensitiveArea::consequence`]'s. Only
    /// the force push needs words of its own, because nothing before this task
    /// said anything about one.
    #[must_use]
    pub const fn consequence(self) -> &'static str {
        match self {
            Self::BroadDelete => SensitiveArea::BroadFilesystemChange.consequence(PathRole::Change),
            Self::ForcePush => FORCE_PUSH_CONSEQUENCE,
            Self::SensitiveRead => SensitiveArea::SecretMaterial.consequence(PathRole::Read),
        }
    }

    /// A list of acts as one sentence names them: the words of
    /// [`Danger::as_str`], comma-separated with `or` before the last.
    ///
    /// One place lists the acts for every sentence that lists them, so that two
    /// sentences a user meets — the confirmation `sure hook allow-once` writes
    /// and the refusal it writes instead — cannot disagree about what an
    /// allowance covers. Every act reads as a clause of *because it would …*,
    /// which is the frame both sentences put them in, and that is why the list
    /// of three reads as one sentence rather than three.
    ///
    /// An empty list is an empty string. No sentence here asks for one: a
    /// sentence naming no act would be a sentence about nothing, and the two
    /// callers both have a non-empty list by construction.
    #[must_use]
    pub fn in_a_sentence(acts: &[Self]) -> String {
        let mut sentence = String::new();
        for (index, act) in acts.iter().enumerate() {
            if index > 0 {
                sentence.push_str(match (index + 1, acts.len()) {
                    // The comma is Oxford's and it is there for three or more:
                    // "a or b" needs none, and "a, b, or c" is the list this
                    // module's own sentences already write.
                    (last, len) if last == len && len > 2 => ", or ",
                    (last, len) if last == len => " or ",
                    _ => ", ",
                });
            }
            sentence.push_str(act.as_str());
        }
        sentence
    }
}

/// What a force push costs, in the user's words.
const FORCE_PUSH_CONSEQUENCE: &str = "This would replace commits that were already published. \
     Anything built on them — someone else's branch, a release, a review — is left pointing at \
     work that is no longer there.";

/// The half of a named-danger block that is the same for all three.
///
/// It says what SURE answers rather than what the harness did. Both integrations
/// are Observed (Tier 1), so a sentence claiming the command was stopped would
/// be a false green.
const DANGER_TAIL: &str = " A harness hook cannot ask you about this, so SURE does not allow it.";

/// The half of the sentence a spent allowance produces that is the same for all
/// three.
///
/// The words are *SURE would* for the same reason [`DANGER_TAIL`]'s are: an
/// allowance is a decision SURE reaches, not a fact about the harness, and
/// nothing in this build can establish that the harness acted on it.
const ALLOWANCE_TAIL: &str = " You recorded a one-time allowance for this exact request, and \
     this use spends it, so SURE would let this one through. No further request with these words \
     would be allowed.";

/// The sentence a request held for a named danger is answered with.
#[must_use]
pub fn danger_reason(danger: Danger) -> String {
    format!("{}{DANGER_TAIL}", danger.consequence())
}

/// The sentence a request allowed by a spent allowance is answered with.
#[must_use]
pub fn allowance_reason(danger: Danger) -> String {
    format!("{}{ALLOWANCE_TAIL}", danger.consequence())
}

/// The sentence a held request is answered with when SURE could not read its own
/// record of allowances.
///
/// A state of its own, and a sentence of its own, because the user may hold a
/// grant for exactly this request and SURE does not know whether they do. The
/// generic hold says nothing about it and the spent-allowance sentence would
/// claim a use that was never written; this says what happened to the read and
/// what the answer still is.
const ALLOWANCE_UNREADABLE: &str = " SURE could not read its record of one-time allowances, so \
     it has not spent one and does not allow this.";

/// [`ALLOWANCE_UNREADABLE`], for the danger the request was held for.
///
/// The danger is not named in the sentence — the block's own reason already
/// says what the command would do — so the parameter is here to keep the two
/// sentences answering the same request rather than to be printed.
#[must_use]
pub fn allowance_unreadable_reason(danger: Danger) -> String {
    format!("{}{ALLOWANCE_UNREADABLE}", danger.consequence())
}

/// Which danger a strict-mode category is, when it is one of the three.
///
/// A whole-location change is a broad delete and a read of credentials is a
/// sensitive read; the migration, CI and configuration categories are held by
/// strict mode and are **not** dangers, so no allowance can let them through.
/// That is a decision rather than an omission: what a one-time allowance covers
/// is the three acts `THREAT_MODEL.md` calls dangerous, and widening it to
/// every category strict holds would make the allowance a way to run under a
/// mode the user did not change.
fn danger_of(area: SensitiveArea, role: PathRole) -> Option<Danger> {
    match (area, role) {
        (SensitiveArea::BroadFilesystemChange, _) => Some(Danger::BroadDelete),
        (SensitiveArea::SecretMaterial, PathRole::Read) => Some(Danger::SensitiveRead),
        _ => None,
    }
}

/// The danger a claimed command line would do, if SURE can read it and its own
/// classifier calls it destructive.
///
/// Two questions, both answered by code that already exists: what the words are
/// ([`safety::read_simple_command`], which refuses anything with two readings)
/// and what the command does ([`safety::classify`], the one classifier).
fn claimed_danger(action_kind: ActionKind, request: &ToolRequest<'_>) -> Option<Danger> {
    match action_kind {
        // The one kind whose request carries a command line.
        ActionKind::ArbitraryCommand => request.command.and_then(command_danger),
        // None of these carries a command through the harness tool classifiers:
        // a path tool names a path, and the rest name nothing. A kind added to
        // the vocabulary has to be answered here rather than falling through a
        // wildcard, so each is named.
        ActionKind::ReadFile
        | ActionKind::WriteProjectFile
        | ActionKind::DeleteProjectFile
        | ActionKind::ListDirectory
        | ActionKind::ReadMetadata
        | ActionKind::StaticAnalysis
        | ActionKind::RunTests
        | ActionKind::Build
        | ActionKind::TypeCheck
        | ActionKind::Lint
        | ActionKind::StartService
        | ActionKind::LocalProbe
        | ActionKind::BrowserProbe
        | ActionKind::BrowserObservation
        | ActionKind::InstallDependencies
        | ActionKind::NetworkAccess
        | ActionKind::ExternalService => None,
    }
}

/// What one command line would do, read from the classifier's own answer.
///
/// `None` covers every way of not knowing: text with more than one reading, a
/// program the table does not have, an operation it does not have, a command
/// that is not destructive, and a destructive command whose operands name files
/// rather than a location. None of them is a gap — a shell request is held
/// whatever this answers, and the only thing the answer changes is whether the
/// user can spend a recorded allowance on it.
fn command_danger(command: &str) -> Option<Danger> {
    let words = safety::read_simple_command(command)?;
    let (program, arguments) = words.split_first()?;
    let classification = safety::classify(program, arguments);

    // Only a row of the classifier's own table can name a danger. A program it
    // does not know, an operation it does not know, and text it cannot read are
    // all answered with every category, `Destructive` among them, and naming a
    // danger off one of those would be reporting SURE's own failure to classify
    // as a fact about the command.
    let Source::Rule {
        program,
        decided_by,
    } = classification.source()
    else {
        return None;
    };
    if !classification
        .effects()
        .classes()
        .contains(&CommandClass::Destructive)
    {
        return None;
    }

    // A push that destroys is a push with a force flag: the table's `push` row
    // reaches `Destructive` through no other form, so the class is the force
    // flag rather than a second reading of the arguments.
    if program == "git" && decided_by == Some("push") {
        return Some(Danger::ForcePush);
    }

    // Breadth, and it is the same question `strict` asks of a path: one operand
    // that names a location rather than a file. An operand-less destructive
    // command is deliberately not a broad delete — `git clean -fdx` and
    // `git reset --hard` are destructive without naming anything SURE can
    // bound, and naming them would put a sentence about "a whole location" on a
    // command whose location SURE has not established.
    arguments
        .iter()
        .filter(|word| !word.starts_with('-'))
        .any(|word| word_names_a_whole_location(word))
        .then_some(Danger::BroadDelete)
}

/// Whether one operand of a command line names a location rather than a file.
///
/// The folding and the question are `sensitive_area`'s, applied to a word
/// instead of to a path: a backslash is a separator whatever the platform, case
/// is folded on every platform, and [`names_a_whole_location`] decides.
fn word_names_a_whole_location(word: &str) -> bool {
    let folded = folded(word);
    let folded = folded.as_str();
    names_a_whole_location(folded, fold_segments(folded).count())
}

/// One request per way [`assess_request`] can reach a [`Danger`].
///
/// These are not examples, and the list is the whole of what a danger can be
/// read from: a destructive command whose operand names a location, a push that
/// replaces published commits, and strict mode's three path rules — a read of
/// secret material, a change that names a whole location, and a change that
/// names no path at all, which `sensitive_area` answers as the same
/// whole-location change. Nothing else in [`assess_request`] can produce a
/// danger, so the acts that some witness produces *are* the acts a request from
/// this project could be held for.
///
/// The tool names are the harnesses' vocabulary, but no classifier reads them
/// here: [`assess_request`] is handed the [`ActionKind`] directly, which is what
/// the tool maps at the top of this module produce, so a witness cannot be
/// right or wrong about an integration. `every_witness_names_its_danger` holds
/// this list to the rule — it fails if a witness stops producing the danger it
/// exists for, which is the only way the list can go stale.
const DANGER_WITNESSES: [(ActionKind, ToolRequest<'static>); 5] = [
    (
        ActionKind::ArbitraryCommand,
        ToolRequest::running("Bash", "rm -rf build/"),
    ),
    (
        ActionKind::ArbitraryCommand,
        ToolRequest::running("Bash", "git push --force"),
    ),
    (ActionKind::ReadFile, ToolRequest::at("Read", ".env")),
    (
        ActionKind::WriteProjectFile,
        ToolRequest::at("Write", "build/"),
    ),
    (ActionKind::WriteProjectFile, ToolRequest::of("Write")),
];

/// Which of the three acts a request from this project could be held for, under
/// one set of settings.
///
/// Empty means no allowance recorded under these settings can ever be spent:
/// the three acts are the *only* thing an allowance lets through, an act is
/// named only for a request SURE holds, and this answers whether any hold of
/// that kind is reachable at all. `P13-T010` is the caller that has to say so to
/// a user, and
/// [`allowance_could_not_be_spent_reason`] is the sentence it says it with.
///
/// The answer is read off [`assess_request`] through [`DANGER_WITNESSES`] rather
/// than computed from the settings by a second rule, which is the point: a rule
/// stated twice is a rule that can be stated two ways, and this one decides
/// whether SURE tells a user their grant is worth recording.
#[must_use]
pub fn acts_a_request_could_be_held_for(
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> Vec<Danger> {
    let mut acts: Vec<Danger> = Vec::new();
    for (action_kind, request) in DANGER_WITNESSES {
        let Some(danger) =
            assess_request(action_kind, &request, mode, permissions, protection).danger
        else {
            continue;
        };
        // In witness order, which is the order `Danger::ALL` declares, and
        // once each: two witnesses name a broad delete and a sentence must not
        // say so twice.
        if !acts.contains(&danger) {
            acts.push(danger);
        }
    }
    acts
}

/// Why no allowance recorded under these settings could ever be spent, in the
/// user's own words, when that is so.
///
/// `None` is the answer that matters: at least one of the three acts is
/// reachable here, so a matching request can be held for it and spend the grant.
/// `Some` is the sentence `sure hook allow-once` refuses with, and it names the
/// setting that would have to change — `docs/security/PROTECTION_MODE.md` is
/// careful to keep the setting *out* of the sentence a held request is answered
/// with, because the user's next act there is allow or deny; here the user's
/// next act is to change a setting, and a refusal that named the consequence
/// and not the setting would leave them nothing to do with it.
///
/// Both recommendations are *asked of the same rule* rather than asserted —
/// `host_confirmed` with the permission it needs, and `strict` with the settings
/// unchanged — so the sentence can only offer a change that would work. A
/// sentence telling a user to change the wrong setting is the failure this
/// function is here to avoid.
///
/// **Settings only, and the limit is deliberate.** Whether the *subject* the
/// user named is a request SURE would hold is not knowable here: there is no
/// request to read when a grant is written, and reading the words for a danger
/// would be a second rule about what is dangerous beside this module's. So a
/// grant recorded in a project where some act is reachable may still be spent by
/// nothing, and `docs/security/PROTECTION_MODE.md` says so.
#[must_use]
pub fn allowance_could_not_be_spent_reason(
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
    protection: ProtectionMode,
) -> Option<String> {
    if !acts_a_request_could_be_held_for(mode, permissions, protection).is_empty() {
        return None;
    }

    let opening = format!(
        "A one-time allowance is spent by a request SURE holds because it would {}. Under the \
         settings in force for this project no request can be held for any of those acts, so a \
         grant would sit in SURE's store and be spent by nothing.",
        Danger::in_a_sentence(Danger::ALL)
    );

    // The mode this release cannot apply is answered on its own, because it is
    // the one value where nothing else about the settings is the cause: the
    // reason is the value itself. The arm is here because a value this release
    // cannot produce from a file is still a value this function is total over;
    // `crate::config` refuses `custom` in a file and `hook.rs`'s reader falls
    // back to `strict`, so no user reaches this sentence today.
    if protection == ProtectionMode::Custom {
        return Some(format!(
            "{opening} `protection.mode` is `custom`, and this release cannot apply that mode. \
             {CUSTOM_PROTECTION_EXPLANATION} {CUSTOM_PROTECTION_INSTEAD}"
        ));
    }

    // The two settings that would make an act reachable, each verified against
    // the rule before it is offered. The first is the user's own file naming a
    // mode that runs project code, which is what grants `RunProjectCode`; the
    // second is the stricter protection mode, which holds a read of secret
    // material, and the permission an inspection already has is what lets that
    // hold be reached (`Authority::permissions` starts from
    // `ExecutionPermissions::inspect_only`).
    let with_project_code = {
        let mut permissions = permissions.clone();
        permissions.run_project_code = true;
        acts_a_request_could_be_held_for(ExecutionMode::HostConfirmed, &permissions, protection)
    };
    let with_strict = acts_a_request_could_be_held_for(mode, permissions, ProtectionMode::Strict);

    let mut reason = opening;

    // What is in force, and only it: each clause is here because it is a reason
    // nothing is held, and a clause about a setting that is not blocking would
    // be a sentence telling a user to change the wrong thing.
    if !with_project_code.is_empty() {
        reason.push_str(&format!(
            " `execution.mode` is `{}` here, so SURE does not run this project's own code, and a \
             command is refused on that ground before SURE asks what it would do.",
            mode.as_str()
        ));
    }
    if with_strict.is_empty() {
        // Strict names a danger for a read of secret material and for a change
        // that names a whole location, so a run under which neither is reachable
        // is one whose permissions do not let SURE read or change a file. That
        // is a set `Authority::permissions` cannot build — it starts from
        // `inspect_only` — and the clause is here because this function is total
        // over the values it takes, not because a user can meet it.
        reason.push_str(
            " The permissions in force do not let SURE read or change this project's own files, \
             so not even the question strict protection asks about a read of secret material is \
             reached.",
        );
    } else {
        reason.push_str(&format!(
            " `protection.mode` is `{}`, which asks no further question of its own about a read \
             or a change.",
            protection.as_str()
        ));
    }

    // What would make one reachable, and nothing else. The user's own settings
    // file is named as the file rather than as "your settings", because a
    // project's `sure.yaml` cannot do this and a user who edited that one would
    // have changed nothing (`P13-T009`).
    if !with_project_code.is_empty() {
        reason.push_str(&format!(
            " Name `execution.mode: host_confirmed` in your own settings file — the one \
             `sure doctor` prints, not the project's `sure.yaml` — which lets SURE run this \
             project's own code on this machine; a request SURE holds because it would {} would \
             then be within reach of an allowance.",
            Danger::in_a_sentence(&with_project_code)
        ));
    }
    if !with_strict.is_empty() {
        reason.push_str(&format!(
            " Set `protection.mode: strict`, and a request SURE holds because it would {} is \
             within reach of one.",
            Danger::in_a_sentence(&with_strict)
        ));
    }

    reason.push_str(
        " Nothing was written, and SURE records no allowance that a request here could not spend.",
    );
    Some(reason)
}

/// The sentence SURE answers with when the mode in force is the one this
/// release does not implement.
///
/// The two halves are the sentences the configuration reader already refuses
/// `protection.mode: custom` with, quoted from there rather than written again:
/// one value, one explanation, in both places a user can meet it.
fn custom_reason() -> String {
    format!(
        "This protection mode cannot be applied, so SURE does not allow the action. \
         {CUSTOM_PROTECTION_EXPLANATION} {CUSTOM_PROTECTION_INSTEAD}"
    )
}

/// What standard allows, in the user's words.
///
/// The sentence cannot say what standard looked *for*, because standard is the
/// mode that adds no question of its own. What it can say is that nothing in the
/// settings in force stopped the request and that this mode asked nothing
/// further. Saying the request "is not a secret" would be false for the read
/// that is one and that standard lets through, and that read is exactly the
/// difference between the two modes, so this sentence must not imply a check
/// that did not happen.
const STANDARD_ALLOWS: &str = "The settings in force do not stop this request, and standard \
     protection asks no further question about it, so it proceeds.";

/// The same for strict.
///
/// It names the four areas, because the useful half of an allow is what SURE
/// looked for and did not find.
const STRICT_ALLOWS: &str = "Strict protection does not hold this: it is not a database \
     migration, CI/CD configuration, credentials or keys, SURE's settings for this project, or a \
     change that names a whole location, and the permissions in force do not stop it.";

/// The half of a strict-mode block that is the same for every area.
///
/// It says what the mode does — *asks you about this* — rather than what the
/// harness did. Both integrations are Observed (Tier 1), so what SURE has is an
/// answer, and a sentence claiming the action was prevented would be a false
/// green.
const STRICT_TAIL: &str = " Strict protection asks you about this, and a harness hook cannot ask \
     you, so SURE does not allow it.";

/// Why the existing engine refused, unchanged.
///
/// Neither sentence mentions the protection mode, and that is deliberate: the
/// execution mode and the permissions are what stopped the action, and naming
/// the protection setting would tell a user to change the wrong thing.
fn base_decision(decision: ExecutionDecision) -> ProtectionDecision {
    match decision {
        ExecutionDecision::Allowed => ProtectionDecision::allow(),
        ExecutionDecision::NeedsConsent => ProtectionDecision::block(
            "This action needs explicit approval; the hook cannot obtain consent, so it is blocked.",
        ),
        ExecutionDecision::Denied => {
            ProtectionDecision::block("The current execution mode does not permit this action.")
        }
    }
}

/// The sentence a strict-mode block gives a user.
fn strict_reason(area: SensitiveArea, role: PathRole) -> String {
    format!("{}{STRICT_TAIL}", area.consequence(role))
}

/// A category `docs/security/PROTECTION_MODE.md` names for strict mode.
///
/// The document names four — migrations, CI/CD configuration, secret/config
/// areas and broad filesystem modifications — and this is those four with the
/// third split in two, because they are not the same question for a read:
/// opening a file of credentials is what exposes it, while reading a project's
/// configuration is ordinary work. The read/strict question is therefore asked
/// about [`SensitiveArea::SecretMaterial`] and about no other category, and the
/// split is what lets that be said rather than guessed.
///
/// Declared with [`variants!`] so that every category can be visited: a category
/// added without a sentence would otherwise reach a user as an empty reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveArea {
    /// A database migration.
    DatabaseMigration,
    /// CI/CD configuration.
    CiConfiguration,
    /// Credentials and key material.
    SecretMaterial,
    /// SURE's own settings for the project.
    ConfigurationArea,
    /// A change that names a whole location rather than one file.
    BroadFilesystemChange,
}

variants!(
    /// Every category `PROTECTION_MODE.md` names for strict mode.
    SensitiveArea {
        DatabaseMigration,
        CiConfiguration,
        SecretMaterial,
        ConfigurationArea,
        BroadFilesystemChange
    }
);

impl SensitiveArea {
    /// What this category means for the user's own work, in the user's words.
    ///
    /// Consequence first, and the name of the setting nowhere: the document
    /// asks for "consequence, not policy jargon", so none of these says which
    /// mode was in force or which rule fired.
    ///
    /// The match is over the pair, not over the category alone, so a category
    /// added later has to answer for both a read and a change rather than
    /// inheriting one of the two sentences.
    #[must_use]
    pub const fn consequence(self, role: PathRole) -> &'static str {
        match (self, role) {
            (Self::DatabaseMigration, _) => {
                "This would change your project's database \
                migrations. A migration that has run is hard to undo, and it can change the data \
                as well as the shape of it."
            }
            (Self::CiConfiguration, _) => {
                "This would change your project's CI/CD configuration, \
                which decides what runs on its own after a push."
            }
            (Self::SecretMaterial, PathRole::Read) => {
                "This would read a file that holds \
                credentials or keys. Reading one is enough to put the secret in the agent's \
                context, where it can come back out in what the agent writes."
            }
            (Self::SecretMaterial, PathRole::Change) => {
                "This would change a file that holds \
                credentials or keys. Changing one can break a deployment quietly, or write a \
                secret somewhere it was not."
            }
            (Self::ConfigurationArea, _) => {
                "This would change SURE's own settings for this \
                project, which decide what SURE may check and what it keeps."
            }
            (Self::BroadFilesystemChange, _) => {
                "This names a whole location rather than one \
                file, so it could change many files at once. That may be a valid refactor, and \
                it can also remove working code."
            }
        }
    }
}

/// What a request would do to the path it names.
///
/// Two roles rather than one, because a read and a change are not the same
/// question: a secret is exposed by being read, while a migration is only a
/// migration when it is run or edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRole {
    /// The request reads the path.
    Read,
    /// The request writes or deletes the path.
    Change,
}

/// What role a domain action kind gives a path, if it names one at all.
///
/// A match with every variant named rather than a wildcard: a kind added to the
/// domain vocabulary must be answered here, and `_ => None` would silently
/// answer "names nothing" for a kind that names a file.
const fn path_role(action_kind: ActionKind) -> Option<PathRole> {
    match action_kind {
        ActionKind::ReadFile => Some(PathRole::Read),
        ActionKind::WriteProjectFile | ActionKind::DeleteProjectFile => Some(PathRole::Change),
        // None of these carries a path through the harness tool classifiers, and
        // an `ArbitraryCommand` deliberately does not: the only way to know what
        // a command line touches is to read shell grammar, which
        // `crate::safety` refuses, and a guess about a command's target is worse
        // than an answer that declines to make one.
        ActionKind::ListDirectory
        | ActionKind::ReadMetadata
        | ActionKind::StaticAnalysis
        | ActionKind::RunTests
        | ActionKind::Build
        | ActionKind::TypeCheck
        | ActionKind::Lint
        | ActionKind::StartService
        | ActionKind::LocalProbe
        | ActionKind::BrowserProbe
        | ActionKind::BrowserObservation
        | ActionKind::InstallDependencies
        | ActionKind::NetworkAccess
        | ActionKind::ArbitraryCommand
        | ActionKind::ExternalService => None,
    }
}

/// Which of the document's categories a request falls in, if any.
///
/// `None` is an answer rather than a gap: a path SURE does not recognise is not
/// classified, and the base decision alone answers it. Nothing here reads a
/// command line, and no request that names no path is treated as naming one.
///
/// The role comes back with the category because the sentence a user reads
/// depends on both: opening a key and rewriting a key are two different
/// sentences about two different consequences.
fn sensitive_area(
    action_kind: ActionKind,
    path: Option<&str>,
) -> Option<(SensitiveArea, PathRole)> {
    let role = path_role(action_kind)?;

    // A change that names no path is a whole-location change: SURE cannot bound
    // what it would touch, and reading a gap in the request as a reason to say
    // yes is the direction that fails open. A read that names no path is not
    // classified — there is nothing for either the secret rule or any other to
    // be about.
    let Some(path) = path else {
        return match role {
            PathRole::Change => Some((SensitiveArea::BroadFilesystemChange, role)),
            PathRole::Read => None,
        };
    };

    // Windows separators are folded, and the comparison is case-insensitive on
    // every platform rather than only on Windows. Finding a category is the
    // firmer answer, and a rule whose answer changed with the file system under
    // it would be a rule two machines could disagree about.
    let folded = folded(path);
    let folded = folded.as_str();

    if folded.is_empty() {
        return match role {
            PathRole::Change => Some((SensitiveArea::BroadFilesystemChange, role)),
            PathRole::Read => None,
        };
    }

    let segments: Vec<&str> = fold_segments(folded).collect();
    let file_name = segments.last().copied().unwrap_or_default();

    // A read is asked one question and no other, because the document's areas
    // are about changes: "strict also asks before additional sensitive
    // changes". Opening a file of credentials is itself the sensitive act — a
    // secret is exposed by being read, which is why "sensitive read" is one of
    // the dangerous actions `THREAT_MODEL.md` names — while reading a migration
    // or a CI file is ordinary work an agent does all day. So the read rule
    // covers credentials and stops there.
    if role == PathRole::Read {
        return is_secret_material(&segments, file_name)
            .then_some((SensitiveArea::SecretMaterial, role));
    }

    if names_a_whole_location(folded, segments.len()) {
        return Some((SensitiveArea::BroadFilesystemChange, role));
    }
    if is_a_migration(&segments, file_name) {
        return Some((SensitiveArea::DatabaseMigration, role));
    }
    if is_ci_configuration(&segments, file_name) {
        return Some((SensitiveArea::CiConfiguration, role));
    }
    if is_secret_material(&segments, file_name) {
        return Some((SensitiveArea::SecretMaterial, role));
    }
    if is_configuration_area(&segments, file_name) {
        return Some((SensitiveArea::ConfigurationArea, role));
    }
    None
}

/// Directories whose contents are credentials or key material.
const SECRET_DIRECTORIES: [&str; 5] = [".ssh", ".aws", ".gnupg", ".kube", "secrets"];

/// Files that are credentials or key material by name.
const SECRET_FILE_NAMES: [&str; 9] = [
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "kubeconfig",
    ".npmrc",
    ".netrc",
    "terraform.tfstate",
    "secrets",
];

/// Extensions that carry key material or a secret's value.
const SECRET_EXTENSIONS: [&str; 7] = [
    ".pem",
    ".key",
    ".p12",
    ".pfx",
    ".jks",
    ".keystore",
    ".tfvars",
];

/// Directory names a migration lives under.
const MIGRATION_DIRECTORIES: [&str; 4] = ["migrations", "migration", "migrate", "alembic"];

/// Directory names CI/CD configuration lives under.
const CI_DIRECTORIES: [&str; 4] = [".circleci", ".buildkite", ".woodpecker", ".teamcity"];

/// Files that are CI/CD configuration by name.
const CI_FILE_NAMES: [&str; 9] = [
    ".gitlab-ci.yml",
    "jenkinsfile",
    "azure-pipelines.yml",
    ".travis.yml",
    "appveyor.yml",
    "bitrise.yml",
    ".drone.yml",
    ".woodpecker.yml",
    ".gitlab-ci.yaml",
];

/// A path as SURE compares it: both separators folded to one, case folded,
/// surrounding whitespace trimmed.
///
/// One function rather than the same three steps at each call site, because the
/// question *is this the same path* has to have one answer: a path and a word
/// out of a command line are folded by the same rule or the breadth rule would
/// answer differently about the two halves of the same request.
fn folded(text: &str) -> String {
    text.replace('\\', "/").trim().to_lowercase()
}

/// The path segments of a folded path, empty ones dropped.
///
/// A `/` at the front, at the back, or doubled names the same thing as one, and
/// [`names_a_whole_location`] asks the segment count as well as the text.
fn fold_segments(folded: &str) -> impl Iterator<Item = &str> {
    folded.split('/').filter(|part| !part.is_empty())
}

/// Whether the path names a location rather than one file.
fn names_a_whole_location(normalised: &str, segment_count: usize) -> bool {
    matches!(normalised, "." | ".." | "~" | "/")
        // A path ending at a separator names the directory itself, so whatever
        // is under it is in scope. A Windows root arrives here too: `C:\` folds
        // to `c:/`.
        || normalised.ends_with('/')
        // A pattern matches many files by construction.
        || normalised.chars().any(|c| matches!(c, '*' | '?' | '['))
        // A volume with no path on it: `c:` — every path on the drive.
        || (segment_count == 1 && normalised.ends_with(':'))
}

/// Whether the path names a database migration.
///
/// Names rather than contents: SURE recognises the directory a migration lives
/// in (`migrations/`, `alembic/`) and a file whose name ends with the word
/// before its extension. A project that names its migrations something else is
/// not classified, and the base decision alone answers it — a rule that guessed
/// from a similarity would be the new rule the surrounding modules exist to
/// avoid. `tests/migration_tests.rs` is the case that keeps this honest: it is
/// a test about migrations, and matching any name containing the word would
/// hold it.
fn is_a_migration(segments: &[&str], file_name: &str) -> bool {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _extension)| stem);
    segments
        .iter()
        .any(|segment| MIGRATION_DIRECTORIES.contains(segment))
        || stem == "migration"
        || stem.ends_with("_migration")
        || stem.ends_with("-migration")
        || stem.ends_with(".migration")
}

/// Whether the path names CI/CD configuration.
fn is_ci_configuration(segments: &[&str], file_name: &str) -> bool {
    CI_FILE_NAMES.contains(&file_name)
        || segments
            .iter()
            .any(|segment| CI_DIRECTORIES.contains(segment))
        // GitHub keeps workflows one level down, and the other files in
        // `.github` are issue templates and ownership notices rather than
        // anything that decides what runs after a push.
        || segments
            .windows(2)
            .any(|pair| pair[0] == ".github" && pair[1] == "workflows")
}

/// Whether the path names credentials or key material.
fn is_secret_material(segments: &[&str], file_name: &str) -> bool {
    segments
        .iter()
        .any(|segment| SECRET_DIRECTORIES.contains(segment))
        || SECRET_FILE_NAMES.contains(&file_name)
        || SECRET_EXTENSIONS
            .iter()
            .any(|extension| file_name.ends_with(extension))
        || file_name == ".env"
        || file_name.starts_with(".env.")
        // `credentials.json`, `credentials.yaml`, `credentials.csv`: the file
        // the cloud SDKs read a key out of.
        || file_name.starts_with("credentials")
}

/// Whether the path names SURE's own settings for the project.
fn is_configuration_area(segments: &[&str], file_name: &str) -> bool {
    file_name == crate::paths::USER_CONFIG_FILE
        || segments.contains(&crate::paths::PROJECT_CACHE_DIR)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn standard() -> ProtectionMode {
        ProtectionMode::Standard
    }

    fn writer() -> ExecutionPermissions {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.write_project = true;
        permissions
    }

    #[test]
    fn read_file_is_allowed_in_inspect_only() {
        let decision = decide_cursor_tool(
            &ToolRequest::of("Read"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
        // An allow reached by the rule explains itself; `None` is reserved for
        // an event that put no request to the rule (see `allow()`).
        assert!(decision.reason.is_some());
    }

    #[test]
    fn shell_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            &ToolRequest::of("Shell"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn delete_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            &ToolRequest::of("Delete"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn write_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            &ToolRequest::of("Write"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn delete_is_allowed_with_write_permission() {
        let decision = decide_cursor_tool(
            &ToolRequest::at("Delete", "src/lib.rs"),
            ExecutionMode::HostConfirmed,
            &writer(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn write_is_allowed_with_write_permission() {
        let decision = decide_cursor_tool(
            &ToolRequest::at("Write", "src/lib.rs"),
            ExecutionMode::HostConfirmed,
            &writer(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn shell_needs_consent_even_with_run_project_code() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.run_project_code = true;
        let decision = decide_cursor_tool(
            &ToolRequest::of("Shell"),
            ExecutionMode::HostConfirmed,
            &permissions,
            standard(),
        );
        // ArbitraryCommand always returns NeedsConsent. A hook that cannot ask
        // for consent must fail closed, so it maps to Block rather than Warn.
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn unknown_tool_is_treated_as_arbitrary_command() {
        let decision = decide_cursor_tool(
            &ToolRequest::of("UnknownTool"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
    }

    #[test]
    fn claude_code_bash_is_blocked_in_inspect_only() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Bash"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn claude_code_read_is_allowed_in_inspect_only() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Read"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn claude_code_write_is_blocked_in_inspect_only() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Write"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn claude_code_edit_is_blocked_in_inspect_only() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Edit"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn claude_code_delete_is_blocked_in_inspect_only() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Delete"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn claude_code_write_is_allowed_with_write_permission() {
        let decision = decide_claude_code_tool(
            &ToolRequest::at("Write", "src/lib.rs"),
            ExecutionMode::HostConfirmed,
            &writer(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn claude_code_edit_is_allowed_with_write_permission() {
        let decision = decide_claude_code_tool(
            &ToolRequest::at("Edit", "src/lib.rs"),
            ExecutionMode::HostConfirmed,
            &writer(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn claude_code_bash_needs_consent_even_with_run_project_code() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.run_project_code = true;
        let decision = decide_claude_code_tool(
            &ToolRequest::of("Bash"),
            ExecutionMode::HostConfirmed,
            &permissions,
            standard(),
        );
        // ArbitraryCommand always returns NeedsConsent. A hook that cannot ask
        // for consent must fail closed, so it maps to Block rather than Warn.
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn claude_code_unknown_tool_is_treated_as_arbitrary_command() {
        let decision = decide_claude_code_tool(
            &ToolRequest::of("UnknownTool"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            standard(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
    }

    // --- P13-T004: the protection mode changes the answer -------------------

    /// The one the acceptance criterion is about: the same operation, two
    /// modes, two decisions. `Read` is the case both modes can reach without a
    /// permission this build cannot grant, and the document's "secret/config
    /// area" is the area strict adds.
    #[test]
    fn strict_blocks_a_secret_read_that_standard_allows() {
        let inspect = ExecutionPermissions::inspect_only();
        let read = ToolRequest::at("Read", ".env");

        let under_standard = decide_cursor_tool(
            &read,
            ExecutionMode::InspectOnly,
            &inspect,
            ProtectionMode::Standard,
        );
        let under_strict = decide_cursor_tool(
            &read,
            ExecutionMode::InspectOnly,
            &inspect,
            ProtectionMode::Strict,
        );

        assert_eq!(under_standard.decision, ProtectionDecisionKind::Allow);
        assert_eq!(under_strict.decision, ProtectionDecisionKind::Block);
        let reason = under_strict.reason.expect("a block has a reason");
        assert!(
            reason.contains("credentials"),
            "the reason should say what the file is, in the user's words: {reason}"
        );
        assert!(
            !reason.contains("protection.mode") && !reason.contains("strict mode is set"),
            "the reason must explain the consequence rather than name the setting: {reason}"
        );
    }

    /// Every path here is a change: the document's areas, and the paths that
    /// look like them without being them. The second half is the guard against
    /// the failure this criterion invites — a mode that holds more than the
    /// document says is a new rule wearing an existing name.
    #[test]
    fn strict_holds_a_change_in_the_documents_areas_and_nothing_else() {
        let cases = [
            // Migrations: by directory, and by name.
            ("migrations/0001_init.sql", true),
            ("db/migrate/20260919_add_users.rb", true),
            ("alembic/versions/0001_init.py", true),
            ("db/20260919_add_users_migration.sql", true),
            // CI/CD configuration.
            (".github/workflows/ci.yml", true),
            (".circleci/config.yml", true),
            ("Jenkinsfile", true),
            (".gitlab-ci.yml", true),
            // Credentials and keys.
            (".env", true),
            (".env.production", true),
            ("secrets/prod.yaml", true),
            ("certs/server.pem", true),
            (".ssh/id_ed25519", true),
            ("terraform.tfstate", true),
            // SURE's own settings for the project.
            ("sure.yaml", true),
            (".sure/cache.json", true),
            // The paths that are not any of those.
            ("src/lib.rs", false),
            ("README.md", false),
            (".github/ISSUE_TEMPLATE/bug.md", false),
            ("tests/migration_tests.rs", false),
            ("docs/adr/0004-thin-harness-integrations.md", false),
            ("src/config/settings.rs", false),
        ];
        for (path, held) in cases {
            let under_standard = decide_cursor_tool(
                &ToolRequest::at("Write", path),
                ExecutionMode::HostConfirmed,
                &writer(),
                ProtectionMode::Standard,
            );
            let under_strict = decide_cursor_tool(
                &ToolRequest::at("Write", path),
                ExecutionMode::HostConfirmed,
                &writer(),
                ProtectionMode::Strict,
            );
            assert_eq!(
                under_standard.decision,
                ProtectionDecisionKind::Allow,
                "standard held a change to {path}, which the document says it does not"
            );
            assert_eq!(
                under_strict.decision == ProtectionDecisionKind::Block,
                held,
                "strict and the document disagree about a change to {path}"
            );
            if held {
                assert!(under_strict.reason.is_some());
            }
        }
    }

    /// A read is held for credentials and for nothing else, because the
    /// document's other areas are about changes. Reading a migration or a CI
    /// file is work an agent does all day; reading a key is how the key leaves.
    #[test]
    fn strict_holds_a_read_only_where_the_read_is_the_sensitive_act() {
        let cases = [
            (".env", true),
            ("secrets/prod.yaml", true),
            ("id_rsa", true),
            ("certs/server.pem", true),
            ("C:\\Users\\dev\\project\\.env", true),
            ("migrations/0001_init.sql", false),
            (".github/workflows/ci.yml", false),
            ("sure.yaml", false),
            ("src/lib.rs", false),
        ];
        let inspect = ExecutionPermissions::inspect_only();
        for (path, held) in cases {
            let under_strict = decide_cursor_tool(
                &ToolRequest::at("Read", path),
                ExecutionMode::InspectOnly,
                &inspect,
                ProtectionMode::Strict,
            );
            assert_eq!(
                under_strict.decision == ProtectionDecisionKind::Block,
                held,
                "strict and the document disagree about a read of {path}"
            );
        }
    }

    #[test]
    fn a_change_that_names_a_whole_location_is_held_under_strict() {
        for path in ["", ".", "..", "src/", "C:\\", "**/*", "src/*.rs"] {
            let decision = decide_cursor_tool(
                &ToolRequest::at("Delete", path),
                ExecutionMode::HostConfirmed,
                &writer(),
                ProtectionMode::Strict,
            );
            assert_eq!(
                decision.decision,
                ProtectionDecisionKind::Block,
                "strict let a whole-location change through: {path:?}"
            );
        }
    }

    /// A request that names no path at all is not evidence of anything, so a
    /// read without one is not held; a change without one is held, because SURE
    /// cannot bound what it would touch.
    #[test]
    fn a_request_that_names_no_path_is_answered_by_what_it_would_do() {
        let inspect = ExecutionPermissions::inspect_only();
        let read = decide_cursor_tool(
            &ToolRequest::of("Read"),
            ExecutionMode::InspectOnly,
            &inspect,
            ProtectionMode::Strict,
        );
        assert_eq!(read.decision, ProtectionDecisionKind::Allow);

        let change = decide_claude_code_tool(
            &ToolRequest::of("Delete"),
            ExecutionMode::HostConfirmed,
            &writer(),
            ProtectionMode::Strict,
        );
        assert_eq!(change.decision, ProtectionDecisionKind::Block);
    }

    /// A rule whose answer changed with the file system under it would be a
    /// rule two machines could disagree about, so separator and case are folded
    /// on every platform rather than only on Windows.
    #[test]
    fn paths_are_matched_without_regard_to_separator_or_case() {
        let inspect = ExecutionPermissions::inspect_only();
        for path in [
            ".env",
            "C:\\Users\\dev\\project\\.env",
            "SECRETS/PROD.YAML",
            "c:/Users/dev/project/Id_Rsa",
            "Certs/Server.PEM",
        ] {
            let decision = decide_cursor_tool(
                &ToolRequest::at("Read", path),
                ExecutionMode::InspectOnly,
                &inspect,
                ProtectionMode::Strict,
            );
            assert_eq!(
                decision.decision,
                ProtectionDecisionKind::Block,
                "strict missed a credential path: {path}"
            );
        }
    }

    /// The mode asks its question only after the engine has answered, and never
    /// weakens an answer it was given.
    #[test]
    fn a_refusal_from_the_execution_mode_is_not_diluted_or_respelled() {
        let inspect = ExecutionPermissions::inspect_only();
        let denied = decide_cursor_tool(
            &ToolRequest::at("Write", ".env"),
            ExecutionMode::InspectOnly,
            &inspect,
            ProtectionMode::Strict,
        );
        assert_eq!(denied.decision, ProtectionDecisionKind::Block);
        assert_eq!(
            denied.reason.as_deref(),
            Some("The current execution mode does not permit this action."),
            "the reason should be the engine's own, not one the mode wrote"
        );
    }

    /// `custom` is a mode this release refuses in a file, and the decision path
    /// is total over the enum: if the value ever arrives at a decision, the
    /// answer is a refusal with the configuration reader's own words rather
    /// than an allow nobody chose.
    #[test]
    fn custom_is_refused_with_the_settings_readers_own_words() {
        let decision = decide_cursor_tool(
            &ToolRequest::at("Read", "src/lib.rs"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            ProtectionMode::Custom,
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        let reason = decision.reason.expect("a refusal says why");
        assert!(reason.contains(CUSTOM_PROTECTION_EXPLANATION), "{reason}");
        assert!(reason.contains(CUSTOM_PROTECTION_INSTEAD), "{reason}");
    }

    #[test]
    fn every_area_has_a_sentence_for_both_roles() {
        for area in SensitiveArea::ALL {
            for role in [PathRole::Read, PathRole::Change] {
                let sentence = area.consequence(role);
                assert!(
                    sentence.len() > 40,
                    "{area:?} has no real sentence for {role:?}: {sentence}"
                );
                assert!(
                    sentence.ends_with('.'),
                    "{area:?}/{role:?} is not a sentence: {sentence}"
                );
            }
        }
    }

    #[test]
    fn every_strict_block_says_what_would_happen_and_what_sure_answers() {
        for area in SensitiveArea::ALL {
            let reason = strict_reason(*area, PathRole::Change);
            assert!(reason.ends_with(STRICT_TAIL), "{reason}");
            assert!(
                !reason.contains("protection.mode"),
                "policy jargon reached a user: {reason}"
            );
        }
    }

    #[test]
    fn only_the_actions_that_name_a_file_have_a_role() {
        for kind in ActionKind::ALL {
            let role = path_role(*kind);
            let expected = match kind {
                ActionKind::ReadFile => Some(PathRole::Read),
                ActionKind::WriteProjectFile | ActionKind::DeleteProjectFile => {
                    Some(PathRole::Change)
                }
                _ => None,
            };
            assert_eq!(role, expected, "{kind:?}");
        }
    }

    // --- P13-T005: the three dangers, and what may be allowed once ----------

    /// The canonical broad delete, on every spelling of "a whole location" the
    /// breadth rule already recognises. `rm -rf build` is deliberately absent:
    /// the rule reads a location from a trailing separator, a glob, a drive or
    /// one of the location words, and `build` is a name like any other — which
    /// is a limit, not an accident, and it is the same limit strict mode
    /// applies to a path.
    #[test]
    fn a_destructive_command_over_a_whole_location_is_a_broad_delete() {
        for line in [
            "rm -rf /",
            "rm -rf .",
            "rm -rf ..",
            "rm -rf ~",
            "rm -rf *",
            "rm -rf build/",
            "rm -rf -- .",
            "del *.*",
            "rmdir /s ..",
            "shred -u ~",
            "git clean -fdx .",
            "git rm -r --cached .",
        ] {
            assert_eq!(
                command_danger(line),
                Some(Danger::BroadDelete),
                "{line:?} was not named"
            );
        }
    }

    /// An operand-free destructive command is not a broad delete. `git clean
    /// -fdx` and `git reset --hard` destroy a lot without naming anything SURE
    /// can bound, and a sentence claiming *a whole location* about a command
    /// whose location SURE has not established would be a sentence SURE cannot
    /// stand behind. They are still held; they are not overridable.
    #[test]
    fn a_destructive_command_that_names_nothing_is_not_named_as_broad() {
        for line in ["git clean -fdx", "git reset --hard", "git prune"] {
            assert_eq!(command_danger(line), None, "{line:?}");
        }
    }

    #[test]
    fn a_push_that_destroys_history_is_a_force_push() {
        for line in [
            "git push --force",
            "git push -f origin main",
            "git push --force-with-lease origin main",
            "git push --force-if-includes origin main",
        ] {
            assert_eq!(
                command_danger(line),
                Some(Danger::ForcePush),
                "{line:?} was not named"
            );
        }
    }

    /// The half that keeps the naming honest: an ordinary push is network and
    /// nothing else, and a command that merely mentions the words is not a
    /// force push — the classifier decides, not a search for "force".
    #[test]
    fn a_push_that_destroys_nothing_is_not_a_force_push() {
        for line in [
            "git push",
            "git push origin main",
            "git log --force",
            "echo git push --force",
        ] {
            assert_eq!(command_danger(line), None, "{line:?}");
        }
    }

    /// Everything SURE cannot read or cannot classify stays unnamed, which is
    /// the answer that asserts nothing: the request keeps the block it already
    /// had, and no allowance can be recorded for a danger SURE did not name.
    #[test]
    fn a_command_sure_cannot_read_or_classify_names_no_danger() {
        assert_eq!(command_danger(""), None);
        assert_eq!(command_danger("   "), None);
        // Text with more than one reading.
        for line in [
            "rm -rf \"my dir\"",
            "rm -rf 'my dir'",
            "rm -rf my\\ dir",
            "rm -rf $HOME",
            "cargo test && rm -rf /",
            "rm -rf . || true",
            "cat secrets > out",
        ] {
            assert_eq!(command_danger(line), None, "{line:?} was read");
        }
        // Read, and the classifier has no row: every category, `Destructive`
        // among them, and SURE's own refusal to classify is not a fact about
        // the command.
        assert_eq!(command_danger("frobnicate /"), None);
        // A destructive command whose operand is a name rather than a location.
        assert_eq!(command_danger("rm -rf build"), None);
        assert_eq!(command_danger("del report.txt"), None);
    }

    #[test]
    fn every_danger_has_a_sentence_in_the_users_words() {
        for danger in Danger::ALL {
            let consequence = danger.consequence();
            assert!(consequence.len() > 40, "{danger:?}: {consequence}");
            assert!(consequence.ends_with('.'), "{danger:?}: {consequence}");
            let held = danger_reason(*danger);
            assert!(held.starts_with(consequence), "{held}");
            assert!(held.contains("SURE does not allow it"), "{held}");
            let allowed = allowance_reason(*danger);
            assert!(allowed.starts_with(consequence), "{allowed}");
            assert!(
                allowed.contains("SURE would let this one through"),
                "{allowed}"
            );
            let unread = allowance_unreadable_reason(*danger);
            assert!(unread.starts_with(consequence), "{unread}");
            assert!(unread.contains("does not allow this"), "{unread}");
            for sentence in [&held, &allowed, &unread] {
                assert!(
                    !sentence.contains("protection.mode") && !sentence.contains("can_grant"),
                    "policy jargon reached a user: {sentence}"
                );
            }
        }
    }

    #[test]
    fn every_danger_has_one_stored_name() {
        // The accessor and the serde attribute are two spellings of one name, and
        // this is where they are held together: a stored decision written by one
        // and read by the other must not be a decision this build cannot name.
        for danger in Danger::ALL {
            let written = serde_json::to_value(danger).expect("a unit variant serialises");
            assert_eq!(
                written,
                serde_json::Value::String(danger.wire_name().to_owned()),
                "{danger:?} has a different serde name from its wire name"
            );
            assert_eq!(
                serde_json::from_value::<Danger>(written).expect("and reads back"),
                *danger
            );
            assert_ne!(
                danger.wire_name(),
                danger.as_str(),
                "{danger:?}: the sentence a user reads is not the name a record keeps"
            );
        }
    }

    /// The whole point of the reading: a shell request that SURE would have
    /// answered with one generic sentence about consent now says what the
    /// command would do. The refusal is not weakened — it is the same Block.
    #[test]
    fn a_shell_request_held_for_a_named_danger_says_what_it_would_do() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.run_project_code = true;
        let assessment = assess_claude_code_tool(
            &ToolRequest::running("Bash", "rm -rf build/"),
            ExecutionMode::HostConfirmed,
            &permissions,
            ProtectionMode::Standard,
        );
        assert_eq!(assessment.decision.decision, ProtectionDecisionKind::Block);
        assert_eq!(assessment.danger, Some(Danger::BroadDelete));
        let reason = assessment.decision.reason.expect("a block says why");
        assert!(reason.contains("whole location"), "{reason}");
        assert!(
            !reason.contains("rm -rf") && !reason.contains("build"),
            "a field of the request was echoed into a sentence SURE writes: {reason}"
        );
    }

    /// A request SURE cannot read is answered exactly as it was before this
    /// existed: the same sentence, and no danger to spend an allowance on.
    #[test]
    fn a_shell_request_sure_cannot_read_keeps_the_consent_sentence() {
        // The permissions that reach `NeedsConsent` rather than `Denied`: an
        // arbitrary command is held for consent even where running project code
        // is allowed, because a hook cannot ask for the consent.
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.run_project_code = true;
        for command in ["rm -rf \"my dir\"", "cargo test && rm -rf /", "npm test"] {
            let assessment = assess_claude_code_tool(
                &ToolRequest::running("Bash", command),
                ExecutionMode::HostConfirmed,
                &permissions,
                ProtectionMode::Standard,
            );
            assert_eq!(assessment.danger, None, "{command:?}");
            assert_eq!(
                assessment.decision.decision,
                ProtectionDecisionKind::Block,
                "{command:?}"
            );
            assert_eq!(
                assessment.decision.reason.as_deref(),
                Some(
                    "This action needs explicit approval; the hook cannot obtain consent, so it \
                     is blocked."
                ),
                "{command:?}"
            );
        }
    }

    /// A path tool carries no command line, so nothing is read for one — and a
    /// request that was never held carries no danger either.
    #[test]
    fn a_danger_only_ever_attaches_to_a_held_request() {
        let inspect = ExecutionPermissions::inspect_only();
        let allowed = assess_cursor_tool(
            &ToolRequest::at("Read", "src/lib.rs"),
            ExecutionMode::InspectOnly,
            &inspect,
            ProtectionMode::Standard,
        );
        assert_eq!(allowed.decision.decision, ProtectionDecisionKind::Allow);
        assert_eq!(allowed.danger, None);

        // A command line on a tool that does not carry one is not read.
        assert_eq!(
            claimed_danger(
                ActionKind::ReadFile,
                &ToolRequest::running("Read", "rm -rf /")
            ),
            None
        );
        assert_eq!(
            claimed_danger(
                ActionKind::ArbitraryCommand,
                &ToolRequest::running("Bash", "rm -rf /")
            ),
            Some(Danger::BroadDelete)
        );
    }

    /// The engine's own refusal is not overridable: a one-time allowance is a
    /// way to let one request through the protection question, not a way to run
    /// under an execution mode the user did not change.
    #[test]
    fn the_execution_modes_refusal_is_never_a_danger() {
        let denied = assess_cursor_tool(
            &ToolRequest::at("Write", ".env"),
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            ProtectionMode::Strict,
        );
        assert_eq!(denied.decision.decision, ProtectionDecisionKind::Block);
        assert_eq!(denied.danger, None);

        let refused_mode = assess_cursor_tool(
            &ToolRequest::running("Shell", "rm -rf /"),
            ExecutionMode::HostConfirmed,
            &ExecutionPermissions::inspect_only(),
            ProtectionMode::Custom,
        );
        assert_eq!(refused_mode.danger, None);
    }

    /// Strict mode's categories, and which of them are the three dangers. A
    /// read of credentials is one; a change to a secret, a migration and CI
    /// configuration are held and are not.
    #[test]
    fn strict_names_a_danger_only_for_the_acts_this_task_is_about() {
        let cases = [
            ("Read", ".env", true, Some(Danger::SensitiveRead)),
            ("Read", "src/lib.rs", false, None),
            ("Write", ".env", true, None),
            ("Write", "migrations/0001_init.sql", true, None),
            ("Write", ".github/workflows/ci.yml", true, None),
            ("Delete", ".", true, Some(Danger::BroadDelete)),
            ("Delete", "src/lib.rs", false, None),
        ];
        for (tool, path, held, expected) in cases {
            let permissions = if tool == "Read" {
                ExecutionPermissions::inspect_only()
            } else {
                writer()
            };
            let assessment = assess_cursor_tool(
                &ToolRequest::at(tool, path),
                ExecutionMode::HostConfirmed,
                &permissions,
                ProtectionMode::Strict,
            );
            assert_eq!(assessment.danger, expected, "{tool} {path}");
            assert_eq!(
                assessment.decision.decision == ProtectionDecisionKind::Block,
                held,
                "{tool} {path}"
            );
        }
    }

    // --- what the writer reads before it writes (`P13-T010`) ------------------

    /// Settings under which nothing in `assess_request` stops a witness: the
    /// user's own file named the mode that runs project code, and every
    /// permission an act could need is granted. The three dangers are reachable
    /// from here, which is what makes this the reading the witness list is
    /// checked against.
    fn everything_granted() -> ExecutionPermissions {
        let mut permissions = writer();
        permissions.run_project_code = true;
        permissions
    }

    /// The witness list is the whole of what a danger can be read from, and this
    /// is what holds it to that: each witness must produce the danger it exists
    /// for under settings that reach every rule, and the five of them must name
    /// all three acts. It fails if the rule changes so that a witness stops
    /// naming its danger — the one way the list can go stale — and it fails if a
    /// fourth act is added with no witness to reach it, because the acts the
    /// list produces would then be shorter than the vocabulary.
    #[test]
    fn every_witness_names_its_danger() {
        let permissions = everything_granted();
        let expected = [
            Danger::BroadDelete,
            Danger::ForcePush,
            Danger::SensitiveRead,
            Danger::BroadDelete,
            Danger::BroadDelete,
        ];
        for ((action_kind, request), expected) in DANGER_WITNESSES.iter().zip(expected) {
            let assessment = assess_request(
                *action_kind,
                request,
                ExecutionMode::HostConfirmed,
                &permissions,
                ProtectionMode::Strict,
            );
            assert_eq!(
                assessment.danger,
                Some(expected),
                "{} is no longer held for {}",
                request.subject().expect("every witness names a subject"),
                expected.as_str()
            );
        }
        assert_eq!(
            acts_a_request_could_be_held_for(
                ExecutionMode::HostConfirmed,
                &permissions,
                ProtectionMode::Strict
            ),
            Danger::ALL.to_vec(),
            "an act in the vocabulary has no witness that reaches it"
        );
    }

    /// The list the writer carries into its confirmation, at the settings that
    /// decide it, and the invariant that the list and the refusal can never
    /// disagree: a refusal is exactly the case where the list is empty.
    #[test]
    fn the_acts_left_for_an_allowance_are_read_off_the_rule() {
        let inspect_only = ExecutionPermissions::inspect_only();
        let runner = everything_granted();

        // The default configuration — no `sure.yaml` that says anything and no
        // user settings file — leaves nothing. This is the case the brief for
        // `P13-T010` measured as the one every user is in.
        assert_eq!(
            acts_a_request_could_be_held_for(
                ExecutionMode::InspectOnly,
                &inspect_only,
                ProtectionMode::Standard
            ),
            Vec::new(),
            "the default settings were read as leaving an act an allowance could be spent on"
        );
        // Strict puts one back, on the permission every run has: a read of a
        // file credentials live in. So a grant written in this project is
        // spendable, and the confirmation may say so.
        assert_eq!(
            acts_a_request_could_be_held_for(
                ExecutionMode::InspectOnly,
                &inspect_only,
                ProtectionMode::Strict
            ),
            vec![Danger::SensitiveRead]
        );
        // The user's own file naming the mode, with standard protection: the two
        // destructive shell acts, in the order the vocabulary declares them, and
        // the read is not among them because standard asks no question about a
        // read.
        assert_eq!(
            acts_a_request_could_be_held_for(
                ExecutionMode::HostConfirmed,
                &runner,
                ProtectionMode::Standard
            ),
            vec![Danger::BroadDelete, Danger::ForcePush]
        );
        // Everything at once, which is where a user who took both remedies
        // lands: all three, once each, in the vocabulary's own order.
        assert_eq!(
            acts_a_request_could_be_held_for(
                ExecutionMode::HostConfirmed,
                &runner,
                ProtectionMode::Strict
            ),
            Danger::ALL.to_vec()
        );

        // The invariant, over every combination the two enums and a few
        // permission sets can make — including ones no authority builds.
        let mut blind = ExecutionPermissions::inspect_only();
        blind.inspect = false;
        blind.write_project = false;
        for mode in ExecutionMode::ALL {
            for protection in ProtectionMode::ALL {
                for permissions in [
                    inspect_only.clone(),
                    runner.clone(),
                    writer(),
                    blind.clone(),
                ] {
                    let acts = acts_a_request_could_be_held_for(*mode, &permissions, *protection);
                    assert_eq!(
                        allowance_could_not_be_spent_reason(*mode, &permissions, *protection)
                            .is_none(),
                        !acts.is_empty(),
                        "{mode:?} {protection:?} {permissions:?}: the sentence and the list disagree"
                    );
                }
            }
        }
    }

    /// The sentence a user reads in the settings they actually have, and the
    /// rule that keeps its remedies honest: every change it offers must be one
    /// the same rule says would work.
    #[test]
    fn the_refusal_names_the_setting_and_only_a_change_that_would_work() {
        let reason = allowance_could_not_be_spent_reason(
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
            ProtectionMode::Standard,
        )
        .expect("the default settings leave nothing an allowance could be spent on");

        // The setting — not the value alone — and the act the allowance is for.
        for needle in [
            "execution.mode",
            "inspect_only",
            "protection.mode",
            "standard",
            "host_confirmed",
            "strict",
            "sure doctor",
            "sure.yaml",
            "Nothing was written",
        ] {
            assert!(
                reason.contains(needle),
                "the refusal does not say {needle}: {reason}"
            );
        }
        assert!(
            reason.contains(&Danger::in_a_sentence(Danger::ALL)),
            "the refusal does not name the acts an allowance covers: {reason}"
        );

        // And the case where one of the two remedies would not work: SURE may
        // not read or change this project's files, so strict protection would
        // ask its question of nobody. The sentence must offer the mode and not
        // the protection, because a user who did as they were told and was
        // refused again has been told the wrong thing.
        let mut blind = ExecutionPermissions::inspect_only();
        blind.inspect = false;
        blind.write_project = false;
        let reason = allowance_could_not_be_spent_reason(
            ExecutionMode::InspectOnly,
            &blind,
            ProtectionMode::Standard,
        )
        .expect("no act is reachable when SURE may not read or change a file");
        assert!(
            reason.contains("execution.mode"),
            "the refusal dropped a remedy that would work: {reason}"
        );
        assert!(
            !reason.contains("Set `protection.mode: strict`"),
            "the refusal offers a change that would leave the grant unspendable: {reason}"
        );
        assert!(
            reason.contains("do not let SURE read or change"),
            "the refusal does not say why that remedy is not offered: {reason}"
        );

        // The mode this release cannot apply is answered with the value named,
        // and with the sentences the configuration reader already refuses it
        // with rather than a second explanation of the same value.
        let reason = allowance_could_not_be_spent_reason(
            ExecutionMode::HostConfirmed,
            &everything_granted(),
            ProtectionMode::Custom,
        )
        .expect("a mode this release cannot apply holds every request, so no act is reachable");
        assert!(reason.contains("`protection.mode` is `custom`"), "{reason}");
        assert!(reason.contains(CUSTOM_PROTECTION_INSTEAD), "{reason}");
    }
}
