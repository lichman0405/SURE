//! The local services a project declares, as typed declarations rather than
//! command lines.
//!
//! `checks.services` is the one setting in `sure.yaml` that asks SURE to start
//! something, and the whole of this module is the shape it may be written in. A
//! declaration is **narrower than a command**: it names a launcher SURE knows,
//! a port on loopback and a path that answers once the service is up, and there
//! is no field anywhere in it that accepts a line to run. The distance between
//! `kind: node_entry` and `command: bash -c "..."` is the point — the second is
//! what a project can write in any repository it controls, and it is the shape
//! `docs/adr/0016-declared-services.md` rejects.
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
    /// tree it was handed — **by its spelling or by where it resolves to**, since
    /// a junction needs no privilege and the planner asks the file system as well
    /// as the text.
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

/// How a declared service is started, as a **tagged type** and never a command
/// line.
///
/// It is written as an ordinary mapping with a `kind` key:
///
/// ```yaml
/// launcher:
///   kind: node_entry
///   entry: server.js
/// ```
///
/// **The kind is a key SURE reads and a key it does not know is refused**, and so
/// is a key written beside `entry`: `deny_unknown_fields` reaches both. That
/// matters more here than anywhere else in this file, because this is the field a
/// project reaches for when it wants SURE to run something general, and the answer
/// has to be a failure rather than a setting that quietly did nothing. An
/// implementation that dropped an unrecognised key would let a project believe it
/// had configured something SURE never read.
///
/// **The tag is a plain `kind` key rather than a YAML `!tag`, and that is a
/// constraint rather than a style.** `serde_yaml_ng` implements `deserialize_enum`
/// by requiring a tag, so an *externally* tagged enum written the way a person
/// would naturally write it —
/// `launcher: { node_entry: { entry: … } }` — does not parse at all:
/// *`invalid type: map, expected a YAML tag starting with '!'`*. An internally
/// tagged enum is read as an ordinary mapping instead, which makes it the only
/// spelling of the two that a project can actually write in this format, and the
/// only one whose unknown keys `deny_unknown_fields` can reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Launcher {
    /// `node`, with the entry file as its one argument.
    ///
    /// The program is the **name** `node` and never a path: which `node` this
    /// machine runs is the machine's own answer through `PATH`, and naming a
    /// resolved executable would freeze one machine's answer into a project's
    /// configuration file. See the module documentation for why this is the
    /// only variant in this build.
    ///
    /// Selected by `kind: node_entry`, with `entry` beside it.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// The block as `sure.example.yaml` writes it and as the architecture docs
    /// quote it. It is a *string in a test* and not a read of those files on
    /// purpose: a documentation file can be edited by someone who never builds
    /// this crate, and what has to fail then is a test rather than a user's first
    /// `sure check`. **This test exists because the first shape this module had
    /// did not parse at all** — an externally tagged enum written as a nested
    /// mapping, which `serde_yaml_ng` refuses because it requires a YAML tag — and
    /// nothing in the tree deserialized one from text, so every test in the round
    /// passed against a format no project could write.
    const DOCUMENTED: &str = "\
checks:
  services:
    - name: api
      directory: packages/api
      launcher:
        kind: node_entry
        entry: server.js
      port: 4310
      readiness: /health
      page: /
";

    #[test]
    fn the_documented_shape_parses_into_a_declaration() {
        let config = Config::from_yaml(DOCUMENTED).expect("the documented block loads");

        assert_eq!(
            config.checks.services,
            vec![ServiceDeclaration {
                name: "api".to_owned(),
                directory: Some("packages/api".to_owned()),
                launcher: Launcher::NodeEntry {
                    entry: "server.js".to_owned(),
                },
                port: 4310,
                readiness: "/health".to_owned(),
                page: Some("/".to_owned()),
            }],
        );
    }

    #[test]
    fn a_key_beside_the_entry_is_refused_rather_than_dropped() {
        // The field a project reaches for when it wants a general command, and
        // the answer has to be a load error: a key the deserializer dropped would
        // leave a project believing it had configured something SURE never read.
        let text = DOCUMENTED.replace(
            "        entry: server.js\n",
            "        entry: server.js\n        command: bash -c 'curl evil'\n",
        );
        let error = Config::from_yaml(&text).expect_err("the stray key is refused");

        assert!(
            format!("{error}").contains("command"),
            "the error should name the key it refused, and it is: {error}",
        );
    }

    #[test]
    fn a_launcher_kind_this_build_does_not_have_is_refused() {
        let text = DOCUMENTED.replace("kind: node_entry", "kind: shell_command");
        let error = Config::from_yaml(&text).expect_err("an unknown kind is refused");

        assert!(
            format!("{error}").contains("shell_command"),
            "the error should name the kind it refused, and it is: {error}",
        );
    }
}
