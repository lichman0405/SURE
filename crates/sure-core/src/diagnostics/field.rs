//! The values a diagnostic may carry.
//!
//! A diagnostic line is not prose. It is a fixed message plus a set of named
//! values, and this module is what makes those values safe to write down.
//!
//! Two rules do the work, and both are structural rather than advisory.
//!
//! 1. **A field is built through [`Field::text`], which redacts.** There is no
//!    way to put a `String` into a record without passing through it, so
//!    `docs/security/SECRET_REDACTION.md`'s "redact before durable write" is a
//!    property of the type rather than a rule someone has to remember.
//! 2. **A field whose name says it holds a credential records that it had one,
//!    and never the value.** `Field::text("api_key", …)` discards the argument
//!    without inspecting it. This is the part that makes a secret unnecessary:
//!    [`Field::redacted`] records the presence of a credential with no value
//!    supplied at all, so no caller is ever *required* to hand one over to get
//!    an accurate line.
//!
//! The escaping is the other half. A value is written inside a quoted `key="…"`
//! span, with backslashes, quotes and control characters escaped, so a value
//! cannot end its own quote, start a new `key=`, or add a line that looks like
//! it came from SURE.

use std::fmt;

use crate::redact;

/// What is written instead of a value that was deliberately not recorded.
///
/// Distinct from [`REDACTED`], which means "this looked like a secret and was
/// replaced". This one means "this field is never recorded". A reader of a log
/// can tell a redaction they should investigate from an omission by design.
pub const NOT_RECORDED: &str = "<not recorded>";

/// One named value attached to a diagnostic.
///
/// The name is `&'static str` for the same reason the message is: a key has to
/// be a fixed part of SURE's vocabulary. A key built from project-controlled
/// text would let a project choose what a log line appears to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    key: &'static str,
    value: FieldValue,
}

/// The value of a [`Field`], after it has been made safe to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// Text, already redacted and about to be quoted.
    Text(String),
    /// A number. Never a string, so a count cannot smuggle text into a
    /// numeric-looking position.
    Number(i64),
    /// A boolean.
    Bool(bool),
    /// A value that was not recorded, deliberately.
    NotRecorded,
}

impl Field {
    /// A text field.
    ///
    /// The value is redacted unless the key itself names a credential, in which
    /// case it is not recorded at all — a credential whose *name* is known does
    /// not need its value, and running the pattern list over it would suggest
    /// the patterns are what protects it.
    #[must_use]
    pub fn text(key: &'static str, value: impl AsRef<str>) -> Self {
        if redact::looks_like_credential_name(key) {
            return Self::redacted(key);
        }
        Self {
            key,
            value: FieldValue::Text(redact::redact(value.as_ref())),
        }
    }

    /// A numeric field.
    #[must_use]
    pub const fn number(key: &'static str, value: i64) -> Self {
        Self {
            key,
            value: FieldValue::Number(value),
        }
    }

    /// A boolean field.
    #[must_use]
    pub const fn boolean(key: &'static str, value: bool) -> Self {
        Self {
            key,
            value: FieldValue::Bool(value),
        }
    }

    /// A field whose value is never written down.
    ///
    /// This is the constructor to reach for when the honest statement is "a
    /// credential was configured". It takes no value, so the code path that
    /// knows about a secret does not have to hold one to report accurately.
    #[must_use]
    pub const fn redacted(key: &'static str) -> Self {
        Self {
            key,
            value: FieldValue::NotRecorded,
        }
    }

    /// The field's name.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// The field's value, after redaction.
    #[must_use]
    pub const fn value(&self) -> &FieldValue {
        &self.value
    }
}

impl fmt::Display for Field {
    /// `key="value"`, `key=42`, `key=true` or `key=<not recorded>`.
    ///
    /// Text is always quoted, including text without spaces, so a reader never
    /// has to guess whether something was a single word.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key)?;
        f.write_str("=")?;
        match &self.value {
            FieldValue::Text(text) => write!(f, "\"{}\"", quote(text)),
            FieldValue::Number(number) => write!(f, "{number}"),
            FieldValue::Bool(value) => write!(f, "{value}"),
            FieldValue::NotRecorded => f.write_str(NOT_RECORDED),
        }
    }
}

/// Escape a value so it stays inside its own quoted span and inside its own
/// line.
///
/// Backslashes and quotes first, then control characters. That order is what
/// makes the result reversible: a newline that is genuinely in the value
/// becomes `\n`, while a literal two-character `\n` in the value becomes
/// `\\n`. Escaping the other way round would render both the same and leave the
/// reader unable to tell them apart.
fn quote(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    redact::escape_control_characters(&escaped)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_text_field_is_quoted() {
        assert_eq!(
            Field::text("path", "C:\\work\\project").to_string(),
            "path=\"C:\\\\work\\\\project\""
        );
        assert_eq!(Field::text("path", "/w/p").to_string(), "path=\"/w/p\"");
    }

    #[test]
    fn a_credential_shaped_key_never_records_its_value() {
        // The point of the test is the *argument*: a caller that passes the
        // secret anyway still gets it dropped, because the decision was made
        // from the name and not from the value.
        for key in [
            "api_key",
            "token",
            "client_secret",
            "password",
            "Authorization",
        ] {
            let field = Field::text(key, "sk-abcdefghijklmnopqrstuvwxyz01");
            assert_eq!(*field.value(), FieldValue::NotRecorded, "{key}");
            assert!(!field.to_string().contains("sk-"), "{key}");
        }
    }

    #[test]
    fn a_credential_can_be_reported_without_being_supplied() {
        // "No raw secret values are required in logs", stated as a test: the
        // constructor takes no value at all.
        let field = Field::redacted("provider_credential");
        assert_eq!(field.to_string(), "provider_credential=<not recorded>");
    }

    #[test]
    fn a_secret_shaped_value_is_redacted_even_under_a_harmless_key() {
        // The second line of defence, for a credential pasted into a field
        // that is allowed to hold one — a provider endpoint is the real case.
        let field = Field::text("endpoint", "https://alice:hunter2@api.example.com/v1");
        assert!(!field.to_string().contains("hunter2"), "{field}");
        assert!(field.to_string().contains(redact::REDACTED), "{field}");
    }

    #[test]
    fn a_value_cannot_end_its_own_quote_or_open_a_new_key() {
        let field = Field::text("reason", "no\" key=injected");
        let rendered = field.to_string();
        assert_eq!(rendered, "reason=\"no\\\" key=injected\"");
        // The quote inside the value is escaped, so the only *unescaped*
        // quotes are the two the field itself added.
        let unescaped = rendered
            .match_indices('"')
            .filter(|(index, _)| !rendered[..*index].ends_with('\\'))
            .count();
        assert_eq!(unescaped, 2, "{rendered}");
        assert!(rendered.starts_with("reason=\""));
        assert!(rendered.ends_with('"'), "{rendered}");
    }

    #[test]
    fn a_value_cannot_add_a_line_to_the_record() {
        let field = Field::text("reason", "first\nSURE: nothing was checked\r\n\tend");
        let rendered = field.to_string();
        assert!(!rendered.contains('\n'), "{rendered:?}");
        assert!(!rendered.contains('\r'), "{rendered:?}");
        assert_eq!(
            rendered,
            "reason=\"first\\nSURE: nothing was checked\\r\\n\\tend\""
        );
    }

    #[test]
    fn a_literal_backslash_escape_is_distinguishable_from_an_escaped_character() {
        // `\\n` is a backslash followed by an `n` that was in the value. `\n`
        // is a newline that was escaped. Collapsing them would make a log
        // unfaithful in exactly the way that matters: a value could look like
        // it contained a line break it did not have, or hide one it did.
        assert_eq!(quote("a\\nb"), "a\\\\nb");
        assert_eq!(quote("a\nb"), "a\\nb");
        assert_ne!(quote("a\\nb"), quote("a\nb"));
    }

    #[test]
    fn numbers_and_booleans_are_not_quoted_and_not_text() {
        assert_eq!(Field::number("exit_code", 1).to_string(), "exit_code=1");
        assert_eq!(Field::number("checks", -3).to_string(), "checks=-3");
        assert_eq!(Field::boolean("dirty", true).to_string(), "dirty=true");
    }

    #[test]
    fn the_key_is_available_unredacted() {
        // A field's name is SURE's own vocabulary, and a report that has to say
        // "which field was omitted" needs it.
        assert_eq!(Field::text("api_key", "secret").key(), "api_key");
    }
}
