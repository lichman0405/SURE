//! Checking a document against the schema that describes it.
//!
//! # Why this is not a JSON Schema library
//!
//! A general validator was the first choice, and it was rejected on cost:
//! `jsonschema` pulls in an HTTP client, a TLS implementation and roughly fifty
//! transitive crates to evaluate eleven keywords.
//! `docs/architecture/RUST_DESIGN.md` asks that the dependency tree stay small
//! enough for native Windows packaging to stay tractable, and a network stack
//! inside a local-first tool is the wrong shape even as a test-only dependency.
//!
//! The risk of doing it here instead is the one `sure-testkit`'s workspace
//! reader names: a hand-written checker that quietly mis-reads its input
//! reports "no violations", which is a false green inside the check that exists
//! to prevent false greens. Two things answer that risk, and neither is a
//! promise to be careful:
//!
//! 1. **An unsupported keyword is an error, not a skip.** [`Schema::parse`]
//!    walks the whole document, `properties` and `items` included, and refuses
//!    one containing anything this module does not enforce. A schema cannot
//!    silently outgrow the validator; the parse fails and CI stops. The
//!    supported set is [`SUPPORTED_KEYWORDS`], and
//!    [`supported_keywords`] hands it to a test so the list cannot change
//!    without someone deciding to.
//! 2. **The validator is tested against documents that must fail.** Every
//!    keyword has a test that removes or corrupts the thing it checks and
//!    asserts a violation comes back. A validator that accepted everything
//!    would fail those tests.
//!
//! `format` is enforced rather than treated as an annotation. Draft 2020-12
//! makes `format` annotation-only by default, which is exactly the kind of
//! quiet difference that makes a reader believe a timestamp was checked when it
//! was not. The one format SURE uses, `date-time`, is checked here, and any
//! other value is refused at parse time.
//!
//! Only `minimum` is supported, not `maximum`. Where a bound has to be exact,
//! the schemas write it as an `enum`, which is how `capability_tier` is bounded
//! to 0, 1 and 2. An `items` array — the draft-07 tuple form, replaced by
//! `prefixItems` in 2020-12 — is a malformed schema here rather than a silently
//! mis-enforced one.

use std::collections::BTreeSet;
use std::fmt;

use serde_json::{Map, Value};

/// Keywords this module enforces. Anything else stops the parse.
const SUPPORTED_KEYWORDS: &[&str] = &[
    "$schema",
    "title",
    "description",
    "type",
    "required",
    "properties",
    "items",
    "enum",
    "minimum",
    "format",
    "additionalProperties",
];

/// The supported keywords that describe rather than constrain.
///
/// They cannot be violated, so nothing in [`Schema::check`] reads them and
/// nothing in [`Schema::check_keyword_shape`] has a shape to enforce. Listing
/// them separately is what lets a test tell "this keyword is an annotation" from
/// "someone added this keyword and forgot to enforce it", which would otherwise
/// both look like a keyword nothing checks.
pub const ANNOTATION_KEYWORDS: &[&str] = &["$schema", "title", "description"];

/// The one `format` this module knows how to check.
const SUPPORTED_FORMAT: &str = "date-time";

/// A schema that cannot be used, because this module would not enforce all of
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// The schema text was not valid JSON.
    NotJson {
        /// Which schema.
        name: String,
        /// What the parser said.
        message: String,
    },
    /// The schema used a keyword this module does not enforce.
    ///
    /// Refused rather than ignored: a constraint that is not applied is worse
    /// than a constraint that is absent, because the schema reads as though it
    /// were there.
    Unsupported {
        /// Which schema.
        name: String,
        /// Where in the schema.
        path: String,
        /// The keyword.
        keyword: String,
    },
    /// A supported keyword had the wrong shape.
    Malformed {
        /// Which schema.
        name: String,
        /// Where in the schema.
        path: String,
        /// What is wrong with it.
        problem: String,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJson { name, message } => {
                write!(f, "{name} is not valid JSON: {message}")
            }
            Self::Unsupported {
                name,
                path,
                keyword,
            } => write!(
                f,
                "{name} uses \"{keyword}\" at {path}, which this validator does not enforce.\n\n\
                 SURE refuses to check a document against a schema it only partly understands, \
                 because a constraint that is silently skipped reads exactly like one that \
                 passed."
            ),
            Self::Malformed {
                name,
                path,
                problem,
            } => write!(f, "{name} is malformed at {path}: {problem}"),
        }
    }
}

impl std::error::Error for SchemaError {}

/// Why a document does not satisfy its schema.
///
/// Every variant names the value, not just the rule, so a message can be acted
/// on without opening the schema.
///
/// Not `Eq`: [`ViolationKind::BelowMinimum`] carries the schema's `minimum` as
/// an `f64`, which is a value read out of JSON and not a total order. Claiming
/// `Eq` for it would be a claim about a type that does not have one.
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationKind {
    /// A required key was absent.
    Missing {
        /// The key.
        key: String,
    },
    /// A value had the wrong JSON type.
    WrongType {
        /// What the schema allows, as written in the schema.
        expected: String,
        /// What the document had.
        found: String,
    },
    /// A value was not one of the allowed ones.
    NotAllowed {
        /// The values the schema allows.
        allowed: Vec<String>,
    },
    /// A number was below the schema's minimum.
    BelowMinimum {
        /// The minimum.
        minimum: f64,
    },
    /// A string was not of the declared format.
    BadFormat {
        /// The format.
        format: String,
    },
    /// An object had a key the schema does not describe.
    UnexpectedKey {
        /// The key.
        key: String,
    },
}

/// One place where a document and its schema disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// Which schema.
    pub schema: String,
    /// Where in the document, as a dotted path.
    pub path: String,
    /// What is wrong.
    pub kind: ViolationKind,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = if self.path.is_empty() {
            "the document".to_owned()
        } else {
            self.path.clone()
        };
        match &self.kind {
            ViolationKind::Missing { key } => {
                write!(
                    f,
                    "{at} is missing \"{key}\", which {} requires",
                    self.schema
                )
            }
            ViolationKind::WrongType { expected, found } => {
                write!(f, "{at} must be {expected}; {} found {found}", self.schema)
            }
            ViolationKind::NotAllowed { allowed } => write!(
                f,
                "{at} must be one of {}; {} would not accept it",
                allowed
                    .iter()
                    .map(|value| format!("\"{value}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.schema
            ),
            ViolationKind::BelowMinimum { minimum } => {
                write!(
                    f,
                    "{at} must be at least {minimum}; {} found less",
                    self.schema
                )
            }
            ViolationKind::BadFormat { format } => {
                write!(f, "{at} must be a {format} as {} defines it", self.schema)
            }
            ViolationKind::UnexpectedKey { key } => write!(
                f,
                "{at} has \"{key}\", which {} does not describe",
                self.schema
            ),
        }
    }
}

/// A schema, checked at parse time and usable afterwards.
#[derive(Debug, Clone)]
pub struct Schema {
    name: String,
    root: Value,
}

impl Schema {
    /// Read a schema, refusing one this module cannot fully enforce.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError`] if the text is not JSON, uses a keyword outside
    /// [`SUPPORTED_KEYWORDS`], or uses a supported keyword in a shape this
    /// module does not understand.
    pub fn parse(name: &str, text: &str) -> Result<Self, SchemaError> {
        let root: Value = serde_json::from_str(text).map_err(|error| SchemaError::NotJson {
            name: name.to_owned(),
            message: error.to_string(),
        })?;
        let schema = Self {
            name: name.to_owned(),
            root,
        };
        schema.check_enforceable()?;
        Ok(schema)
    }

    /// What the schema is called in a violation message.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Every way the document disagrees with the schema.
    ///
    /// Empty means the document satisfies the schema. All violations are
    /// reported rather than the first, because a caller fixing one at a time
    /// would need as many runs as there are mistakes.
    #[must_use]
    pub fn validate(&self, instance: &Value) -> Vec<Violation> {
        let mut found = Vec::new();
        self.check(instance, "", &self.root, &mut found);
        found
    }

    /// Walk the schema and refuse anything this module would not enforce.
    fn check_enforceable(&self) -> Result<(), SchemaError> {
        self.check_schema_object(&self.root, "")
    }

    /// Check one schema or subschema, then the ones nested inside it.
    ///
    /// The recursion is the point. Reading only the top level would let a
    /// keyword this module does not enforce sit inside `properties` and be
    /// skipped, which is the exact failure the check exists to prevent.
    fn check_schema_object(&self, schema: &Value, path: &str) -> Result<(), SchemaError> {
        let shown = if path.is_empty() { "/" } else { path };
        let object = schema.as_object().ok_or_else(|| SchemaError::Malformed {
            name: self.name.clone(),
            path: shown.to_owned(),
            problem: "a schema must be a JSON object".to_owned(),
        })?;

        for (keyword, value) in object {
            if !SUPPORTED_KEYWORDS.contains(&keyword.as_str()) {
                return Err(SchemaError::Unsupported {
                    name: self.name.clone(),
                    path: shown.to_owned(),
                    keyword: keyword.clone(),
                });
            }
            self.check_keyword_shape(keyword, value, path)?;
        }

        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            for (key, subschema) in properties {
                self.check_schema_object(subschema, &join(path, key))?;
            }
        }
        if let Some(items) = object.get("items") {
            self.check_schema_object(items, &join(path, "items"))?;
        }
        Ok(())
    }

    /// Refuse a supported keyword written in a shape this module would
    /// misread.
    fn check_keyword_shape(
        &self,
        keyword: &str,
        value: &Value,
        path: &str,
    ) -> Result<(), SchemaError> {
        let malformed = |problem: String| SchemaError::Malformed {
            name: self.name.clone(),
            path: join(path, keyword),
            problem,
        };
        match keyword {
            "$schema" | "title" | "description" => value
                .is_string()
                .then_some(())
                .ok_or_else(|| malformed("must be a string".to_owned())),
            "type" => match value {
                Value::String(name) if is_type_name(name) => Ok(()),
                Value::Array(names)
                    if !names.is_empty()
                        && names.iter().all(|n| n.as_str().is_some_and(is_type_name)) =>
                {
                    Ok(())
                }
                _ => Err(malformed(
                    "must be a JSON type name, or a non-empty array of them".to_owned(),
                )),
            },
            "required" => match value {
                Value::Array(names) if names.iter().all(|n| n.is_string()) => Ok(()),
                _ => Err(malformed("must be an array of strings".to_owned())),
            },
            "properties" => value
                .is_object()
                .then_some(())
                .ok_or_else(|| malformed("must be an object".to_owned())),
            "items" => value
                .is_object()
                .then_some(())
                .ok_or_else(|| malformed("must be a single schema object".to_owned())),
            "enum" => match value {
                Value::Array(values) if !values.is_empty() => Ok(()),
                _ => Err(malformed("must be a non-empty array".to_owned())),
            },
            "minimum" => value
                .is_number()
                .then_some(())
                .ok_or_else(|| malformed("must be a number".to_owned())),
            "format" => match value.as_str() {
                Some(SUPPORTED_FORMAT) => Ok(()),
                Some(other) => Err(malformed(format!(
                    "\"{other}\" is not a format this validator enforces; only \"{SUPPORTED_FORMAT}\" is"
                ))),
                None => Err(malformed("must be a string".to_owned())),
            },
            "additionalProperties" => match value {
                // The schema form of this keyword needs `properties` or
                // `patternProperties` reasoning inside it, which is not
                // implemented. Only the boolean form is understood.
                Value::Bool(_) => Ok(()),
                _ => Err(malformed(
                    "must be a boolean; the subschema form is not enforced".to_owned(),
                )),
            },
            // Reached only by [`ANNOTATION_KEYWORDS`], which constrain nothing
            // and so have no shape to check. Adding a keyword to
            // [`SUPPORTED_KEYWORDS`] without adding an arm here lands in this
            // case too, and the tests fail on it:
            // `every_supported_keyword_is_either_enforced_or_an_annotation`.
            _ => Ok(()),
        }
    }

    fn check(&self, instance: &Value, path: &str, schema: &Value, found: &mut Vec<Violation>) {
        let Some(object) = schema.as_object() else {
            return;
        };

        self.check_type(instance, path, object, found);
        self.check_enum(instance, path, object, found);
        self.check_minimum(instance, path, object, found);
        self.check_format(instance, path, object, found);

        if let Some(required) = object.get("required").and_then(Value::as_array) {
            let Some(instance_object) = instance.as_object() else {
                return;
            };
            for key in required.iter().filter_map(Value::as_str) {
                if !instance_object.contains_key(key) {
                    found.push(self.violation(
                        path,
                        ViolationKind::Missing {
                            key: key.to_owned(),
                        },
                    ));
                }
            }
        }

        if let Some(instance_object) = instance.as_object() {
            // Read outside the `properties` lookup on purpose. A schema may say
            // `additionalProperties: false` with no `properties` at all, which
            // means "this object has no keys"; hanging the check off
            // `properties` would silently allow every key in that case.
            let properties = object.get("properties").and_then(Value::as_object);
            let closed = object.get("additionalProperties") == Some(&Value::Bool(false));
            for (key, value) in instance_object {
                match properties.and_then(|properties| properties.get(key)) {
                    Some(subschema) => {
                        let sub_path = join(path, key);
                        self.check(value, &sub_path, subschema, found);
                    }
                    // Only reported when the schema says so. Five of the seven
                    // document schemas are lower bounds, so a field they do not
                    // mention is not a mistake — the domain types carry SURE's
                    // own provenance fields beyond what a reader needs.
                    None if closed => {
                        found.push(
                            self.violation(path, ViolationKind::UnexpectedKey { key: key.clone() }),
                        );
                    }
                    None => {}
                }
            }
        }

        if let Some(items) = object.get("items")
            && let Some(array) = instance.as_array()
        {
            for (index, value) in array.iter().enumerate() {
                let sub_path = format!("{path}[{index}]");
                self.check(value, &sub_path, items, found);
            }
        }
    }

    fn check_type(
        &self,
        instance: &Value,
        path: &str,
        object: &Map<String, Value>,
        found: &mut Vec<Violation>,
    ) {
        let Some(declared) = object.get("type") else {
            return;
        };
        let names: Vec<&str> = match declared {
            Value::String(name) => vec![name.as_str()],
            Value::Array(list) => list.iter().filter_map(Value::as_str).collect(),
            _ => return,
        };
        if names.iter().any(|name| type_matches(name, instance)) {
            return;
        }
        found.push(self.violation(
            path,
            ViolationKind::WrongType {
                expected: names.join(" or "),
                found: type_name(instance).to_owned(),
            },
        ));
    }

    fn check_enum(
        &self,
        instance: &Value,
        path: &str,
        object: &Map<String, Value>,
        found: &mut Vec<Violation>,
    ) {
        let Some(allowed) = object.get("enum").and_then(Value::as_array) else {
            return;
        };
        if allowed.contains(instance) {
            return;
        }
        found.push(
            self.violation(
                path,
                ViolationKind::NotAllowed {
                    allowed: allowed
                        .iter()
                        .map(|value| match value {
                            Value::String(text) => text.clone(),
                            other => other.to_string(),
                        })
                        .collect(),
                },
            ),
        );
    }

    fn check_minimum(
        &self,
        instance: &Value,
        path: &str,
        object: &Map<String, Value>,
        found: &mut Vec<Violation>,
    ) {
        let Some(minimum) = object.get("minimum").and_then(Value::as_f64) else {
            return;
        };
        let Some(number) = instance.as_f64() else {
            return;
        };
        if number < minimum {
            found.push(self.violation(path, ViolationKind::BelowMinimum { minimum }));
        }
    }

    fn check_format(
        &self,
        instance: &Value,
        path: &str,
        object: &Map<String, Value>,
        found: &mut Vec<Violation>,
    ) {
        if object.get("format").and_then(Value::as_str) != Some(SUPPORTED_FORMAT) {
            return;
        }
        let Some(text) = instance.as_str() else {
            return;
        };
        if !is_date_time(text) {
            found.push(self.violation(
                path,
                ViolationKind::BadFormat {
                    format: SUPPORTED_FORMAT.to_owned(),
                },
            ));
        }
    }

    fn violation(&self, path: &str, kind: ViolationKind) -> Violation {
        Violation {
            schema: self.name.clone(),
            path: path.to_owned(),
            kind,
        }
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

fn is_type_name(name: &str) -> bool {
    matches!(
        name,
        "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
    )
}

fn type_matches(name: &str, instance: &Value) -> bool {
    match name {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "number" => instance.is_number(),
        // A whole number. `1.0` is a number but not an integer, and the schemas
        // use this for versions and counts.
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        _ => false,
    }
}

fn type_name(instance: &Value) -> &'static str {
    match instance {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Whether a string is an RFC 3339 date-time.
///
/// Deliberately shape-checking rather than calendar-checking: `2026-02-31` is
/// accepted here and rejected by the timestamp parser that reads it. The job of
/// this function is to catch a value that is not a timestamp at all — a date
/// with no time, a local time with no offset — before it reaches a stored
/// record.
fn is_date_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    // `YYYY-MM-DDTHH:MM:SS` is the shortest form, and the offset follows.
    if bytes.len() < 20 || text.len() != bytes.len() {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .is_some_and(|slice| slice.iter().all(u8::is_ascii_digit))
    };
    let separators = |index: usize, expected: u8| bytes.get(index) == Some(&expected);

    if !(digits(0..4)
        && separators(4, b'-')
        && digits(5..7)
        && separators(7, b'-')
        && digits(8..10)
        && separators(10, b'T')
        && digits(11..13)
        && separators(13, b':')
        && digits(14..16)
        && separators(16, b':')
        && digits(17..19))
    {
        return false;
    }

    let number = |range: std::ops::Range<usize>| -> u32 {
        text.get(range)
            .and_then(|part| part.parse().ok())
            .unwrap_or(u32::MAX)
    };
    if number(5..7) < 1
        || number(5..7) > 12
        || number(8..10) < 1
        || number(8..10) > 31
        || number(11..13) > 23
        || number(14..16) > 59
        // 60 is a leap second, and is legal.
        || number(17..19) > 60
    {
        return false;
    }

    let mut rest = &text[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let length = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if length == 0 {
            return false;
        }
        rest = &fraction[length..];
    }
    if rest == "Z" || rest == "z" {
        return true;
    }
    let Some(offset) = rest.strip_prefix(['+', '-']) else {
        return false;
    };
    offset.len() == 5
        && offset.as_bytes().get(2) == Some(&b':')
        && offset.bytes().enumerate().all(|(index, byte)| {
            if index == 2 {
                byte == b':'
            } else {
                byte.is_ascii_digit()
            }
        })
        && offset[..2].parse::<u32>().is_ok_and(|hours| hours <= 23)
}

/// The keywords this module enforces, for a test that keeps the list honest.
#[must_use]
pub fn supported_keywords() -> BTreeSet<&'static str> {
    SUPPORTED_KEYWORDS.iter().copied().collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema(text: &str) -> Schema {
        Schema::parse("test.schema.json", text).expect("the schema is enforceable")
    }

    #[test]
    fn a_missing_required_key_is_reported_by_name() {
        let schema = schema(r#"{"type":"object","required":["id","status"]}"#);
        let found = schema.validate(&json!({"id": "chk_1"}));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            found[0].kind,
            ViolationKind::Missing {
                key: "status".to_owned()
            }
        );
        assert!(found[0].to_string().contains("status"), "{found:?}");
    }

    #[test]
    fn a_present_key_holding_null_satisfies_required() {
        // `required` is about presence. A schema that also wants a non-null
        // value says so with `type`, and the two are checked separately.
        let schema = schema(r#"{"type":"object","required":["session_id"]}"#);
        assert!(schema.validate(&json!({"session_id": null})).is_empty());
    }

    #[test]
    fn a_wrong_type_is_reported_with_both_types_named() {
        let schema = schema(r#"{"type":"object","properties":{"count":{"type":"integer"}}}"#);
        let found = schema.validate(&json!({"count": "three"}));
        assert_eq!(found.len(), 1, "{found:?}");
        match &found[0].kind {
            ViolationKind::WrongType { expected, found } => {
                assert_eq!(expected, "integer");
                assert_eq!(found, "string");
            }
            other => panic!("expected a type violation, got {other:?}"),
        }
    }

    #[test]
    fn a_float_is_not_an_integer() {
        let schema = schema(r#"{"type":"object","properties":{"count":{"type":"integer"}}}"#);
        assert_eq!(schema.validate(&json!({"count": 1.5})).len(), 1);
        assert!(schema.validate(&json!({"count": 2})).is_empty());
    }

    #[test]
    fn a_union_type_is_understood() {
        let schema = schema(r#"{"type":"object","properties":{"x":{"type":["string","null"]}}}"#);
        assert!(schema.validate(&json!({"x": "a"})).is_empty());
        assert!(schema.validate(&json!({"x": null})).is_empty());
        assert_eq!(schema.validate(&json!({"x": 1})).len(), 1);
    }

    #[test]
    fn an_out_of_enum_value_lists_the_allowed_ones() {
        let schema = schema(r#"{"type":"object","properties":{"s":{"enum":["a","b"]}}}"#);
        let found = schema.validate(&json!({"s": "c"}));
        assert_eq!(found.len(), 1, "{found:?}");
        let message = found[0].to_string();
        assert!(message.contains("\"a\""), "{message}");
        assert!(message.contains("\"b\""), "{message}");
        assert!(schema.validate(&json!({"s": "a"})).is_empty());
    }

    #[test]
    fn a_number_below_the_minimum_is_reported() {
        let schema =
            schema(r#"{"type":"object","properties":{"v":{"type":"integer","minimum":1}}}"#);
        assert_eq!(schema.validate(&json!({"v": 0})).len(), 1);
        assert!(schema.validate(&json!({"v": 1})).is_empty());
    }

    #[test]
    fn an_array_element_is_checked_at_its_own_index() {
        let schema = schema(
            r#"{"type":"object","properties":{"items":{"type":"array","items":{"type":"object","required":["id"]}}}}"#,
        );
        let found = schema.validate(&json!({"items": [{"id": "a"}, {}, {"id": "c"}]}));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path, "items[1]");
    }

    #[test]
    fn an_unexpected_key_is_only_reported_where_the_schema_forbids_it() {
        let open = schema(r#"{"type":"object","required":["id"]}"#);
        assert!(open.validate(&json!({"id": "a", "extra": 1})).is_empty());

        let closed = schema(
            r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"}},"additionalProperties":false}"#,
        );
        let found = closed.validate(&json!({"id": "a", "extra": 1}));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path, "");
        assert!(found[0].to_string().contains("extra"), "{found:?}");
        assert!(closed.validate(&json!({"id": "a"})).is_empty());
    }

    #[test]
    fn a_closed_object_with_no_properties_allows_no_keys_at_all() {
        // A regression, kept because the first version of this module hung the
        // check off the `properties` lookup and so silently allowed every key
        // when a schema named none. That is the shape of a false green: the
        // schema read as though it forbade something and nothing was checked.
        let nothing_allowed = schema(r#"{"type":"object","additionalProperties":false}"#);
        let found = nothing_allowed.validate(&json!({"a": 1, "b": 2}));
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(nothing_allowed.validate(&json!({})).is_empty());
    }

    #[test]
    fn a_date_time_is_checked_rather_than_annotated() {
        let schema = schema(
            r#"{"type":"object","properties":{"t":{"type":"string","format":"date-time"}}}"#,
        );
        for good in [
            "2026-09-14T09:10:56Z",
            "2026-09-14T09:10:56.827Z",
            "2026-09-14T09:10:56+01:00",
            "2026-09-14T09:10:56.827-05:00",
        ] {
            assert!(
                schema.validate(&json!({"t": good})).is_empty(),
                "{good} was rejected"
            );
        }
        for bad in [
            "2026-09-14",
            "2026-09-14T09:10:56",
            "2026-09-14 09:10:56Z",
            "2026-13-14T09:10:56Z",
            "2026-09-14T25:10:56Z",
            "not a timestamp",
            "",
        ] {
            assert_eq!(
                schema.validate(&json!({"t": bad})).len(),
                1,
                "{bad} was accepted"
            );
        }
    }

    #[test]
    fn an_unsupported_keyword_stops_the_parse_rather_than_being_skipped() {
        // The property that keeps this validator honest. A schema that grows a
        // keyword this module does not enforce must fail loudly, because a
        // constraint nobody applies reads exactly like one that passed.
        for text in [
            r#"{"type":"object","allOf":[{"required":["a"]}]}"#,
            r#"{"type":"object","properties":{"a":{"pattern":"^x$"}}}"#,
            r#"{"type":"object","properties":{"a":{"minLength":1}}}"#,
            r##"{"$ref":"#/definitions/x"}"##,
            r#"{"type":"object","properties":{"a":{"oneOf":[{"type":"string"}]}}}"#,
            r#"{"type":"object","patternProperties":{"^a$":{"type":"string"}}}"#,
        ] {
            let error = Schema::parse("test.schema.json", text).unwrap_err();
            match error {
                SchemaError::Unsupported { keyword, .. } => {
                    assert!(!keyword.is_empty());
                }
                other => panic!("{text} was accepted: {other:?}"),
            }
        }
    }

    #[test]
    fn a_keyword_this_module_does_not_understand_in_a_nested_subschema_is_refused() {
        // The nested walk is what makes the previous test more than a check of
        // the top level.
        let error = Schema::parse(
            "test.schema.json",
            r#"{"type":"object","properties":{"a":{"type":"array","items":{"contains":{"type":"string"}}}}}"#,
        )
        .unwrap_err();
        match error {
            SchemaError::Unsupported { path, keyword, .. } => {
                assert_eq!(path, "a.items");
                assert_eq!(keyword, "contains");
            }
            other => panic!("expected an unsupported keyword, got {other:?}"),
        }
    }

    #[test]
    fn a_supported_keyword_in_the_wrong_shape_is_refused() {
        for text in [
            r#"{"type":"object","required":"id"}"#,
            r#"{"type":"object","properties":[]}"#,
            // The draft-07 tuple form. Draft 2020-12 replaced it with
            // `prefixItems`, so an array here is a schema from the wrong draft
            // and this module would read it as a single schema.
            r#"{"type":"array","items":[{"type":"string"}]}"#,
            r#"{"type":"object","properties":{"a":{"enum":[]}}}"#,
            r#"{"type":"object","additionalProperties":{"type":"string"}}"#,
        ] {
            assert!(
                Schema::parse("test.schema.json", text).is_err(),
                "{text} was accepted"
            );
        }
    }

    #[test]
    fn a_format_this_module_cannot_check_is_refused() {
        // Silently treating it as an annotation is how a reader comes to
        // believe an email address or a URI was validated.
        let error = Schema::parse("test.schema.json", r#"{"format":"email"}"#).unwrap_err();
        assert!(error.to_string().contains("email"), "{error}");
    }

    #[test]
    fn schema_text_that_is_not_json_is_an_error_rather_than_an_empty_schema() {
        let error = Schema::parse("test.schema.json", "{not json").unwrap_err();
        assert!(matches!(error, SchemaError::NotJson { .. }));
    }

    #[test]
    fn every_violation_names_the_schema_it_came_from() {
        let schema = schema(r#"{"type":"object","required":["id"]}"#);
        let found = schema.validate(&json!({}));
        assert_eq!(found[0].schema, "test.schema.json");
    }

    #[test]
    fn every_supported_keyword_is_either_enforced_or_an_annotation() {
        // The hole this closes: a keyword added to `SUPPORTED_KEYWORDS` without
        // an enforcement path would be parsed without complaint and then
        // ignored, which is a constraint that reads exactly like one that
        // passed. Each entry here is a schema using the keyword and a document
        // that violates it.
        let constraining: &[(&str, &str, serde_json::Value)] = &[
            ("type", r#"{"type":"string"}"#, json!(1)),
            ("required", r#"{"required":["a"]}"#, json!({})),
            (
                "properties",
                r#"{"properties":{"a":{"type":"string"}}}"#,
                json!({"a": 1}),
            ),
            (
                "items",
                r#"{"type":"array","items":{"type":"string"}}"#,
                json!(["ok", 1]),
            ),
            ("enum", r#"{"enum":["a","b"]}"#, json!("c")),
            ("minimum", r#"{"minimum":1}"#, json!(0)),
            (
                "format",
                r#"{"type":"string","format":"date-time"}"#,
                json!("not a timestamp"),
            ),
            (
                "additionalProperties",
                r#"{"properties":{"a":{"type":"string"}},"additionalProperties":false}"#,
                json!({"a": "ok", "b": 1}),
            ),
        ];

        for (keyword, text, instance) in constraining {
            let schema = schema(text);
            assert!(
                !schema.validate(instance).is_empty(),
                "\"{keyword}\" is listed as supported and is not enforced"
            );
        }

        // The rest are annotations, and an annotation is allowed to be the only
        // thing a schema says. Anything in neither list is a keyword nobody has
        // decided about.
        let described: Vec<&str> = constraining
            .iter()
            .map(|(keyword, _, _)| *keyword)
            .chain(ANNOTATION_KEYWORDS.iter().copied())
            .collect();
        for keyword in supported_keywords() {
            assert!(
                described.contains(&keyword),
                "\"{keyword}\" is supported, and this test does not say whether it \
                 constrains anything"
            );
        }
        assert_eq!(described.len(), supported_keywords().len());
    }

    #[test]
    fn the_supported_keyword_list_is_the_one_the_documentation_names() {
        // A prompt to update `docs/architecture/PROTOCOL.md` when this changes.
        assert_eq!(
            supported_keywords(),
            [
                "$schema",
                "additionalProperties",
                "description",
                "enum",
                "format",
                "items",
                "minimum",
                "properties",
                "required",
                "title",
                "type",
            ]
            .into_iter()
            .collect()
        );
    }
}
