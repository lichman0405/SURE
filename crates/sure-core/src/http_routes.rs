//! The routes a project declares, and asking the ones SURE may ask.
//!
//! `P5-T003`'s acceptance, and it is two sentences:
//!
//! > *Known local routes can be probed safely.*
//! > *Response evidence is bound to run/fingerprint.*
//!
//! # The absence this module closes, in the words of the module that recorded it
//!
//! [`crate::runtime_probes`] states why `P5-T001` proposed no route check:
//!
//! > *A route check is deliberately absent, and the reason is [`CheckReason`]'s:
//! > a probe that asked whether `/api/health` answers would have to name where
//! > SURE read that a route exists, and nothing in the discovery holds one — a
//! > `package.json` declares scripts and dependencies, not routes. A reason built
//! > for it would therefore name a file that says no such thing. **Routes are
//! > `P5-T003`'s work and they arrive with the reading that can point at them.***
//!
//! This module is that reading. A [`Route`] here is never a guess about a
//! framework: it is a **line of a project's source**, and
//! [`Route::declared_in`] with [`Route::line`] is where a reader goes to see the
//! same characters SURE read. [`RouteReading`] is what it found, and
//! [`NotProbed`] is every route it found and did not ask, each with the reason.
//!
//! # Known means read, and read means one line states the method and the path
//!
//! A project's route table is not a file. It is a value some code builds, and
//! the code that builds it can compute a path from a variable, take it from a
//! configuration key, mount a router under a prefix three files away, or loop
//! over a list of names. **This module reads none of those**, because none of
//! them is a fact one line states, and
//! `docs/product/MVP_SPEC.md` bounds the whole subject to what is *"frontend/
//! backend route/API consistency where **deterministically discoverable**"*.
//!
//! So the reading is narrow on purpose, and the narrowness is stated rather
//! than discovered by a user:
//!
//! | stack | what is read | what makes the path the whole path |
//! |---|---|---|
//! | Python | a decorator `@<app>.<verb>("<path>")` or `@<app>.route("<path>", methods=[...])` | `<app>` is a name this file binds to `Flask(` or `FastAPI(` |
//! | JavaScript, TypeScript | a statement-initial `<app>.<verb>("<path>", …)` | `<app>` is a name this file binds to `express()` |
//! | Rust | `.route("<path>", <verb>(<handler>))` on a chain a `let` names | that name is not the argument of a `nest(` or `merge(` call in the same file |
//!
//! **The third column is the part that decides a verdict, and each stack needs a
//! different rule because each stack mounts differently.** In Flask, FastAPI and
//! Express, a route declared on *the application object* is served at the path it
//! names — later `app.use("/api", router)` and `app.include_router(router)`
//! calls mount *other* objects and move nothing already declared. That is why
//! the rule is *the receiver must be the application object*: a route on
//! `router.get("/health")` is a route whose real path depends on a line that may
//! not even be in this file, and asking `/health` for it would report a working
//! route as missing. In Rust the application object is not distinguishable from
//! a nested router — both are `Router::new()` — so the rule is the other way
//! round: the chain's name must not appear as the argument of a `nest(` or
//! `merge(` call.
//!
//! **A route SURE can read and cannot place is [`NotProbed`] and not a
//! failure.** It is the same shape [`crate::runtime_probes::NotPlanned`] has and
//! for the same reason: a route that produced no check would produce no row, and
//! a reader takes the absence of a row for the absence of a problem.
//!
//! # One method, and it is the one `probe.rs` builds
//!
//! [`RouteMethod`] has the seven methods a route can be declared for, and
//! [`RouteMethod::is_a_read`] answers the HTTP question *would sending this
//! change something*. **Neither of those is the question that decides what SURE
//! asks.** [`crate::probe`] writes exactly one request line —
//! [`Endpoint::request_line`] is `GET <path> HTTP/1.1` and it is the only thing
//! that function has ever built — so `HEAD` and `OPTIONS` are reads by HTTP's
//! meaning and **not reads this product can make**.
//!
//! **So there are two ways a route is not asked for its method and they are
//! different claims**: [`NotProbedBecause::WouldChangeSomething`] is about the
//! project — this route is a `POST`, and a smoke check that sent one would be
//! changing a project's data to see whether it works — and
//! [`NotProbedBecause::NotTheReadSureAsks`] is about SURE — this route is a
//! `HEAD`, which changes nothing, and SURE has no way to send one. Collapsing
//! them would make the second read as a fact about the project.
//!
//! # A path with a slot in it is read and not asked
//!
//! `/items/{item_id}` and `/items/<int:item_id>` and `/items/:id` all name a
//! resource and none of them names a *request*. SURE has no value for the slot,
//! and **inventing one — `1`, `test`, `example` — would be SURE asking a question
//! the project never offered, and reporting the answer as a fact about the
//! project.** A `404` on `/items/1` is not evidence about `/items/{item_id}`.
//! [`NotProbedBecause::HasASlot`] is that refusal, and it is the same refusal
//! [`crate::probe::Endpoint`] makes one level down when it declines to
//! percent-encode a path a caller may have encoded already.
//!
//! # Safe is a property of the constructors, not of this module's care
//!
//! Three things make this safe, and none of them is a check a caller could
//! forget:
//!
//! - **The address is loopback.** [`Endpoint`] refuses anything that is not
//!   [`is_loopback`](std::net::IpAddr::is_loopback), so a route probe is
//!   [`ActionKind::LocalProbe`] and needs
//!   [`Permission::Inspect`](sure_domain::execution::Permission::Inspect), which
//!   the vocabulary grants unconditionally. A probe against a machine that is
//!   not this one is [`ActionKind::ExternalService`] and needs a permission this
//!   module has no constructor for.
//! - **The method is `GET`**, for the reason one section up.
//! - **Nothing is written but one request line and three headers**, which is
//!   [`crate::probe`]'s existing argument and not a second one made here.
//!
//! The one thing this module could get wrong on its own is *where* it asks: a
//! route read from the source and asked at the wrong path is a working project
//! reported as broken. That is what the third column of the table above is for,
//! and it is why every uncertain case is a [`NotProbed`] rather than a probe.
//!
//! # Response evidence carries the fingerprint the run was built for
//!
//! **Every [`CheckResult`] this module produces carries
//! [`Enforcement::check_plan`]'s fingerprint**, and it is read there rather than
//! taken as a parameter for the reason [`crate::runtime_start::StartSmoke::of`]
//! gives about the same line: *"A fingerprint that arrived separately is one that
//! could describe a project state the command was not admitted for."*
//!
//! `docs/architecture/EVIDENCE_MODEL.md` is where that matters. Its **test
//! freshness** rule is that *"a test run against an older relevant project
//! fingerprint cannot prove the final code passes"*, and
//! [`CheckResult::project_fingerprint`] is the field that rule reads. A route
//! that answered at 12:01 says nothing about the tree at 12:04, and the result
//! is the object that knows which tree it was about. [`RouteSmoke`] has no
//! constructor that omits it, so a caller cannot produce a route result whose
//! fingerprint came from anywhere but the run's own plan.
//!
//! # What this does not do
//!
//! **It reads no route whose path is not a literal on the line that declares
//! it.** A path built with `f"{base}/health"`, `"/" + name`, `format!(...)`, or a
//! registered list is invisible here, and a project that declares all its routes
//! that way has no routes SURE knows. **The falsifier is the ordinary one**: a
//! task that needs those routes must first produce a reading that can *point at*
//! them, and [`Route`] is the type it would produce. Until then this absence is
//! the honest one, because the alternative is a probe at a path SURE guessed.
//!
//! **A literal that is not a path is not read either.** Express's
//! `app.get('*', …)` catch-all and a bare `app.get('health', …)` are the two a
//! real project writes, and a leading `/` is what tells a request target apart
//! from a pattern: `*` names every path rather than one, and `health` is a path
//! relative to a mount point this file does not name — so *what would SURE ask*
//! has no answer for either, and a reading that reported one as
//! [`NotProbedBecause::NotARequestTarget`] would be claiming the project cannot
//! serve it. Both are the same absence as the paragraph above and not a
//! different one.
//!
//! **It does not read a route's mount point from another file.** A `router`
//! object whose `app.use("/api", router)` lives in a file the scan did not show
//! is not asked, and neither is one whose mount is in the same file — the rule
//! is the receiver, not the search, and a search that came up empty would be a
//! search that concluded *not mounted* from *not found here*.
//!
//! **It does not read a route table shape.** `routes.json`, a Next.js `app/`
//! directory, an OpenAPI document and a Rails `routes.rb` are four other
//! readings, and each is a task with its own evidence. Nothing here refuses
//! them; this module simply does not look.
//!
//! **It does not follow a redirect, retry, or read a response body.** A `3xx` is
//! [`CheckStatus::Pass`](sure_domain::status::CheckStatus::Pass) because the
//! service answered successfully, which is the whole claim the check's title
//! makes. What a person would *see* after the redirect is
//! [`crate::browser`]'s question and not this one's.
//!
//! **It asks nothing about a body.** The same limit [`crate::probe`] states: a
//! route that answers `200` with `{"error": "not implemented"}` passes here, and
//! a check that could tell the difference needs to know what the body should
//! contain, which is a contract this module does not read.

use std::fmt;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use sure_domain::evidence::{AnchorSubject, EvidenceAnchor, EvidenceClass};
use sure_domain::execution::ActionKind;
use sure_domain::ids::CheckId;
use sure_domain::severity::Severity;
use sure_domain::status::CheckResult;
use sure_domain::variants::variants;

use crate::checks::check_id;
use crate::discover::Discovery;
use crate::enforce::Enforcement;
use crate::probe::Limits as ProbeLimits;
use crate::probe::{Endpoint, Probe, ProbeOutcome};
use crate::references::{ReferenceOptions, is_source_candidate};
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason};

/// The HTTP methods a route can be declared for.
///
/// All seven, including the three SURE cannot send, because a vocabulary that
/// listed only what this build uses would make [`NotProbedBecause`] unable to
/// say *why* a route was not asked — and the reason is the part a reader needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RouteMethod {
    /// A read, and the one method [`crate::probe`] knows how to send.
    Get,
    /// A read SURE has no request line for.
    Head,
    /// A read SURE has no request line for.
    Options,
    /// A request that changes something.
    Post,
    /// A request that changes something.
    Put,
    /// A request that changes something.
    Patch,
    /// A request that changes something.
    Delete,
}

variants!(
    /// Every method, in the order a report reads them.
    RouteMethod { Get, Head, Options, Post, Put, Patch, Delete }
);

impl RouteMethod {
    /// The name the method is spelled with in a project's source, upper case.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }

    /// The name the method is spelled with in a route declaration, lower case.
    ///
    /// Flask, FastAPI, Express and axum all write the method as the name of the
    /// call that declares the route — `@app.get`, `app.post`, `.patch(` — and
    /// they all write it in lower case. This is the spelling a reader compares
    /// against, and it is the inverse of [`Self::from_call`] by construction:
    /// `every_method_round_trips_through_the_call_spelling` is what holds that.
    #[must_use]
    pub const fn as_call(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Head => "head",
            Self::Options => "options",
            Self::Post => "post",
            Self::Put => "put",
            Self::Patch => "patch",
            Self::Delete => "delete",
        }
    }

    /// The method a route declaration's call name names, if it names one.
    ///
    /// `None` for every other identifier, which is what keeps `router.use(` and
    /// `app.listen(` from being read as routes.
    #[must_use]
    pub fn from_call(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|method| method.as_call() == name)
    }

    /// Whether sending this method is a request that changes something.
    ///
    /// `GET`, `HEAD` and `OPTIONS` are the three methods HTTP defines as safe,
    /// and the answer is HTTP's rather than this product's. It is a *different*
    /// question from [`Self::is_the_one_sure_asks`], and the two are different
    /// fields on [`NotProbedBecause`] because they are different claims about a
    /// route that was not asked.
    #[must_use]
    pub const fn is_a_read(self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Options)
    }

    /// Whether this is the one method SURE's probe knows how to send.
    ///
    /// [`Get`](Self::Get), and the reason is [`crate::probe`]'s rather than a
    /// choice made here: [`Endpoint::request_line`] is `GET <path> HTTP/1.1` and
    /// it is the only request line that function has ever built. A build that
    /// taught it a second method would widen this answer, and
    /// `only_get_is_asked_and_the_reason_is_the_request_line` is what would have
    /// to be rewritten to notice.
    #[must_use]
    pub const fn is_the_one_sure_asks(self) -> bool {
        matches!(self, Self::Get)
    }
}

impl fmt::Display for RouteMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One route a project declares, as one line of one file states it.
///
/// **There is no constructor a caller can reach.** A `Route` is produced by
/// [`RouteReading::of`] from a line SURE read, so a route whose `declared_in` and
/// `line` do not lead to the characters it claims is not a value this module can
/// be handed. That is the whole of what makes the anchor meaningful: the same
/// reason [`crate::runtime_probes::RuntimeProbe`] has no `new`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    method: RouteMethod,
    path: String,
    declared_in: PathBuf,
    line: usize,
}

impl Route {
    /// The method the route is declared for.
    #[must_use]
    pub const fn method(&self) -> RouteMethod {
        self.method
    }

    /// The path, exactly as the declaring line spells it between its quotes.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The file the route was read from, relative to the project root.
    #[must_use]
    pub fn declared_in(&self) -> &Path {
        &self.declared_in
    }

    /// The line the route was read from, counting from one.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The method and path as one string, the way a report says them.
    ///
    /// **The path is escaped here rather than at each place a sentence is built,
    /// because every one of them comes through here**: a check's title, the reason
    /// [`crate::schedule`] prints under it, the anchor's locator, both
    /// descriptions and the tag [`Self::check_id`] digests. A path is the one
    /// string in a route that a project writes, and a sentence carrying a control
    /// byte out of it could erase itself as it was read. [`Self::path`] is the
    /// unescaped one, and it is what the request line is built from.
    #[must_use]
    pub fn spelling(&self) -> String {
        format!("{} {}", self.method, in_a_sentence(&self.path))
    }

    /// Where a reader goes to see the characters SURE read.
    ///
    /// [`AnchorSubject::LineRange`] rather than a subject invented for routes:
    /// the vocabulary has twelve subjects and none of them is a route, and the
    /// claim being anchored is *this is on line N of this file* — which is
    /// exactly what that subject says. The location is the file and the locator
    /// is the sentence naming the line, which is how
    /// [`crate::env_completeness`] builds the same kind of anchor.
    ///
    /// **Both fields are project text and both are escaped**, which is where this
    /// differs from the anchor named above: a file whose name carries a control
    /// byte would otherwise reach a report through the anchor without passing
    /// through a sentence first, and an anchor is printed. The escaped spelling is
    /// still a location a reader can follow — the escape names the byte rather
    /// than hiding it — and the other option is a report line a project can erase.
    #[must_use]
    pub fn anchor(&self) -> EvidenceAnchor {
        EvidenceAnchor::new(
            AnchorSubject::LineRange,
            in_a_sentence(&display_path(&self.declared_in)),
            format!(
                "the route `{}` declared on line {}",
                self.spelling(),
                self.line
            ),
        )
    }

    /// One line of plain language, for a report or a person.
    ///
    /// Both project strings in it are escaped — see [`in_a_sentence`] — for the
    /// reason the module's other sentences are: this is the line printed next to
    /// the check, and a project must not be able to end it early.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "`{}`, declared in {} on line {}.",
            self.spelling(),
            in_a_sentence(&display_path(&self.declared_in)),
            self.line
        )
    }

    /// The check this route is, named after the route and where it was read.
    ///
    /// The component is the declaring **file**, not the directory the way
    /// [`crate::runtime_probes`] uses a component: two routes on one path in two
    /// files are two different declarations with two different anchors, and an
    /// identifier that collapsed them would make a stored result join to the
    /// wrong one. The tag is the method and the path, and
    /// [`check_id`] keeps them apart with a digest even where its readable part
    /// collides — `/a-b` and `/ab` both spell `ab` after the filter.
    #[must_use]
    pub fn check_id(&self) -> CheckId {
        check_id(
            &display_path(&self.declared_in),
            &format!("route{}", self.spelling()),
        )
    }

    /// The endpoint this route is asked at, on the address SURE found.
    ///
    /// # Errors
    ///
    /// [`EndpointError::UnsafePath`](crate::probe::EndpointError::UnsafePath) if
    /// the path as the project wrote it cannot go into a request line. That is
    /// not a defect this module invents a fix for — percent-encoding a path a
    /// project may have encoded already changes what is asked for — and the
    /// route is reported [`NotProbedBecause::NotARequestTarget`] instead.
    pub fn endpoint(&self, address: SocketAddr) -> Result<Endpoint, crate::probe::EndpointError> {
        Endpoint::at(address.ip(), address.port(), self.path.clone())
    }

    /// Why this route is not one SURE may ask, if it is not.
    ///
    /// The three questions are asked in the order a reader would ask them: is
    /// this a request that changes something, is it a method SURE can send, and
    /// does the path name a request rather than a shape.
    fn because_not_asked(&self) -> Option<NotProbedBecause> {
        if !self.method.is_a_read() {
            return Some(NotProbedBecause::WouldChangeSomething(self.method));
        }
        if !self.method.is_the_one_sure_asks() {
            return Some(NotProbedBecause::NotTheReadSureAsks(self.method));
        }
        if has_a_slot(&self.path) {
            return Some(NotProbedBecause::HasASlot);
        }
        None
    }
}

/// Why [`Endpoint`] refused a route, as the reason a reader is given.
///
/// One function rather than the same `match` in the two places a route is turned
/// into an endpoint, because the whole point of the two variants is that the
/// sentence a reader gets is true of the refusal that happened, and two matches
/// are two places for one of them to start answering the other's sentence.
fn refusal(error: crate::probe::EndpointError) -> NotProbedBecause {
    match error {
        crate::probe::EndpointError::NotLoopback { address } => {
            NotProbedBecause::NotOnThisMachine { address }
        }
        crate::probe::EndpointError::UnsafePath { .. } => NotProbedBecause::NotARequestTarget,
    }
}

/// Project text, made safe to place in a sentence SURE writes.
///
/// **A route's path and the file it was read from are both project text, and both
/// go inside a sentence** rather than being handed to a renderer as a field,
/// because they are what a reader needs in order to find the thing SURE is talking
/// about. [`quoted`] returns the characters between a literal's quotes unchanged,
/// so a project whose source file holds a real control byte inside a route's
/// string reaches SURE's own output carrying it: `\u{1b}` written in a source file
/// is four characters and harmless, but the byte itself is one, and a lone carriage
/// return or an escape sequence such as an escape plus `[2K` can erase the line
/// the report is being printed on.
///
/// That is the class of defect this repository treats as the most serious one — a
/// false green in the terminal rather than in a verdict, because a failing route
/// could erase the report of its own failure while a person was reading it — and
/// it is the treatment [`crate::runtime_start`] and [`crate::setup`] already give
/// the project text they print, through the same helper.
///
/// [`Route::path`] is deliberately **not** escaped. It is what SURE asks for, and
/// a request line built from an escaped path would ask for something the project
/// does not serve; the escape belongs where text is composed into output, which is
/// what this function is for.
fn in_a_sentence(text: &str) -> String {
    crate::redact::escape_control_characters(text)
}

/// Whether a path names a shape rather than a request.
///
/// Three spellings, because the three stacks spell a parameter three ways:
/// `{item_id}` in FastAPI, axum and Express 5, `<int:item_id>` in Flask, and
/// `:item_id` in Express. **The third is matched on a whole segment** — a path
/// with a colon inside a segment, `/clock/12:30`, names a request and not a
/// shape, and a rule that looked for `:` anywhere would refuse it.
///
/// What each of the three has in common is that the project wrote a placeholder
/// where a value goes, and SURE has no value.
fn has_a_slot(path: &str) -> bool {
    path.contains('{')
        || path.contains('<')
        || path
            .split('/')
            .any(|segment| segment.starts_with(':') && segment.len() > 1)
}

/// Why a route SURE read is not asked.
///
/// **Every variant names what was seen.** A reader of a report gets this
/// sentence and nothing else, so *the receiver is not the application object* is
/// a claim about a line, and *SURE cannot see where this is mounted* would be a
/// claim about the absence of a line — which is the difference between an
/// observation and a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotProbedBecause {
    /// The route is declared for a method that would change something.
    WouldChangeSomething(RouteMethod),
    /// The route is a read, and not the read SURE's probe knows how to send.
    NotTheReadSureAsks(RouteMethod),
    /// The path has a slot in it and SURE has no value to put there.
    HasASlot,
    /// The route is declared on something that is not the application object.
    NotOnTheApplication {
        /// The name the route is declared on, as the line spells it.
        receiver: String,
    },
    /// The route is on a name this file mounts under a prefix.
    MountedUnder {
        /// The prefix the mounting call names.
        prefix: String,
    },
    /// A mounting call in this file names something SURE cannot read as a name.
    ///
    /// `nest("/api", make_router())` mounts a value, not a binding, and a
    /// reading that could not tell which name it moved has to assume it moved
    /// the one being asked about. **The whole file's Rust routes are withheld**,
    /// which is the safe direction: the cost is a route SURE did not check, and
    /// the alternative is a route SURE checked at a path the project does not
    /// serve.
    MountedInAWaySureCannotRead,
    /// The route is declared on a value no `let` in the file gives a name.
    OnSomethingSureCannotName,
    /// The path as the project wrote it cannot be put into a request line.
    NotARequestTarget,
    /// The address the caller named is not on this machine.
    ///
    /// **Separate from [`Self::NotARequestTarget`] because a reason has to be
    /// true of what it is given.** [`Route::endpoint`] refuses two different
    /// things — a path that cannot go into a request line and an address that is
    /// not loopback — and both are refusals, so collapsing them produces a
    /// sentence that claims *the path cannot be written into a request line* about
    /// a route whose path is perfectly writable. The address is carried because
    /// it is the caller's and the reader has to see which one was refused.
    NotOnThisMachine {
        /// The address that was refused.
        address: IpAddr,
    },
    /// The caller's budget ran out before SURE reached this route.
    BeyondTheBudget,
}

impl NotProbedBecause {
    /// One line of plain language, for someone who is not a programmer.
    ///
    /// **The whole sentence is escaped at this boundary, rather than in the arms
    /// that interpolate a project's text.** Two arms carry one today:
    /// [`Self::MountedUnder`]'s `prefix` and [`Self::NotOnTheApplication`]'s
    /// `receiver` are both read out of the project's own source — the mount call's
    /// literal and the name the route is declared on — and escaping those two by
    /// name is *exactly* the enumeration that missed them. Every other sentence in
    /// this module was escaped one call site at a time, and this one was passed
    /// over because the module had written down that it was SURE's own text. It is
    /// not, for those two arms, so the escape belongs where the sentence is
    /// composed and not where a project's text happens to be today.
    /// [`RouteCheck::of`] escapes at its own boundary for the same reason: one call
    /// covers every arm that exists and every arm added later.
    #[must_use]
    pub fn plain_description(&self) -> String {
        in_a_sentence(&match self {
            Self::WouldChangeSomething(method) => format!(
                "SURE did not ask this: a {method} request changes something, and checking \
                 whether a project works is not a reason to change it."
            ),
            Self::NotTheReadSureAsks(method) => format!(
                "SURE did not ask this: it is a {method} route, which is a read, and the \
                 probe this build has sends GET and nothing else."
            ),
            Self::HasASlot => "SURE did not ask this: the path has a placeholder in it where \
                               a value goes, and SURE has no value to put there."
                .to_owned(),
            Self::NotOnTheApplication { receiver } => format!(
                "SURE did not ask this: the route is declared on `{receiver}`, and only the \
                 application object's own routes are served at the path they name."
            ),
            Self::MountedUnder { prefix } => format!(
                "SURE did not ask this: this file mounts it under `{prefix}`, so the path it \
                 is served at is not the path it is declared with."
            ),
            Self::MountedInAWaySureCannotRead => {
                "SURE did not ask this: something in this file mounts a router built in \
                 place, so SURE cannot tell which routes moved."
                    .to_owned()
            }
            Self::OnSomethingSureCannotName => {
                "SURE did not ask this: the route is declared on a value the file never gives \
                 a name, so SURE cannot tell whether something later mounts it."
                    .to_owned()
            }
            Self::NotARequestTarget => {
                "SURE did not ask this: the path cannot be written into an HTTP request line \
                 as the project spells it."
                    .to_owned()
            }
            Self::NotOnThisMachine { address } => format!(
                "SURE did not ask this: {address} is not this machine, and a route check asks \
                 a service running here."
            ),
            Self::BeyondTheBudget => {
                "SURE did not ask this: it reached the number of routes this check was \
                 allowed to ask."
                    .to_owned()
            }
        })
    }
}

impl fmt::Display for NotProbedBecause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.plain_description())
    }
}

/// A route SURE read and did not ask, with the reason.
///
/// **This is not a skipped check result.** A result says *this check did not
/// run*, and there is no check here to have not run — the shape is
/// [`crate::runtime_probes::NotPlanned`]'s, which exists so that *SURE could not
/// do this* is a value a report shows rather than a row that is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotProbed {
    route: Route,
    because: NotProbedBecause,
}

impl NotProbed {
    /// The route that was read.
    #[must_use]
    pub const fn route(&self) -> &Route {
        &self.route
    }

    /// Why it is not asked.
    #[must_use]
    pub const fn because(&self) -> &NotProbedBecause {
        &self.because
    }

    /// One line: the route, and why.
    ///
    /// **This is the sentence a project most easily reaches**, since a route SURE
    /// will not ask is exactly where an odd path ends up: a path that cannot go
    /// into a request line is reported here by name, and the name is a project's.
    /// Both project strings are escaped — see [`in_a_sentence`] — and
    /// [`NotProbedBecause::plain_description`] is too — **and it is not all of
    /// SURE's own text**: two of its arms carry a name the project's source
    /// spelled, which is why that sentence is escaped at its own boundary rather
    /// than here.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "`{}` in {} on line {}: {}",
            self.route.spelling(),
            in_a_sentence(&display_path(&self.route.declared_in)),
            self.route.line,
            self.because
        )
    }
}

/// A route that is a check: the declaration, and the check SURE would run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCheck {
    route: Route,
    proposal: CheckProposal,
}

impl RouteCheck {
    /// The route the check is about.
    #[must_use]
    pub const fn route(&self) -> &Route {
        &self.route
    }

    /// The check, ready for a [`PlanBuilder`](crate::schedule::PlanBuilder).
    #[must_use]
    pub const fn proposal(&self) -> &CheckProposal {
        &self.proposal
    }

    /// One line: the route, and where it was read.
    #[must_use]
    pub fn plain_description(&self) -> String {
        self.route.plain_description()
    }
}

impl RouteCheck {
    /// The check a route becomes.
    ///
    /// **The weight is [`Severity::MustFix`], critical, and
    /// [`EvidenceClass::ObservedFact`]**, which is the row
    /// [`crate::runtime_probes`] gives a serve probe and the argument is the
    /// same one: SURE made the request and read what came back rather than
    /// computing an answer from the project state, so the evidence is observed.
    /// The severity is a claim about what a broken route means — a project that
    /// declares `GET /health` and does not answer it has a surface that does not
    /// work, and the report may not call that a detail. A caller that disagrees
    /// is disagreeing about the product's priorities and not about this
    /// function, which is why the two are written here rather than passed in.
    fn of(route: Route) -> Self {
        let proposal = CheckProposal::new(
            route.check_id(),
            format!("the route `{}` answers", route.spelling()),
            Severity::MustFix,
            true,
            EvidenceClass::ObservedFact,
            CheckReason::RouteDeclared {
                // Escaped **here** rather than in `schedule.rs`, because this is
                // where a project's text enters a structure another module turns
                // into a sentence: `CheckReason::plain_description` writes this
                // file into a line of its own, and `CheckReason::anchor` uses it
                // as an anchor's location. One escape at the boundary covers both,
                // and a second one there would be a second place to keep true.
                declared_in: in_a_sentence(&display_path(&route.declared_in)),
                route: route.spelling(),
                line: route.line,
            },
            &[ActionKind::LocalProbe],
        );
        Self { route, proposal }
    }
}

/// What SURE read out of a project's source: the routes that are checks, and the
/// routes that are not.
///
/// Built by [`Self::of`], which is the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteReading {
    checks: Vec<RouteCheck>,
    not_probed: Vec<NotProbed>,
    unread: Vec<PathBuf>,
}

impl RouteReading {
    /// Every route this project declares in a form SURE reads.
    ///
    /// The files are read in a fixed order — sorted by path, as
    /// [`crate::references`] sorts its candidates and for the same reason: which
    /// file is the one the budget stops at is a result, and it must not depend on
    /// the order a directory happened to come back in.
    ///
    /// **A file SURE could not open is in [`Self::unread`] rather than silently
    /// absent**, because an unread file is a place routes could be and a reading
    /// that counted it as a file with no routes would answer *this project
    /// declares none* from a file it never looked at.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        Self::with_options(discovery, &ReferenceOptions::default())
    }

    /// The same, under a caller's budget.
    #[must_use]
    pub fn with_options(discovery: &Discovery, options: &ReferenceOptions) -> Self {
        let mut reading = Self {
            checks: Vec::new(),
            not_probed: Vec::new(),
            unread: Vec::new(),
        };

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
            // **Joining is safe here because of what the scanner did, and the
            // reason is written down because it is one module away.** `scan`
            // answers a link with `SkipReason::NotFollowed` before the ignore
            // tables are consulted, so [`Discovery::scan`]'s file list holds no
            // symlink for this join to follow — and no component of `path` can be
            // `..`, because every one of them came from a directory entry name.
            // A candidate is therefore a real file inside the project, which is
            // what makes `fs::metadata` and `fs::read` below uninteresting: the
            // policy about links is the scanner's and is enforced once, rather
            // than re-decided per reader.
            let full = discovery.root.join(path);
            let Ok(metadata) = fs::metadata(&full) else {
                reading.unread.push(path.to_path_buf());
                continue;
            };
            if metadata.len() > options.max_file_bytes
                || files_read >= options.max_files
                || bytes_read.saturating_add(metadata.len()) > options.max_total_bytes
            {
                reading.unread.push(path.to_path_buf());
                continue;
            }
            let Ok(bytes) = fs::read(&full) else {
                reading.unread.push(path.to_path_buf());
                continue;
            };
            let Ok(text) = String::from_utf8(bytes) else {
                reading.unread.push(path.to_path_buf());
                continue;
            };

            files_read += 1;
            bytes_read = bytes_read.saturating_add(metadata.len());
            reading.read_file(path, &text);
        }

        reading
    }

    /// The routes that are checks, in the order the files were read.
    #[must_use]
    pub fn checks(&self) -> &[RouteCheck] {
        &self.checks
    }

    /// The routes that were read and are not checks, each with the reason.
    #[must_use]
    pub fn not_probed(&self) -> &[NotProbed] {
        &self.not_probed
    }

    /// The source files this reading did not read.
    ///
    /// A file is here when it was bigger than the budget allows in one file,
    /// when the file or byte budget ran out, when the operating system would not
    /// open it, or when its bytes are not UTF-8 text. **All four are the same
    /// fact to a reader of this list** — SURE did not look inside this file — and
    /// they are one list rather than four for the reason
    /// [`crate::scan::Scan::skipped`] gives about its own.
    #[must_use]
    pub fn unread(&self) -> &[PathBuf] {
        &self.unread
    }

    /// Whether every source file in the scan was read.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unread.is_empty()
    }

    /// Whether the reading found no routes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty() && self.not_probed.is_empty()
    }

    /// Read one file's routes, and file each one.
    fn read_file(&mut self, path: &Path, text: &str) {
        match Stack::of(path) {
            Some(Stack::Python) => self.read_python(path, text),
            Some(Stack::JavaScript) => self.read_javascript(path, text),
            Some(Stack::Rust) => self.read_rust(path, text),
            None => {}
        }
    }

    /// Add a route, as a check or as a route SURE read and will not ask.
    fn file(&mut self, route: Route) {
        match route.because_not_asked() {
            Some(because) => self.not_probed.push(NotProbed { route, because }),
            // The path is settled here rather than in the asking half, and the
            // port is a placeholder because the question is about the path: a
            // route whose path cannot go into a request line is refused once, in
            // the reading, where the anchor still says which line it came from.
            None => match Endpoint::loopback(0, route.path.clone()) {
                Ok(_) => self.checks.push(RouteCheck::of(route)),
                // Only the path can be refused here: the address is this module's
                // own `0.0.0.0`-free loopback placeholder, so `NotLoopback` is
                // named rather than reached, and it is named as the reason it is
                // rather than folded into the path's.
                Err(error) => self.not_probed.push(NotProbed {
                    route,
                    because: refusal(error),
                }),
            },
        }
    }

    /// Python: a decorator on a name this file binds to an application class.
    fn read_python(&mut self, path: &Path, text: &str) {
        let applications = bound_names(text, &["Flask(", "FastAPI("]);
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let Some(rest) = line.trim_start().strip_prefix('@') else {
                continue;
            };
            let Some((receiver, call, arguments)) = receiver_call(rest) else {
                continue;
            };
            if !applications.iter().any(|name| name == receiver) {
                continue;
            }
            let Some((literal, _quote, _end)) = quoted(arguments) else {
                continue;
            };
            if !literal.starts_with('/') {
                continue;
            }

            let methods = match call {
                "route" | "api_route" => methods_keyword(arguments),
                _ => RouteMethod::from_call(call).map(|method| vec![method]),
            };
            let Some(methods) = methods else {
                continue;
            };
            for method in methods {
                self.file(Route {
                    method,
                    path: literal.to_owned(),
                    declared_in: path.to_path_buf(),
                    line: line_number,
                });
            }
        }
    }

    /// JavaScript and TypeScript: a statement-initial call on the application.
    fn read_javascript(&mut self, path: &Path, text: &str) {
        let applications = bound_names(text, &["express("]);
        if applications.is_empty() {
            return;
        }
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let Some((receiver, call, arguments)) = receiver_call(line.trim_start()) else {
                continue;
            };
            let Some(method) = RouteMethod::from_call(call) else {
                continue;
            };
            if !applications.iter().any(|name| name == receiver) {
                continue;
            }
            let Some((literal, _quote, _end)) = quoted(arguments) else {
                continue;
            };
            if !literal.starts_with('/') {
                continue;
            }
            self.file(Route {
                method,
                path: literal.to_owned(),
                declared_in: path.to_path_buf(),
                line: line_number,
            });
        }
    }

    /// Rust: `.route("<path>", <verb>(<handler>))` on a chain a `let` names.
    fn read_rust(&mut self, path: &Path, text: &str) {
        let mounts = rust_mounts(text);
        let mut chain: Option<String> = None;

        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            let bound = let_binding(trimmed).map(str::to_owned);

            for (receiver, method, literal) in
                rust_routes_on(trimmed, chain.as_deref(), bound.as_deref())
            {
                let name = receiver.clone().unwrap_or_default();
                let because = if receiver.is_none() {
                    Some(NotProbedBecause::OnSomethingSureCannotName)
                } else if mounts.unreadable {
                    Some(NotProbedBecause::MountedInAWaySureCannotRead)
                } else {
                    mounts
                        .under
                        .iter()
                        .find(|(mounted, _prefix)| mounted == &name)
                        .map(|(_mounted, prefix)| NotProbedBecause::MountedUnder {
                            prefix: prefix.clone(),
                        })
                };

                let route = Route {
                    method,
                    path: literal,
                    declared_in: path.to_path_buf(),
                    line: line_number,
                };
                match because {
                    Some(because) => self.not_probed.push(NotProbed { route, because }),
                    None => self.file(route),
                }
            }

            // The name the *next* line's chain belongs to. A `;` ends whatever
            // statement was open, a line that binds a name whose expression runs
            // past its own end hands that name forward, and anything else — a
            // continuation line, a comment between two calls — leaves it alone.
            if trimmed.ends_with(';') {
                chain = None;
            } else if let Some(name) = bound {
                chain = Some(name);
            }
        }
    }
}

/// Which family of source a file belongs to, by extension.
///
/// The extensions are [`crate::references::is_source_candidate`]'s, read through
/// that predicate rather than restated — the two modules must agree about which
/// files are source, or one of them would read a file the other calls a
/// document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stack {
    Python,
    JavaScript,
    Rust,
}

impl Stack {
    /// The stack a path's extension names, if it names one.
    fn of(path: &Path) -> Option<Self> {
        if !is_source_candidate(path) {
            return None;
        }
        let extension = path.extension()?.to_string_lossy().to_lowercase();
        match extension.as_str() {
            "py" | "pyi" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            _ => Some(Self::JavaScript),
        }
    }
}

/// The names a text binds, on any line, to one of these constructions.
///
/// `app = FastAPI()`, `const app = express()`, `let app = Router::new()` — three
/// spellings of one act, and each is a name on the left of a single `=` and one
/// of `constructors` at the start of everything on the right. **`==` and `=>`
/// cannot be mistaken for it**: the left side of the first `=` in `a == b` is
/// `a `, which is not a name once trimmed only at the ends, and the right side of
/// `a =` in `a => b` starts with `>`.
///
/// A name bound twice is one name: the reading wants the set, not the history.
fn bound_names(text: &str, constructors: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let Some(name) = let_binding(line) else {
            continue;
        };
        let Some(after) = line
            .split_once('=')
            .map(|(_left, right)| right.trim_start())
        else {
            continue;
        };
        if constructors
            .iter()
            .any(|constructor| after.starts_with(constructor))
        {
            names.push(name.to_owned());
        }
    }
    names
}

/// The name a line binds on the left of its first `=`, if it binds one.
///
/// `const app`, `let app`, `var app` and a bare Python or Rust `app` all reduce to
/// `app`, by dropping a declaration keyword if there is one and refusing
/// anything left that is not a plain identifier. The refusal is what keeps
/// `if a` and `return x` out.
///
/// **A comparison is not a binding, and the two guards are what tell them
/// apart.** `a == b` splits at its first `=` into `a ` and `= b`, and the right
/// side opening with `=` is the operator rather than an assignment; `a >= b`
/// and `a != b` leave the operator's first character on the left, where it is
/// not an identifier character. Without them this function answers `a` for
/// `if a == b {` — a name that was compared and never bound, which the mount
/// rule would then treat as a router.
fn let_binding(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let trimmed = trimmed
        .strip_prefix("const ")
        .or_else(|| trimmed.strip_prefix("let "))
        .or_else(|| trimmed.strip_prefix("var "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);
    let (name, right) = trimmed.split_once('=')?;
    // `x => y` is a closure or an arrow function, and the `>` is the operator.
    if right.starts_with('=') || right.starts_with('>') {
        return None;
    }
    let name = name.trim();
    if name.is_empty() || !name.chars().all(identifier_character) {
        return None;
    }
    if !name
        .chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_' || first == '$')
    {
        return None;
    }
    Some(name)
}

/// Whether a character may appear in the identifier spellings these four
/// languages share.
///
/// Deliberately the intersection rather than the union: a name SURE reads here
/// is compared against a name a project wrote, and a rule that accepted more
/// than one language allows would accept a string that is not a name in any of
/// them.
fn identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// `<receiver>.<call>(` at the start of a line, and the text of the arguments.
///
/// **Statement-initial, and that is the whole of its strictness.** A line that
/// begins with anything else is not read, which keeps `cache.get("/health")`
/// inside a larger expression from being read as a route declaration — and the
/// receiver test one level up is what keeps a `Map` with a path-shaped key out.
fn receiver_call(line: &str) -> Option<(&str, &str, &str)> {
    let receiver_end = line
        .char_indices()
        .take_while(|(_index, character)| identifier_character(*character))
        .count();
    if receiver_end == 0 {
        return None;
    }
    let receiver = &line[..receiver_end];
    let after_receiver = &line[receiver_end..];
    let after_dot = after_receiver.strip_prefix('.')?;
    let call_end = after_dot
        .char_indices()
        .take_while(|(_index, character)| identifier_character(*character))
        .count();
    if call_end == 0 {
        return None;
    }
    let call = &after_dot[..call_end];
    let arguments = after_dot[call_end..].strip_prefix('(')?;
    Some((receiver, call, arguments))
}

/// The first quoted literal on a line: its contents, its quote, and where it
/// ends.
///
/// **The character after a backslash is skipped**, so a literal containing an
/// escaped quote does not end early. The returned end index is one past the
/// closing quote and is always a character boundary: it is either the end of the
/// text or the index of an ASCII quote byte, and the slice starts just past one.
fn quoted(line: &str) -> Option<(&str, char, usize)> {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let opening = bytes[index];
        if opening == b'"' || opening == b'\'' {
            let start = index + 1;
            let mut at = start;
            while at < bytes.len() {
                if bytes[at] == b'\\' {
                    at += 2;
                    continue;
                }
                if bytes[at] == opening {
                    return Some((&line[start..at], opening as char, at + 1));
                }
                at += 1;
            }
            return None;
        }
        index += 1;
    }
    None
}

/// The methods a `route(` call's `methods=[...]` keyword names.
///
/// **A call with no `methods` keyword is `GET` and not *unknown***, because that
/// is what the frameworks this reads document the spelling to mean: Flask's
/// `route` defaults to `["GET"]` and FastAPI's `api_route` requires the keyword,
/// so the default is the fact. A keyword present but naming nothing SURE reads as
/// a method yields `None`, which withholds the route rather than guessing at
/// `GET`.
fn methods_keyword(arguments: &str) -> Option<Vec<RouteMethod>> {
    let Some(after) = arguments.split_once("methods") else {
        return Some(vec![RouteMethod::Get]);
    };
    let list = after.1.split_once('[').map(|(_before, list)| list)?;
    let list = list.split_once(']').map(|(list, _after)| list)?;

    let mut methods = Vec::new();
    let mut rest = list;
    while let Some((literal, _quote, end)) = quoted(rest) {
        let method = RouteMethod::ALL
            .iter()
            .copied()
            .find(|method| method.as_str() == literal.trim().to_uppercase())?;
        methods.push(method);
        rest = &rest[end..];
    }
    if methods.is_empty() {
        None
    } else {
        Some(methods)
    }
}

/// The router names a Rust file mounts, and whether any mount was unreadable.
#[derive(Debug, Default)]
struct RustMounts {
    under: Vec<(String, String)>,
    unreadable: bool,
}

/// Every `nest` and `merge` call in a Rust file, and what each one mounts.
///
/// `nest("<prefix>", name)` mounts `name` under `<prefix>`, and `merge(name)`
/// mounts `name` at the root — so a `merge` is a mount with an empty prefix and
/// is recorded as one rather than special-cased, because the point is that the
/// name moved and not where it went.
fn rust_mounts(text: &str) -> RustMounts {
    let mut mounts = RustMounts::default();
    for line in text.lines() {
        if let Some(after) = line.split_once("nest(").map(|(_before, after)| after) {
            match quoted(after) {
                Some((prefix, _quote, end)) => {
                    let rest = after[end..].trim_start();
                    match rest.strip_prefix(',') {
                        Some(rest) => match plain_name(rest) {
                            Some(name) => mounts.under.push((name, prefix.to_owned())),
                            None => mounts.unreadable = true,
                        },
                        None => mounts.unreadable = true,
                    }
                }
                None => mounts.unreadable = true,
            }
        }
        if let Some(after) = line.split_once("merge(").map(|(_before, after)| after) {
            match plain_name(after) {
                Some(name) => mounts.under.push((name, String::new())),
                None => mounts.unreadable = true,
            }
        }
    }
    mounts
}

/// The identifier a fragment is, with the punctuation a call site puts after it
/// removed.
///
/// `api);` is `api`, because a call and a statement both have to end somewhere
/// and neither the closing parenthesis nor the semicolon is part of the name.
/// **Stripping is a loop rather than one `strip_suffix`**, since a `merge(name)`
/// argument is followed by both, and a rule that removed one of the two would
/// read a mount as unreadable depending on which call it was written in.
///
/// What survives the strip still has to be a plain identifier, so
/// `make_router()` — which leaves `make_router(` — and `api.clone()` are both
/// `None`, and the caller treats that as an unreadable mount rather than as a
/// name. The `(` that the stripping cannot remove is exactly what makes a call
/// distinguishable from a name.
fn plain_name(fragment: &str) -> Option<String> {
    let mut trimmed = fragment.trim();
    while let Some(shorter) = trimmed
        .strip_suffix(')')
        .or_else(|| trimmed.strip_suffix(';'))
    {
        trimmed = shorter.trim_end();
    }
    if trimmed.is_empty() || !trimmed.chars().all(identifier_character) {
        return None;
    }
    if !trimmed
        .chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
    {
        return None;
    }
    Some(trimmed.to_owned())
}

/// Every Rust route one line declares, and the name each one is on.
///
/// **Every `.route(` on the line, not the first**, because a chain can declare
/// several: `Router::new().route(a, …).route(b, …)` is two routes and reading one
/// of them would be a reading that lost a route without saying so.
///
/// The name an occurrence belongs to is decided by what precedes it, and the four
/// cases are the four ways axum is written:
///
/// ```text
/// app.route("/health", get(health));                     // the name is on the line
/// .route("/health", get(health));                        // the chain's name
/// let app = Router::new()
///     .route("/health", get(health));                    // the `let`'s name
/// Router::new().route("/health", get(health))            // no name at all
/// ```
///
/// The last is `None` rather than an empty string, so that the caller can tell
/// *this route has no name* from *this route has a name that happens to be
/// empty* — the second is not a thing a project can write, and a reading that
/// spelled both the same way would make the reason it reports a guess.
fn rust_routes_on(
    line: &str,
    chain: Option<&str>,
    bound: Option<&str>,
) -> Vec<(Option<String>, RouteMethod, String)> {
    let mut found = Vec::new();
    let mut searched = 0;
    while let Some(at) = line[searched..].find(".route(").map(|at| at + searched) {
        searched = at + ".route(".len();
        let arguments = &line[searched..];
        let (Some(method), Some(path)) = (rust_method(arguments), rust_path(arguments)) else {
            continue;
        };

        let before = &line[..at];
        let receiver = if before.is_empty() {
            // The line opens with the call, so it continues a chain — the name
            // is the one an earlier line established.
            chain.map(str::to_owned)
        } else if is_an_identifier(before) {
            Some(before.to_owned())
        } else {
            // `let app = Router::new().route(…` and `.route(a).route(b)` both
            // land here. The first has the name on its own line and the second
            // has it on an earlier one, and `chain` is empty for the first
            // because the statement has only just started.
            chain.or(bound).map(str::to_owned)
        };
        found.push((receiver, method, path));
    }
    found
}

/// Whether a fragment is exactly one identifier.
fn is_an_identifier(fragment: &str) -> bool {
    !fragment.is_empty()
        && fragment.chars().all(identifier_character)
        && fragment
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_' || first == '$')
}

/// The path in a `.route(` argument list.
fn rust_path(arguments: &str) -> Option<String> {
    let (literal, _quote, _end) = quoted(arguments)?;
    literal.starts_with('/').then(|| literal.to_owned())
}

/// The method in a `.route(path, <verb>(handler))` argument list.
///
/// The verb is the second argument's call name, which is how axum spells it —
/// `get(handler)`, `post(handler)` — and the first argument is skipped by
/// walking past its quoted literal rather than by splitting on a comma, so a
/// path containing a comma does not move the second argument.
fn rust_method(arguments: &str) -> Option<RouteMethod> {
    let (_literal, _quote, end) = quoted(arguments)?;
    let rest = arguments[end..].trim_start().strip_prefix(',')?;
    let rest = rest.trim_start();
    let call_end = rest
        .char_indices()
        .take_while(|(_index, character)| identifier_character(*character))
        .count();
    if call_end == 0 {
        return None;
    }
    RouteMethod::from_call(&rest[..call_end])
}

/// What one route smoke is allowed to spend.
///
/// Two budgets: [`probe::Limits`](crate::probe::Limits) for each request the way
/// [`crate::runtime_start::Limits`] carries one, and a count for how many routes
/// are asked at all. A route list is a project's, and a project may declare ten
/// thousand of them; the count is what makes *SURE checked your routes* a
/// bounded promise rather than one that grows with a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    request: ProbeLimits,
    most_routes: usize,
}

impl Limits {
    /// Bounds for one route smoke.
    ///
    /// # Errors
    ///
    /// [`LimitsError::ZeroRoutes`] if `most_routes` is zero, and it is a refusal
    /// rather than a round-up for the reason
    /// [`crate::runtime_start::LimitsError`] gives about its window: a check
    /// allowed to ask nothing is a check that can never answer its question, and
    /// a caller that asked for one has made a mistake a constructor can catch
    /// once instead of a report explaining it later.
    pub fn new(request: ProbeLimits, most_routes: usize) -> Result<Self, LimitsError> {
        if most_routes == 0 {
            return Err(LimitsError::ZeroRoutes);
        }
        Ok(Self {
            request,
            most_routes,
        })
    }

    /// What each request may spend.
    #[must_use]
    pub const fn request(&self) -> ProbeLimits {
        self.request
    }

    /// How many routes may be asked.
    #[must_use]
    pub const fn most_routes(&self) -> usize {
        self.most_routes
    }
}

/// Why a route smoke's bounds were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitsError {
    /// The caller allowed no route to be asked.
    ZeroRoutes,
}

impl fmt::Display for LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRoutes => formatter
                .write_str("a route smoke allowed to ask nothing is a check that can never answer"),
        }
    }
}

impl std::error::Error for LimitsError {}

/// Asking a running service about the routes its project declares.
///
/// Built by [`Self::of`], which is the only constructor: the address comes from
/// the caller, the routes come from the reading, and the fingerprint comes from
/// the enforcement — so a smoke assembled from parts that disagree is not a value
/// this module can be handed.
#[derive(Debug)]
pub struct RouteSmoke<'a> {
    address: SocketAddr,
    probe: Probe,
    limits: Limits,
    enforcement: &'a Enforcement,
    asked: Vec<Route>,
    not_probed: Vec<NotProbed>,
}

impl<'a> RouteSmoke<'a> {
    /// A route smoke over `reading`, asking the service at `address`.
    ///
    /// **`address` is the caller's and not this module's**, because a port is a
    /// fact about a running service and not about a route: a `dev` script that
    /// prints *listening on 5173* knows something no manifest writes down, and
    /// [`crate::runtime_start::Limits`] is where the running half of this product
    /// already carries it. A caller with no address has nothing for this
    /// constructor to take, which is the same refusal
    /// [`crate::runtime_start`] makes when it reports a warning instead of
    /// probing a port it does not know.
    ///
    /// The routes SURE will not ask keep their reasons, and the budget's own
    /// refusals join them: **[`Self::not_probed`] is the reading's list plus
    /// whatever the budget left out**, so one accessor answers *what did SURE not
    /// look at* for the whole smoke.
    #[must_use]
    pub fn of(
        reading: &RouteReading,
        address: SocketAddr,
        enforcement: &'a Enforcement,
        limits: Limits,
    ) -> Self {
        let mut asked = Vec::new();
        let mut not_probed = reading.not_probed.clone();

        for check in reading.checks() {
            let route = check.route().clone();
            if asked.len() >= limits.most_routes {
                not_probed.push(NotProbed {
                    route,
                    because: NotProbedBecause::BeyondTheBudget,
                });
                continue;
            }
            match route.endpoint(address) {
                Ok(_) => asked.push(route),
                Err(error) => not_probed.push(NotProbed {
                    route,
                    because: refusal(error),
                }),
            }
        }

        Self {
            address,
            probe: Probe::new(limits.request),
            limits,
            enforcement,
            asked,
            not_probed,
        }
    }

    /// The address the routes are asked at.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// The bounds this smoke works under.
    ///
    /// The limits the caller gave, and not [`Self::asked`]'s length: a smoke
    /// allowed eight routes that the reading offered three of is still a smoke
    /// allowed eight, and a report that read the count back would say the budget
    /// was three and then wonder why a fourth route was [`NotProbed`].
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// The routes SURE will ask, in the reading's order.
    #[must_use]
    pub fn asked(&self) -> &[Route] {
        &self.asked
    }

    /// The routes SURE will not ask, each with the reason.
    #[must_use]
    pub fn not_probed(&self) -> &[NotProbed] {
        &self.not_probed
    }

    /// Ask every route, and report what came back.
    ///
    /// **Infallible, and [`crate::runtime_start::StartSmoke::run`] gives the
    /// reason**: every way an exchange can end is an outcome about the project
    /// rather than an error in the checker, so the one thing a caller could do
    /// with a `Result` — drop the answer it did not expect — is not available.
    ///
    /// One result per asked route, each carrying the fingerprint
    /// [`Enforcement::check_plan`] holds. The order is the reading's, and nothing
    /// downstream should depend on it: [`CheckSchedule`](crate::schedule) sorts.
    #[must_use]
    pub fn run(&self) -> Vec<CheckResult> {
        let fingerprint = self.enforcement.check_plan().fingerprint.clone();
        let mut results = Vec::with_capacity(self.asked.len());

        for route in &self.asked {
            let Ok(endpoint) = route.endpoint(self.address) else {
                // Refused at construction, so this cannot happen; a route that
                // reached the asked list and cannot be turned into an endpoint
                // now would be a change to `Endpoint`'s rule rather than a state
                // to report, and reporting it as a skipped check would be the
                // one thing this module must not do — a row that reads as *SURE
                // chose not to ask*.
                continue;
            };
            let outcome: ProbeOutcome = self.probe.get(&endpoint);
            results.push(outcome.verdict(
                route.check_id(),
                &endpoint,
                Severity::MustFix,
                true,
                fingerprint.clone(),
            ));
        }

        results
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_method_round_trips_through_the_call_spelling() {
        for method in RouteMethod::ALL {
            assert_eq!(
                RouteMethod::from_call(method.as_call()),
                Some(*method),
                "{method} did not survive its own call spelling"
            );
        }
        assert_eq!(RouteMethod::from_call("use"), None);
        assert_eq!(RouteMethod::from_call(""), None);
        assert_eq!(RouteMethod::from_call("Get"), None);
    }

    #[test]
    fn only_get_is_asked_and_the_reason_is_the_request_line() {
        // Read out of the module that builds the line rather than restated: if
        // `probe.rs` ever learns a second method, this is where the answer has
        // to change, and it will fail rather than quietly disagree.
        let endpoint = Endpoint::loopback(1, "/health").unwrap();
        assert_eq!(endpoint.request_line(), "GET /health HTTP/1.1");
        assert!(RouteMethod::Get.is_the_one_sure_asks());
        for method in RouteMethod::ALL {
            if *method == RouteMethod::Get {
                continue;
            }
            assert!(
                !method.is_the_one_sure_asks(),
                "{method} is not the read SURE can send"
            );
        }
        // And the two questions really are different questions: HEAD is a read
        // this build cannot send, and POST is not a read at all.
        assert!(RouteMethod::Head.is_a_read());
        assert!(!RouteMethod::Head.is_the_one_sure_asks());
        assert!(!RouteMethod::Post.is_a_read());
    }

    #[test]
    fn a_slot_is_a_shape_and_a_colon_inside_a_segment_is_not() {
        for path in ["/items/{item_id}", "/items/<int:item_id>", "/items/:id"] {
            assert!(has_a_slot(path), "{path} names a shape");
        }
        for path in ["/health", "/clock/12:30", "/a:b/c", "/items/"] {
            assert!(!has_a_slot(path), "{path} names a request");
        }
    }

    #[test]
    fn a_control_byte_in_a_projects_text_cannot_reach_a_sentence_sure_writes() {
        // A real escape byte, and not the four characters `\u{1b}`: what a source
        // file can hold between a literal's quotes is the byte, and the byte is
        // what `quoted` hands back. `ESC [ 2K` clears the line it is written on, so
        // a project spelling a route this way could erase SURE's own report of that
        // route while somebody was reading the report.
        let route = Route {
            method: RouteMethod::Get,
            path: "/x\u{1b}[2Ky".to_owned(),
            declared_in: PathBuf::from("src/app\u{1b}[2K.js"),
            line: 4,
        };
        let check = RouteCheck::of(route.clone());
        let anchor = route.anchor();
        // The sentence a route SURE will not ask is given with the reason this
        // route actually reaches: `Endpoint::at` refuses a path carrying a byte
        // that cannot be written into a request line, and `NotARequestTarget` is
        // what the reading reports for it. **It is built here rather than read off
        // a workspace because a file name cannot hold this byte on Windows** —
        // a route whose *path* carries one is reachable through the filesystem,
        // and one whose *file* carries one is not — so the file-name half of the
        // escaping has this test and no integration test.
        let not_probed = NotProbed {
            route: route.clone(),
            because: NotProbedBecause::NotARequestTarget,
        };

        for (what, sentence) in [
            ("the spelling", route.spelling()),
            ("the description", route.plain_description()),
            ("the check's title", check.proposal().title().to_owned()),
            (
                "the reason a report prints",
                check.proposal().reason().plain_description(),
            ),
            ("the anchor's location", anchor.location.clone()),
            ("the anchor's locator", anchor.locator.clone()),
            (
                "the sentence for a route SURE will not ask",
                not_probed.plain_description(),
            ),
        ] {
            assert!(
                !sentence.chars().any(char::is_control),
                "{what} carries a byte out of the project: {sentence:?}"
            );
        }

        // Escaped rather than dropped, so a reader is told which byte was there,
        // and the path SURE would ask for is still the project's own spelling of
        // it: escaping is for what a reader sees and never for what is sent.
        assert!(
            route.spelling().contains("\\u{001b}"),
            "{}",
            route.spelling()
        );
        assert!(
            route.plain_description().contains("src/app\\u{001b}[2K.js"),
            "{}",
            route.plain_description()
        );
        assert_eq!(route.path(), "/x\u{1b}[2Ky");
    }

    #[test]
    fn a_name_the_project_wrote_is_escaped_in_the_sentence_that_repeats_it() {
        // **The two arms of `NotProbedBecause` that carry the project's own text,
        // and the reason they need a test of their own.** The doc above that impl
        // used to say its sentence was SURE's own text, so it was the one sentence
        // in this module with no escape on it -- and the `NotARequestTarget` case
        // the invariant test builds carries no project text at all, so the loop it
        // runs could not tell an escaped boundary from an unescaped one. `receiver`
        // is the name a route is declared on and `prefix` is the literal of a mount
        // call: both are read out of a source file, which makes both the project's
        // to spell and neither of them SURE's.
        //
        // The byte is real and not the four characters `ESC`, for the reason the
        // sibling test gives: what a source file holds between a literal's quotes
        // is the byte, `quoted` hands it back unchanged, and `ESC [ 2K` clears the
        // line it is written on -- so an arm that passed it through would let a
        // project erase SURE's own account of what SURE did not check.
        let injected = "a\u{1b}[2Kb";
        for (what, because) in [
            (
                "the name a route is declared on",
                NotProbedBecause::NotOnTheApplication {
                    receiver: injected.to_owned(),
                },
            ),
            (
                "the prefix a mount call names",
                NotProbedBecause::MountedUnder {
                    prefix: injected.to_owned(),
                },
            ),
        ] {
            let sentence = because.plain_description();
            assert!(
                !sentence.chars().any(char::is_control),
                "{what} carries a byte out of the project: {sentence:?}"
            );
            // Escaped rather than dropped, which is the half a boundary that
            // sanitised by deleting would still pass: a reader has to be able to
            // see which byte the project actually wrote, and a sentence that
            // removed it would satisfy the assertion above while telling the
            // reader less than the file says.
            assert!(
                sentence.contains("a\\u{001b}[2Kb"),
                "{what} lost the project's spelling instead of escaping it: {sentence}"
            );
        }
    }

    #[test]
    fn a_quoted_literal_is_read_with_its_escaping_and_reports_where_it_ends() {
        assert_eq!(
            quoted("x = '/health'").map(|(s, q, _e)| (s, q)),
            Some(("/health", '\''))
        );
        assert_eq!(
            quoted("x = \"/a\\\"b\"").map(|(s, _q, _e)| s),
            Some("/a\\\"b")
        );
        assert_eq!(quoted("no quotes here"), None);
        assert_eq!(quoted("x = 'unterminated"), None);
        // The end index is one past the closing quote, and it is what the Rust
        // argument reader walks to find the second argument. **The slice is
        // taken from the same `source` that was passed in**, not from a second
        // literal spelling the same characters: two spellings can disagree
        // about an escape, and then this test would be measuring the difference
        // between them instead of the index it is about.
        let source = "\"/a,b\", get(h)";
        let (literal, _quote, end) = quoted(source).unwrap();
        assert_eq!(literal, "/a,b");
        assert_eq!(&source[end..], ", get(h)");
    }

    #[test]
    fn a_binding_is_a_name_on_the_left_of_one_equals_sign() {
        assert_eq!(let_binding("app = FastAPI()"), Some("app"));
        assert_eq!(let_binding("const app = express()"), Some("app"));
        assert_eq!(let_binding("    let api = Router::new()"), Some("api"));
        assert_eq!(let_binding("pub app = 1"), Some("app"));
        assert_eq!(let_binding("a == b"), None);
        assert_eq!(let_binding("const a = b = c"), Some("a"));
        assert_eq!(let_binding("1app = x"), None);
        assert_eq!(let_binding("no equals here"), None);
        // A comparison binds nothing, and both operator shapes are here because
        // they are refused by different guards: `==` leaves the operator on the
        // right of the split and `>=` leaves its first character on the left.
        assert_eq!(let_binding("x => y"), None);
        assert_eq!(let_binding("a >= b"), None);
        assert_eq!(let_binding("a != b"), None);
        assert_eq!(let_binding("total += 1"), None);
    }

    #[test]
    fn a_receiver_and_call_are_read_only_at_the_start_of_a_line() {
        assert_eq!(
            receiver_call("app.get(\"/health\")"),
            Some(("app", "get", "\"/health\")"))
        );
        assert_eq!(receiver_call("  app.get("), None);
        assert_eq!(receiver_call("cache.get("), Some(("cache", "get", "")));
        assert_eq!(
            receiver_call("app.listen(3000)"),
            Some(("app", "listen", "3000)"))
        );
        assert_eq!(receiver_call(".route("), None);
        assert_eq!(receiver_call("app = 1"), None);
    }

    #[test]
    fn a_default_method_list_is_get_and_an_unreadable_one_is_not_guessed() {
        assert_eq!(
            methods_keyword("\"/x\")"),
            Some(vec![RouteMethod::Get]),
            "a route with no methods keyword is the framework's documented GET"
        );
        assert_eq!(
            methods_keyword("\"/x\", methods=[\"POST\", \"PUT\"])"),
            Some(vec![RouteMethod::Post, RouteMethod::Put])
        );
        assert_eq!(
            methods_keyword("\"/x\", methods=[\"get\"])"),
            Some(vec![RouteMethod::Get]),
            "the spelling is case-insensitive in the frameworks this reads"
        );
        assert_eq!(methods_keyword("\"/x\", methods=[\"BREW\"])"), None);
        assert_eq!(methods_keyword("\"/x\", methods=[])"), None);
    }

    #[test]
    fn rust_mounts_name_what_moved_and_refuse_what_they_cannot_read() {
        let mounts = rust_mounts("let app = Router::new().nest(\"/api\", api);");
        assert!(!mounts.unreadable);
        assert_eq!(mounts.under, vec![("api".to_owned(), "/api".to_owned())]);

        let mounts = rust_mounts("let app = Router::new().merge(other);");
        assert!(!mounts.unreadable);
        assert_eq!(mounts.under, vec![("other".to_owned(), String::new())]);

        let mounts = rust_mounts("let app = Router::new().nest(\"/api\", make_router());");
        assert!(
            mounts.unreadable,
            "a mounted value is not a name SURE can follow"
        );

        let mounts = rust_mounts("let app = Router::new().nest(path, api);");
        assert!(
            mounts.unreadable,
            "a prefix SURE cannot read is a mount it cannot follow"
        );
    }

    #[test]
    fn a_rust_route_is_read_from_both_shapes_and_from_neither_without_a_name() {
        assert_eq!(
            rust_routes_on("app.route(\"/health\", get(health));", None, None),
            vec![(
                Some("app".to_owned()),
                RouteMethod::Get,
                "/health".to_owned()
            )]
        );
        assert_eq!(
            rust_routes_on(".route(\"/a,b\", post(h));", Some("api"), None),
            vec![(Some("api".to_owned()), RouteMethod::Post, "/a,b".to_owned())]
        );
        assert_eq!(
            rust_routes_on(".route(\"/h\", get(h));", None, None),
            vec![(None, RouteMethod::Get, "/h".to_owned())],
            "a chain SURE cannot name is still a route it read, and `None` is how \
             it says so rather than a name that is empty"
        );
        assert_eq!(
            rust_routes_on(
                "let app = Router::new().route(\"/h\", get(h));",
                None,
                Some("app")
            ),
            vec![(Some("app".to_owned()), RouteMethod::Get, "/h".to_owned())]
        );
        assert_eq!(
            rust_routes_on(
                ".route(\"/a\", get(a)).route(\"/b\", get(b));",
                Some("app"),
                None
            ),
            vec![
                (Some("app".to_owned()), RouteMethod::Get, "/a".to_owned()),
                (Some("app".to_owned()), RouteMethod::Get, "/b".to_owned()),
            ],
            "one line can declare two routes, and a reading that took the first \
             would lose the second without saying so"
        );
        assert_eq!(
            rust_routes_on("let app = Router::new();", None, Some("app")),
            vec![]
        );
        // **The one input where both names are known, and the order they are
        // consulted in is the claim.** A statement that runs past the end of its
        // line — a missing `;`, which is a thing an AI writing Rust produces —
        // leaves a chain open, and the next line's `let` binds a second name. The
        // receiver is the chain's, because `app` is what the call is *on* and
        // `api` is only what the expression *becomes*: reading `bound` first
        // would attribute the route to `api`, so a file that nests `app` under a
        // prefix would have SURE ask `/h` for a route the project serves at
        // `/api/h` — the one mistake the table in this file's module docs exists
        // to prevent. The answer is asserted here rather than left to whichever
        // name happens to be set.
        assert_eq!(
            rust_routes_on(
                "let api = app.route(\"/h\", get(h));",
                Some("app"),
                Some("api")
            ),
            vec![(Some("app".to_owned()), RouteMethod::Get, "/h".to_owned())]
        );
        assert_eq!(
            rust_routes_on(".route(\"relative\", get(h));", Some("a"), None),
            vec![]
        );
        assert_eq!(
            rust_routes_on(".route(\"/h\", on(ANY, h));", Some("a"), None),
            vec![]
        );
        assert_eq!(
            rust_routes_on("router.route(\"/h\", get(h));", None, None),
            vec![(Some("router".to_owned()), RouteMethod::Get, "/h".to_owned())],
            "the name is on the line and the chain is not consulted"
        );
    }

    #[test]
    fn a_stack_is_named_by_extension_and_only_for_source_files() {
        assert_eq!(Stack::of(Path::new("a/b.py")), Some(Stack::Python));
        assert_eq!(Stack::of(Path::new("a/b.mts")), Some(Stack::JavaScript));
        assert_eq!(Stack::of(Path::new("a/b.rs")), Some(Stack::Rust));
        assert_eq!(Stack::of(Path::new("a/b.md")), None);
        assert_eq!(Stack::of(Path::new("a/b")), None);
    }
}
