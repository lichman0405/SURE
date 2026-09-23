//! The local services a project declares, as typed declarations rather than
//! command lines.
//!
//! `checks.services` is the one setting in `sure.yaml` that asks SURE to start
//! something, and the whole of this module is the shape it may be written in. A
//! declaration is **narrower than a command**: it names a launcher SURE knows,
//! a port on loopback and a path that answers once the service is up, and there
//! is no field anywhere in it that accepts a line to run. The distance between
//! `launcher: node_entry` and `command: bash -c "..."` is the point — the
//! second is what a project can write in any repository it controls, and it is
//! the shape `docs/adr/0016-declared-services.md` rejects.
//!
//! Everything here is **read and then validated elsewhere**. This module holds
//! the shapes a file may be written in and nothing about whether a given
//! declaration is one SURE will act on: whether the directory is inside the
//! project, whether the entry is there, whether the port is a real port and
//! whether the paths can be written into a request line are all
//! [`crate::service_plan`]'s work, where each refusal is a value with a sentence
//! attached. A shape that parsed is not a service SURE has agreed to start.
//!
//! # One launcher kind, and the enum is where the second one goes
//!
//! [`Launcher`] has exactly one variant in this build — `node_entry`, meaning
//! *run the `node` this machine has, with this file as its only argument*. That
//! is a decision and not a placeholder:
//!
//! - **Narrowing is what the authorising plan permits.** The plan SURE already
//!   enforces was written for declared checks, and a service declaration is the
//!   same kind of thing — a project asking for work SURE would do — so what may
//!   be added is a form SURE can carry out unambiguously, not a general escape
//!   hatch.
//! - **A Rust launcher would make SURE a build-and-fetch driver.** `cargo run`
//!   compiles, and compiling a project SURE was handed means fetching that
//!   project's dependencies, which is a non-goal: SURE never installs or
//!   retrieves anything to make a check possible.
//! - **A Python launcher would be support claimed without evidence.** Which
//!   `python` a machine has is the question `P18-T012` measured and did not
//!   settle on all three platforms, and `docs/adr/0015-support-ceiling-evidence.md`
//!   is the record of what that measurement did and did not buy. A launcher
//!   kind is a promise that SURE can start a project this way, and a promise is
//!   made where the evidence is.
//!
//! **The enum is the extension point.** A second launcher kind is a variant
//! here, a refusal vocabulary entry beside it, and the same three questions
//! answered for the platform it claims — and the declaration format is designed
//! so that adding one is additive rather than a migration. Unlike a `command`
//! string, a variant cannot be widened by a project file.

use serde::{Deserialize, Serialize};

/// One local service a project declared, for SURE to start and look at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDeclaration {
    /// What this service is called, in the words the project uses.
    ///
    /// **Must not be empty, and must not repeat another declaration's name** —
    /// the planner refuses either, with a value rather than a dropped row. A
    /// name is the whole of what tells two services in one file apart: it is
    /// what a check is titled with, and what its identifier is derived from, so
    /// two declarations sharing one name would be two checks a report could not
    /// tell apart or would report as one.
    pub name: String,
    /// The directory this service runs in, relative to the project root.
    ///
    /// `None` — the value a declaration that does not name one has — means the
    /// project root itself, which is where a single-package project's `start`
    /// lives. A path that leaves the project is refused by the planner rather
    /// than read, so a declaration cannot ask SURE to run something outside the
    /// tree it was handed.
    #[serde(default)]
    pub directory: Option<String>,
    /// How the service is started.
    ///
    /// **A tagged type and never a command line.** This is the field the whole
    /// shape exists for: nothing here splits a string back into arguments, and
    /// the program and the argument vector are SURE's own constants rather than
    /// anything the file supplies. `NodeEntry { entry }` means SURE runs
    /// `node <entry>` in the directory above, where `node` is a **name** on
    /// this machine's `PATH` and not a resolved path, and `<entry>` is exactly
    /// one argument however many spaces it contains.
    pub launcher: Launcher,
    /// The loopback port the service answers on.
    ///
    /// **Zero is refused by the planner, not by `probe::Endpoint`.** An
    /// [`Endpoint`](crate::probe::Endpoint) accepts port zero quite happily —
    /// it is a valid `u16`, and the type's job is to refuse addresses and
    /// paths that would not be the local service they claim to be. A project
    /// that declares no port has told SURE where to ask nothing, and that is a
    /// declaration SURE cannot turn into a check, so the refusal belongs here
    /// where the declaration is read rather than one layer down where the
    /// address is built.
    pub port: u16,
    /// A path that answers once the service is up.
    ///
    /// Written as a path (`/health`) rather than a URL, and validated through
    /// [`Endpoint::loopback`](crate::probe::Endpoint::loopback) against this
    /// declaration's own port. That is what makes an address off this machine
    /// unrepresentable here rather than merely unlikely: the endpoint has no
    /// host field to fill in.
    pub readiness: String,
    /// A path to open in a browser, once the service is up.
    ///
    /// `None` — the value a declaration that does not name one has — plans no
    /// browser check at all. That is not a gap being passed over in silence:
    /// the planner records why, and the reason is that a project which did not
    /// say which page to look at has not said there is a page to look at.
    #[serde(default)]
    pub page: Option<String>,
}

/// How a declared service is started.
///
/// **A key this build does not know is refused at the declaration and ignored
/// inside a variant, and the asymmetry is serde's rather than a choice.** The
/// declaration above carries `deny_unknown_fields`, so `servcies:` or `prot:` is
/// a load error rather than a setting that quietly did nothing; `deny_unknown_fields`
/// is a container attribute and serde refuses it on a variant, so a key written
/// beside `entry` inside `node_entry` is dropped by the deserializer instead. It
/// cannot matter what such a key said — nothing reads it, and every key that does
/// count is required, so a misspelled one is a missing field and an error — but a
/// reader who has just been told that this format refuses what it does not
/// understand should know where it does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Launcher {
    /// `node`, with the entry file as its one argument.
    ///
    /// The program is the **name** `node` and never a path: which `node` this
    /// machine runs is the machine's own answer through `PATH`, and naming a
    /// resolved executable would freeze one machine's answer into a project's
    /// configuration file. See the module documentation for why this is the
    /// only variant in this build.
    NodeEntry {
        /// The JavaScript file to start, relative to the declaration's
        /// directory.
        ///
        /// Must stay inside that directory and be a file that is there, and must
        /// end in `.js`, `.mjs` or `.cjs` — the planner refuses each of the
        /// three separately, so a file that is missing and a file that is not
        /// JavaScript are two different sentences rather than one.
        ///
        /// **No `package.json` is looked for**, deliberately. A directory with a
        /// `server.js` in it and no manifest is a directory SURE can start, and
        /// requiring a manifest would refuse a project that works in the name of
        /// a rule the declaration never stated.
        entry: String,
    },
}
