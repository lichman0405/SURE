//! Grounded semantic-analysis request/response contract.
//!
//! Formalizes the contract between SURE and any semantic-analysis provider.
//! Model output is treated as inference, not deterministic fact. Every
//! assessment must be tied to evidence anchors, and provider errors must
//! never be silently treated as passing results.
//!
//! # The contract
//!
//! 1. **Requests** carry the question, a list of [`EvidenceAnchor`] values,
//!    and the project fingerprint.
//! 2. **Responses** are either [`SemanticResponse::Assessed`] with grounded
//!    assessments, [`SemanticResponse::ProviderError`] for failures, or
//!    [`SemanticResponse::Unknown`] when the response cannot be interpreted.
//! 3. **Unanchored assessments are rejected.** Any assessment without at
//!    least one checkable evidence anchor is dropped. If none remain, the
//!    response becomes [`SemanticResponse::Unknown`].
//! 4. **Provider errors never pass.** Network failure, malformed JSON,
//!    timeout or refusal all map to error or unknown.

use serde::{Deserialize, Serialize};
use sure_domain::evidence::EvidenceAnchor;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

/// A request sent to a semantic-analysis provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRequest {
    /// The question or analysis task.
    pub task: String,
    /// Evidence anchors the model is asked to interpret.
    pub anchors: Vec<EvidenceAnchor>,
    /// The project state this request is about.
    pub project_fingerprint: FingerprintId,
}

impl SemanticRequest {
    /// Build a request for `task` against `project_fingerprint`.
    #[must_use]
    pub fn new(task: impl Into<String>, project_fingerprint: FingerprintId) -> Self {
        Self {
            task: task.into(),
            anchors: Vec::new(),
            project_fingerprint,
        }
    }

    /// Attach evidence anchors.
    #[must_use]
    pub fn with_anchors(mut self, anchors: Vec<EvidenceAnchor>) -> Self {
        self.anchors = anchors;
        self
    }

    /// Add a single evidence anchor.
    #[must_use]
    pub fn with_anchor(mut self, anchor: EvidenceAnchor) -> Self {
        self.anchors.push(anchor);
        self
    }
}

/// Confidence level for a model assessment.
///
/// Deliberately discrete rather than a percentage: uncertainty is expressed
/// as a level, never as a number that could be mistaken for precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// The model is fairly sure.
    High,
    /// The model sees a pattern but it could be wrong.
    Medium,
    /// The model is guessing.
    Low,
}

impl Confidence {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// One assessment from a provider, grounded to specific evidence anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedAssessment {
    /// The claim or conclusion.
    pub claim: String,
    /// How confident the model is.
    pub confidence: Confidence,
    /// The evidence anchors this assessment is based on.
    pub anchors: Vec<EvidenceAnchor>,
    /// Severity, when applicable.
    pub severity: Option<Severity>,
}

impl GroundedAssessment {
    /// Build an assessment.
    ///
    /// `anchors` must not be empty; callers should validate before constructing.
    #[must_use]
    pub fn new(
        claim: impl Into<String>,
        confidence: Confidence,
        anchors: Vec<EvidenceAnchor>,
    ) -> Self {
        Self {
            claim: claim.into(),
            confidence,
            anchors,
            severity: None,
        }
    }

    /// Attach a severity.
    #[must_use]
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Whether every anchor is checkable.
    #[must_use]
    pub fn is_grounded(&self) -> bool {
        !self.anchors.is_empty() && self.anchors.iter().all(|a| a.is_checkable())
    }
}

/// The parsed response from a semantic-analysis provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticResponse {
    /// The provider returned grounded assessments.
    Assessed {
        /// Assessments that passed validation.
        assessments: Vec<GroundedAssessment>,
    },
    /// The provider failed in a way that is not about the project.
    ProviderError {
        /// What went wrong, escaped for human-readable output.
        reason: String,
    },
    /// The provider returned something SURE could not interpret as grounded.
    Unknown {
        /// Why the response could not be used.
        reason: String,
    },
}

impl SemanticResponse {
    /// A response representing a provider failure.
    ///
    /// Control characters in `reason` are escaped before storage.
    #[must_use]
    pub fn provider_error(reason: impl Into<String>) -> Self {
        Self::ProviderError {
            reason: crate::redact::escape_control_characters(&reason.into()),
        }
    }

    /// A response representing an uninterpretable provider answer.
    ///
    /// Control characters in `reason` are escaped before storage.
    #[must_use]
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::Unknown {
            reason: crate::redact::escape_control_characters(&reason.into()),
        }
    }

    /// Whether this response carries any grounded assessments.
    #[must_use]
    pub fn has_assessments(&self) -> bool {
        matches!(
            self,
            Self::Assessed {
                assessments,
            } if !assessments.is_empty()
        )
    }

    /// Whether this response is a provider error.
    #[must_use]
    pub fn is_provider_error(&self) -> bool {
        matches!(self, Self::ProviderError { .. })
    }

    /// Whether this response is unknown.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

/// Why a provider response could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderParseError {
    /// The JSON was malformed.
    MalformedJson {
        /// What the parser said.
        message: String,
    },
    /// The response was empty.
    EmptyResponse,
}

impl std::fmt::Display for ProviderParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedJson { message } => write!(f, "malformed JSON: {message}"),
            Self::EmptyResponse => f.write_str("the provider response was empty"),
        }
    }
}

impl std::error::Error for ProviderParseError {}

/// Internal wire type for deserializing a provider JSON response.
#[derive(Debug, Deserialize)]
struct WireAssessment {
    claim: String,
    #[serde(default = "default_confidence")]
    confidence: WireConfidence,
    #[serde(default)]
    anchors: Vec<EvidenceAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    severity: Option<Severity>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireConfidence {
    High,
    Medium,
    Low,
}

impl From<WireConfidence> for Confidence {
    fn from(w: WireConfidence) -> Self {
        match w {
            WireConfidence::High => Confidence::High,
            WireConfidence::Medium => Confidence::Medium,
            WireConfidence::Low => Confidence::Low,
        }
    }
}

fn default_confidence() -> WireConfidence {
    WireConfidence::Low
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    #[serde(default)]
    assessments: Vec<WireAssessment>,
}

/// Parse a provider JSON response into a [`SemanticResponse`].
///
/// # Errors
///
/// Returns [`ProviderParseError`] when the JSON cannot be parsed.
///
/// # Validation
///
/// - Every assessment must have at least one evidence anchor.
/// - Every anchor must be checkable ([`EvidenceAnchor::is_checkable`]).
/// - Assessments that fail either test are dropped.
/// - If no assessments remain, the result is [`SemanticResponse::Unknown`].
#[must_use = "a parsed response must be handled; provider errors must not become passes"]
pub fn parse_provider_response(json: &str) -> Result<SemanticResponse, ProviderParseError> {
    if json.trim().is_empty() {
        return Ok(SemanticResponse::unknown(
            ProviderParseError::EmptyResponse.to_string(),
        ));
    }

    let wire: WireResponse =
        serde_json::from_str(json).map_err(|e| ProviderParseError::MalformedJson {
            message: e.to_string(),
        })?;

    let mut assessments: Vec<GroundedAssessment> = Vec::new();
    let mut dropped: usize = 0;

    for entry in wire.assessments {
        let claim = crate::redact::escape_control_characters(&entry.claim);
        if entry.anchors.is_empty() {
            dropped += 1;
            continue;
        }
        if !entry.anchors.iter().all(|a| a.is_checkable()) {
            dropped += 1;
            continue;
        }
        assessments.push(GroundedAssessment {
            claim,
            confidence: entry.confidence.into(),
            anchors: entry.anchors,
            severity: entry.severity,
        });
    }

    if assessments.is_empty() {
        let reason = if dropped > 0 {
            format!("provider returned {dropped} assessment(s) with no evidence anchors")
        } else {
            "provider returned no assessments".to_owned()
        };
        return Ok(SemanticResponse::unknown(reason));
    }

    Ok(SemanticResponse::Assessed { assessments })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::{AnchorSubject, EvidenceAnchor};
    use sure_domain::ids::FingerprintId;
    use sure_domain::severity::Severity;

    fn anchor() -> EvidenceAnchor {
        EvidenceAnchor::new(AnchorSubject::File, "src/main.rs", "line 42")
    }

    fn fingerprint() -> FingerprintId {
        FingerprintId::parse("fp_00000000000000000000").expect("well-formed")
    }

    #[test]
    fn a_valid_response_with_anchors_is_parsed_into_assessments() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "The implementation matches the intent",
                    "confidence": "high",
                    "anchors": [
                        {
                            "subject": "file",
                            "location": "src/main.rs",
                            "locator": "line 42"
                        }
                    ],
                    "severity": "must_fix"
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        assert!(result.has_assessments());
        let SemanticResponse::Assessed { assessments } = result else {
            panic!("expected Assessed");
        };
        assert_eq!(assessments.len(), 1);
        assert_eq!(
            assessments[0].claim,
            "The implementation matches the intent"
        );
        assert_eq!(assessments[0].confidence, Confidence::High);
        assert_eq!(assessments[0].severity, Some(Severity::MustFix));
        assert_eq!(assessments[0].anchors.len(), 1);
        assert!(assessments[0].is_grounded());
    }

    #[test]
    fn an_unanchored_assessment_is_rejected_and_the_rest_kept() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "grounded claim",
                    "confidence": "medium",
                    "anchors": [
                        {
                            "subject": "file",
                            "location": "src/lib.rs",
                            "locator": "line 10"
                        }
                    ]
                },
                {
                    "claim": "unanchored claim",
                    "confidence": "high",
                    "anchors": []
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        let SemanticResponse::Assessed { assessments } = result else {
            panic!("expected Assessed, got something else");
        };
        assert_eq!(assessments.len(), 1);
        assert_eq!(assessments[0].claim, "grounded claim");
    }

    #[test]
    fn a_response_with_only_unanchored_assessments_becomes_unknown() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "pass",
                    "confidence": "high",
                    "anchors": []
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        assert!(!result.has_assessments());
        assert!(result.is_unknown());
    }

    #[test]
    fn a_pass_claim_without_evidence_is_rejected() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "pass",
                    "confidence": "high",
                    "anchors": []
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        assert!(matches!(result, SemanticResponse::Unknown { .. }));
    }

    #[test]
    fn malformed_json_yields_provider_error() {
        let result = parse_provider_response("not json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProviderParseError::MalformedJson { .. }));
    }

    #[test]
    fn empty_response_yields_unknown() {
        let result = parse_provider_response("").expect("returns a response");
        assert!(result.is_unknown());
    }

    #[test]
    fn provider_error_maps_to_error_not_pass() {
        let response = SemanticResponse::provider_error("network timeout");
        assert!(response.is_provider_error());
        assert!(!response.has_assessments());
        assert!(!response.is_unknown());

        let response2 = SemanticResponse::unknown("no useful answer");
        assert!(response2.is_unknown());
        assert!(!response2.has_assessments());
    }

    #[test]
    fn request_carries_task_anchors_and_fingerprint() {
        let request = SemanticRequest::new("does this match intent?", fingerprint())
            .with_anchor(anchor())
            .with_anchor(EvidenceAnchor::new(
                AnchorSubject::LineRange,
                "src/lib.rs",
                "line 5",
            ));

        assert_eq!(request.task, "does this match intent?");
        assert_eq!(request.anchors.len(), 2);
        assert_eq!(request.project_fingerprint, fingerprint());
    }

    #[test]
    fn grounded_assessment_requires_nonempty_checkable_anchors() {
        let good = GroundedAssessment::new("x", Confidence::Low, vec![anchor()]);
        assert!(good.is_grounded());

        let bad = GroundedAssessment::new("x", Confidence::Low, vec![]);
        assert!(!bad.is_grounded());

        let empty_loc = GroundedAssessment::new(
            "x",
            Confidence::Low,
            vec![EvidenceAnchor::new(AnchorSubject::File, "", "")],
        );
        assert!(!empty_loc.is_grounded());
    }

    #[test]
    fn control_characters_in_provider_text_are_escaped() {
        let raw = "line one\nline two";
        let response = SemanticResponse::provider_error(raw);
        match response {
            SemanticResponse::ProviderError { reason } => {
                assert!(!reason.contains('\n'));
                assert!(reason.contains("\\n"));
            }
            _ => panic!("expected ProviderError"),
        }
    }

    #[test]
    fn semantic_response_never_looks_like_a_pass() {
        // The contract: ProviderError and Unknown are not passes.
        let error = SemanticResponse::provider_error("something failed");
        let unknown = SemanticResponse::unknown("could not tell");
        assert!(!error.has_assessments());
        assert!(!unknown.has_assessments());
        assert!(error.is_provider_error());
        assert!(unknown.is_unknown());
    }

    #[test]
    fn an_assessment_with_an_empty_anchor_is_rejected() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "has empty anchor",
                    "confidence": "high",
                    "anchors": [
                        {
                            "subject": "file",
                            "location": "",
                            "locator": ""
                        }
                    ]
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        assert!(result.is_unknown());
    }

    #[test]
    fn a_response_with_no_assessments_becomes_unknown() {
        let json = serde_json::json!({
            "assessments": []
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        assert!(result.is_unknown());
    }

    #[test]
    fn default_confidence_is_low() {
        let json = serde_json::json!({
            "assessments": [
                {
                    "claim": "no confidence field",
                    "anchors": [
                        {
                            "subject": "file",
                            "location": "a.rs",
                            "locator": "line 1"
                        }
                    ]
                }
            ]
        })
        .to_string();

        let result = parse_provider_response(&json).expect("parses");
        let SemanticResponse::Assessed { assessments } = result else {
            panic!("expected Assessed");
        };
        assert_eq!(assessments[0].confidence, Confidence::Low);
    }

    #[test]
    fn integration_semantic_analysis_flow() {
        // Build a request, simulate a provider response, parse it, and verify
        // the contract holds end-to-end.
        let request = SemanticRequest::new(
            "Does the implementation match the stated intent?",
            fingerprint(),
        )
        .with_anchors(vec![
            EvidenceAnchor::new(AnchorSubject::File, "README.md", "project goals"),
            EvidenceAnchor::new(AnchorSubject::LineRange, "src/main.rs", "login handler"),
        ]);

        // Simulate a provider response JSON.
        let response_json = serde_json::json!({
            "assessments": [
                {
                    "claim": "The login handler is present but does not reject empty passwords",
                    "confidence": "high",
                    "anchors": [
                        {
                            "subject": "line_range",
                            "location": "src/main.rs",
                            "locator": "login handler"
                        }
                    ],
                    "severity": "must_fix"
                }
            ]
        })
        .to_string();

        let parsed = parse_provider_response(&response_json).expect("parses");
        let SemanticResponse::Assessed { assessments } = parsed else {
            panic!("expected Assessed in the integration flow");
        };

        assert_eq!(assessments.len(), 1);
        let assessment = &assessments[0];
        assert_eq!(
            assessment.claim,
            "The login handler is present but does not reject empty passwords"
        );
        assert_eq!(assessment.confidence, Confidence::High);
        assert_eq!(assessment.severity, Some(Severity::MustFix));
        assert!(assessment.is_grounded());

        // Verify the request anchors are separate from response anchors.
        assert_eq!(request.anchors.len(), 2);
        // The response anchor is what the provider returned.
        assert_eq!(assessment.anchors.len(), 1);
    }
}
