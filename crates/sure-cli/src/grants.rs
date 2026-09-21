//! What the settings in force allow, where they came from, and what they refused.
//!
//! # Why this is a value with two renderers rather than two `writeln!`s
//!
//! Two commands have to answer the same question in the same words. `sure check`
//! says what a run was allowed to do, and `sure config set` says what the file it
//! just wrote now allows. Before `P15-T022` only the first existed, and it did
//! not exist at all: the execution mode reached the machine frame as a bare
//! `details.mode`, full recording was nowhere, and a request the project's file
//! made and the authority layer refused was nowhere either. A person could read
//! a whole `sure check` report and not learn that the project had asked to run
//! its own code, that nothing had granted it, or which file could.
//!
//! That is a shape of silence the repository's rules name: a report that does not
//! say a thing was refused reads as though it was never asked for. So the answer
//! is built once, here, from [`Authority`] — the only place the two files are
//! arbitrated — and rendered by both commands from the same value.
//!
//! # What this module does not do
//!
//! It decides nothing. Every field is a call into `sure_core`: `permissions`,
//! `execution_mode`, `full_recording`, `full_recording_retention_days` and
//! `protection` are the arbitrated answers, and `privileges` is the list of
//! requests each layer made and whether a layer that may grant them did. A
//! second copy of the layering rule here is exactly the drift
//! `docs/architecture/CONFIG_AUTHORITY.md` exists to prevent, so there is none.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sure_core::config::{Authority, Layer, ProjectRequest, ProtectionMode};
use sure_core::execution::{ExecutionMode, ExecutionPermissions, Permission};

/// The settings file a person owns, and everything it decides.
///
/// "Their own" is the operative word: this is the file outside the project, the
/// one a checked project cannot edit, and the only one that can grant the
/// privileged behaviours listed in [`Grants::granted`]. The project's own
/// `sure.yaml` is read by the same run but appears here only as the layer that
/// *asked*, because `Layer::can_grant` is false for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grants {
    /// Where the user's own settings file is, whether or not it exists.
    ///
    /// A path and not a sentence: the human form prints it the way the operating
    /// system spells it, which on Windows is the path Explorer shows.
    pub settings_file: PathBuf,
    /// Whether a file was actually read there.
    ///
    /// `false` means every setting below is a default or the project's, and that
    /// the user has never written anything. It is stated rather than left to be
    /// inferred from an empty file, because "no file" and "an empty file" are two
    /// different facts and only one of them means the person has not answered.
    pub from_a_file: bool,
    /// The execution mode in force, after arbitration.
    pub execution_mode: ExecutionMode,
    /// The permissions in force, after arbitration.
    pub permissions: ExecutionPermissions,
    /// The protection mode in force, after arbitration.
    pub protection: ProtectionMode,
    /// Whether a full recording of the session is permitted.
    pub full_recording: bool,
    /// How many days recorded content is kept for.
    pub retention_days: i64,
    /// Requests some layer made that a layer able to grant them did.
    pub granted: Vec<ProjectRequest>,
    /// Requests that were made and were not granted.
    pub refused: Vec<Refusal>,
}

/// A request that was asked for and is not in force.
///
/// Separate from the request itself because the interesting part of a refusal is
/// *who asked*: a project that asked to run its own code and was refused is the
/// ordinary case this product exists for, and the sentence a person needs is the
/// one that says a project cannot grant itself that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// What was asked for.
    pub request: ProjectRequest,
    /// Every layer that asked, most trusted first.
    pub asked_by: Vec<Layer>,
}

impl Grants {
    /// Read the settings in force out of an [`Authority`].
    ///
    /// `settings_file` is used only when the authority carries no user layer at
    /// all — `Authority::new(None, project)`, which `Authority::load` never
    /// produces. It is a parameter rather than something this module derives,
    /// because deriving it would be a second implementation of the user's own
    /// file location, and `sure_core::paths` is the only one allowed to have one.
    #[must_use]
    pub fn of(authority: &Authority, settings_file: &Path) -> Self {
        let (settings_file, from_a_file) = match authority.user_file() {
            Some(loaded) => (loaded.searched.clone(), loaded.source.is_file()),
            None => (settings_file.to_path_buf(), false),
        };

        let mut granted = Vec::new();
        let mut refused = Vec::new();
        for privilege in authority.privileges() {
            if privilege.is_granted() {
                granted.push(privilege.request);
            } else {
                refused.push(Refusal {
                    request: privilege.request,
                    asked_by: privilege.asked_by,
                });
            }
        }

        Self {
            settings_file,
            from_a_file,
            execution_mode: authority.execution_mode(),
            permissions: authority.permissions(),
            protection: authority.protection().value,
            full_recording: authority.full_recording(),
            retention_days: authority.full_recording_retention_days().value,
            granted,
            refused,
        }
    }

    /// Whether the user's own file grants anything at all.
    ///
    /// True when the file does not exist or grants nothing, which is the
    /// starting state of every machine and the state `sure config set` exists to
    /// leave.
    #[must_use]
    pub fn nothing_granted(&self) -> bool {
        self.granted.is_empty()
    }

    /// Write the human form.
    ///
    /// Printed by `sure check` and by `sure config set`, so a person who has just
    /// run one can compare its answer with the other without a second vocabulary
    /// in between.
    ///
    /// # Errors
    ///
    /// Any failure from `out`.
    pub fn human(&self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "Your own settings, and what they allow")?;
        writeln!(out)?;

        // The path first and on its own line, because it is the answer to the
        // question this whole section exists for: "where would I write that?".
        writeln!(
            out,
            "  Your settings file: {}",
            self.settings_file.display()
        )?;
        if self.from_a_file {
            writeln!(
                out,
                "    SURE read this file. Where it and this project's own sure.yaml disagree \
                 about something either may set, the answer that is in force is below."
            )?;
        } else {
            writeln!(
                out,
                "    There is no file there. SURE is using its own defaults together with this \
                 project's own sure.yaml, and a project's file cannot grant itself anything \
                 below: only your own file can, and `sure config set <setting> <value>` writes \
                 it for you."
            )?;
        }

        writeln!(out, "  Execution mode: {}", self.execution_mode.as_str())?;
        writeln!(out, "    {}", self.execution_mode.plain_description())?;

        let permissions = self.permissions.granted();
        writeln!(out, "  Permissions: {}", permission_phrase(&permissions))?;

        if self.full_recording {
            writeln!(
                out,
                "  Full recording: on — a full transcript of the session may be kept."
            )?;
        } else {
            writeln!(
                out,
                "  Full recording: off — no full transcript of the session is kept. Your own \
                 file is the only one that can turn it on: `sure config set \
                 privacy.full_recording true`."
            )?;
        }
        writeln!(
            out,
            "  Full recording kept for: {} days",
            self.retention_days
        )?;
        writeln!(out, "  Protection: {}", self.protection.as_str())?;

        if !self.refused.is_empty() {
            writeln!(out)?;
            writeln!(out, "  Asked for, and not in force")?;
            for refusal in &self.refused {
                let asked_by = refusal
                    .asked_by
                    .iter()
                    .map(|layer| layer.plain_description())
                    .collect::<Vec<_>>()
                    .join(" and ");
                writeln!(
                    out,
                    "    {}, asked by {asked_by}. It was not granted, and that is the whole of \
                     what not granting it means: SURE did not do it.",
                    refusal.request.plain_description()
                )?;
                writeln!(
                    out,
                    "      Only your own settings file can allow it, and it is {}",
                    self.settings_file.display()
                )?;
                match crate::settings::grant_hint(refusal.request) {
                    Some(hint) => match hint.value {
                        Some(value) => writeln!(
                            out,
                            "      `sure config set {} {value}` writes that grant there.",
                            hint.setting
                        )?,
                        None => writeln!(
                            out,
                            "      `{}` in that file is the setting that decides it.",
                            hint.setting
                        )?,
                    },
                    // Nothing can grant this one in this release, which is a
                    // fact about the build and not a missing sentence. Saying
                    // nothing would leave the reader believing a way exists.
                    None => writeln!(
                        out,
                        "      No setting in this build grants it: the refusal is final until a \
                         build implements it."
                    )?,
                }
            }
        }

        writeln!(out)
    }

    /// The same answer as facts, for the response frame.
    ///
    /// Values and wire names, never prose: a script switching on
    /// `execution_mode` gets the arbitrated mode, and `refused` names every
    /// request that was asked for and not granted, so "was the project's request
    /// allowed" is a question with an answer rather than an absence to interpret.
    #[must_use]
    pub fn machine(&self) -> Value {
        json!({
            "settings_file": self.settings_file.display().to_string(),
            "settings_file_read": self.from_a_file,
            "execution_mode": self.execution_mode.as_str(),
            "permissions": self
                .permissions
                .granted()
                .iter()
                .map(|permission| permission.as_str())
                .collect::<Vec<_>>(),
            "protection": self.protection.as_str(),
            "full_recording": self.full_recording,
            "full_recording_retention_days": self.retention_days,
            "granted": self
                .granted
                .iter()
                .map(|request| request.as_str())
                .collect::<Vec<_>>(),
            "refused": self
                .refused
                .iter()
                .map(|refusal| json!({
                    "request": refusal.request.as_str(),
                    "asked_by": refusal
                        .asked_by
                        .iter()
                        .map(|layer| layer.as_str())
                        .collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>(),
        })
    }
}

/// The permissions in force as one phrase a person reads.
///
/// Built from [`Permission::consent_prompt`] rather than from a second list of
/// words written here: the prompt is the sentence SURE already asks a user to
/// agree to, so a permission whose meaning changes changes this line too, and
/// the two cannot come to mean different things.
///
/// The lone-permission case is special-cased rather than left to the join. "Read
/// your project's files and configuration" on its own does not say that it is
/// the *only* thing allowed, and the difference between "SURE may read your
/// files" and "SURE may only read your files" is the whole of what inspect-only
/// mode means.
fn permission_phrase(permissions: &[Permission]) -> String {
    if permissions == [Permission::Inspect] {
        return "read this project's files and nothing else".to_owned();
    }
    permissions
        .iter()
        .map(|permission| {
            let prompt = permission.consent_prompt();
            let mut chars = prompt.chars();
            match chars.next() {
                Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use sure_core::config::{Config, LoadedConfig};

    fn user(settings: &str) -> LoadedConfig {
        LoadedConfig {
            config: Config::from_yaml(settings).unwrap(),
            source: sure_core::config::ConfigSource::File(PathBuf::from(
                "C:/Users/x/SURE/sure.yaml",
            )),
            searched: PathBuf::from("C:/Users/x/SURE/sure.yaml"),
        }
    }

    fn project(settings: &str) -> LoadedConfig {
        LoadedConfig {
            config: Config::from_yaml(settings).unwrap(),
            source: sure_core::config::ConfigSource::NoFile,
            searched: PathBuf::from("C:/work/p/sure.yaml"),
        }
    }

    fn render(grants: &Grants) -> String {
        let mut out = Vec::new();
        grants.human(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    /// The sentence a machine with no settings file produces.
    ///
    /// This is the state `P15-T022` is about: everything is a default, nothing is
    /// granted, and the section has to say both of those and where the file would
    /// go, because a person cannot fix "nothing is granted" without it.
    #[test]
    fn a_machine_with_no_settings_file_says_so_and_names_the_path() {
        let authority = Authority::new(None, project(""));
        let grants = Grants::of(
            &authority,
            Path::new("C:/Users/x/AppData/Roaming/SURE/sure.yaml"),
        );
        assert!(!grants.from_a_file);
        assert!(grants.nothing_granted());
        assert_eq!(grants.execution_mode, ExecutionMode::InspectOnly);

        let text = render(&grants);
        assert!(text.contains("C:/Users/x/AppData/Roaming/SURE/sure.yaml"));
        assert!(text.contains("There is no file there"));
        assert!(text.contains("Execution mode: inspect_only"));
        assert!(text.contains("Full recording: off"));
        assert!(text.contains("Protection: standard"));
        // Nothing was refused, so there is no refusal section. That absence is
        // not silence: an empty list of refusals is a fact about the settings,
        // and it is the absence of a *denial* rather than the absence of an
        // answer.
        assert!(!text.contains("Asked for, and not in force"));
    }

    /// A project that asks to run its own code is refused, and the refusal names
    /// the file that could allow it.
    ///
    /// The failure this catches is the one the section exists for: a project
    /// whose `sure.yaml` asks for something and a report that says nothing about
    /// it, leaving the person to read "inspect_only" as their own choice.
    #[test]
    fn a_projects_request_that_was_refused_names_the_file_that_could_grant_it() {
        let authority = Authority::new(
            None,
            project("execution:\n  mode: host_confirmed\n  allow_network: true\n"),
        );
        let grants = Grants::of(
            &authority,
            Path::new("C:/Users/x/AppData/Roaming/SURE/sure.yaml"),
        );
        assert_eq!(grants.refused.len(), 2);
        assert_eq!(grants.refused[0].request, ProjectRequest::RunProjectCode);
        assert_eq!(grants.refused[0].asked_by, vec![Layer::Project]);
        // The mode in force is still inspect-only: the project could not grant
        // itself the escalation, which is the rule this whole section reports.
        assert_eq!(grants.execution_mode, ExecutionMode::InspectOnly);

        let text = render(&grants);
        assert!(text.contains("Asked for, and not in force"));
        assert!(text.contains("asked by this project's own sure.yaml"));
        assert!(text.contains("C:/Users/x/AppData/Roaming/SURE/sure.yaml"));
        assert!(text.contains("`sure config set execution.mode host_confirmed`"));
    }

    /// The same two settings, granted by the user's own file, are in force.
    ///
    /// This is the other half of the pair, and the one that makes the refusal
    /// above mean something: if both states produced the same section, the
    /// section would not be evidence about which one a machine is in.
    #[test]
    fn the_users_own_file_is_what_moves_the_grants() {
        let authority = Authority::new(
            Some(user(
                "execution:\n  mode: host_confirmed\n  allow_network: true\n\
                 privacy:\n  full_recording: true\n  full_recording_retention_days: 30\n",
            )),
            project(""),
        );
        let grants = Grants::of(
            &authority,
            Path::new("C:/Users/x/AppData/Roaming/SURE/sure.yaml"),
        );
        assert!(grants.from_a_file);
        assert!(grants.refused.is_empty());
        assert_eq!(grants.execution_mode, ExecutionMode::HostConfirmed);
        assert!(grants.full_recording);
        assert_eq!(grants.retention_days, 30);
        assert!(grants.permissions.network);
        assert_eq!(
            grants.granted,
            vec![
                ProjectRequest::RunProjectCode,
                ProjectRequest::Network,
                ProjectRequest::FullRecording
            ]
        );

        let machine = grants.machine();
        assert_eq!(machine["execution_mode"], "host_confirmed");
        assert_eq!(machine["full_recording"], true);
        assert_eq!(machine["full_recording_retention_days"], 30);
        assert_eq!(machine["settings_file_read"], true);
        assert_eq!(machine["refused"].as_array().unwrap().len(), 0);
    }

    /// The machine form carries the refusal as two wire names, not as a sentence.
    #[test]
    fn the_machine_form_names_every_request_and_who_asked() {
        let authority = Authority::new(None, project("execution:\n  mode: container\n"));
        let grants = Grants::of(&authority, Path::new("C:/Users/x/SURE/sure.yaml"));
        let machine = grants.machine();
        assert_eq!(machine["execution_mode"], "inspect_only");
        assert_eq!(machine["permissions"], json!(["inspect"]));
        assert_eq!(machine["refused"][0]["request"], "run_project_code");
        assert_eq!(machine["refused"][0]["asked_by"], json!(["project"]));
    }

    /// Inspect-only says that reading is the *only* thing allowed.
    ///
    /// The words come from `Permission::consent_prompt`, whose inspect entry
    /// describes what SURE may read and not that it may read nothing else. A
    /// section that printed that sentence alone would be read as permission to do
    /// more than it says, so the lone case has its own sentence and this test is
    /// what keeps it.
    #[test]
    fn inspect_only_is_stated_as_the_only_permission_and_not_as_one_of_them() {
        assert_eq!(
            permission_phrase(&[Permission::Inspect]),
            "read this project's files and nothing else"
        );
        assert_eq!(
            permission_phrase(&[Permission::Inspect, Permission::Network]),
            "read your project's files and configuration; let this check use the internet"
        );
    }
}
