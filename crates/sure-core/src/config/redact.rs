//! Keeping secrets out of anything SURE prints or stores.
//!
//! The primary defence is structural, not this module: [`crate::config::Config`]
//! has no field that accepts a credential, and a project-controlled
//! `sure.yaml` may not carry one (`docs/architecture/CONFIG_AUTHORITY.md`).
//! Redaction is the second line — it covers the value a user pasted into a
//! field that *is* allowed, such as a provider endpoint.
//!
//! `docs/security/SECRET_REDACTION.md` is explicit that detection is imperfect,
//! and this code does not pretend otherwise. It recognises a fixed list of
//! credential shapes and a fixed list of credential-ish names. It will miss
//! secrets written in shapes nobody has thought of yet, which is why the
//! structural rule above matters more than the patterns below.

/// What a redacted span is replaced with.
pub const REDACTED: &str = "***";

/// Name fragments that mark a setting as holding a credential rather than a
/// value.
///
/// Comparison is on the alphanumeric characters only, lowercased, so
/// `api_key`, `apiKey`, `API-KEY` and `api key` all match. The fragments are
/// deliberately long enough not to fire on ordinary setting names: `auth`
/// alone would match `author`, and `key` alone would match almost anything.
const CREDENTIAL_NAME_FRAGMENTS: &[&str] = &[
    "apikey",
    "secret",
    "password",
    "passwd",
    "passphrase",
    "token",
    "credential",
    "privatekey",
    "publickey",
    "accesskey",
    "authkey",
    "authheader",
    "authorization",
    "bearer",
    "clientsecret",
    "sessionkey",
    "signingkey",
];

/// Token shapes worth recognising, as `(prefix, minimum body length)`.
///
/// The minimums are the real ones: `sk-` credentials are never three
/// characters long, and requiring a plausible length is what keeps a sentence
/// containing "risk-free" from being redacted into nonsense.
const TOKEN_SHAPES: &[(&str, usize)] = &[
    ("sk-", 16),
    ("ghp_", 20),
    ("gho_", 20),
    ("ghu_", 20),
    ("ghs_", 20),
    ("ghr_", 20),
    ("github_pat_", 20),
    ("xoxb-", 10),
    ("xoxp-", 10),
    ("xoxa-", 10),
    ("xoxr-", 10),
    ("xoxs-", 10),
    ("ya29.", 20),
    ("AKIA", 16),
    ("ASIA", 16),
    ("eyJ", 30),
];

/// Whether a setting name looks like it holds a credential.
#[must_use]
pub fn looks_like_credential_name(name: &str) -> bool {
    let normalised: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    CREDENTIAL_NAME_FRAGMENTS
        .iter()
        .any(|fragment| normalised.contains(fragment))
}

/// Render a value for a diagnostic without risking a secret in the output.
///
/// Three passes, in order:
///
/// 1. a password inside a URL authority, `https://user:secret@host` → `**:*`;
/// 2. the value of anything assigned to a credential-ish name, `api_key=...`;
/// 3. a run of characters matching a known token shape.
///
/// Control characters are then escaped, so a value cannot forge structure in a
/// multi-line message by containing newlines of its own.
#[must_use]
pub fn redact_for_diagnostic(value: &str) -> String {
    let value = redact_url_authorities(value);
    let value = redact_assignments(&value);
    let value = redact_tokens(&value);
    escape_control_characters(&value)
}

/// Replace the userinfo of every `scheme://user:pass@host` with [`REDACTED`].
fn redact_url_authorities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find("://") {
        let (before, after) = rest.split_at(index + 3);
        out.push_str(before);
        // The authority ends at the first `/`, `?`, `#` or whitespace.
        let end = after
            .find(['/', '?', '#'])
            .or_else(|| after.find(char::is_whitespace))
            .unwrap_or(after.len());
        let (authority, tail) = after.split_at(end);
        match authority.find('@') {
            Some(at) => {
                out.push_str(REDACTED);
                out.push_str(&authority[at..]);
            }
            None => out.push_str(authority),
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Replace the value of `name=value` when `name` names a credential.
///
/// Only `=` is treated as an assignment here. YAML's `key: value` separator is
/// not included on purpose: this function runs over values that have already
/// been extracted from a parsed file, never over raw YAML text, so a colon in
/// the middle of a value is far more likely to be part of a URL or a timestamp
/// than an assignment.
fn redact_assignments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find('=') {
        let equals = cursor + offset;
        let start = name_start(text, equals);
        // Everything up to and including the `=` is copied verbatim either way,
        // so a name that begins before `cursor` is still reproduced correctly.
        out.push_str(&text[cursor..=equals]);
        if start < equals && looks_like_credential_name(&text[start..equals]) {
            out.push_str(REDACTED);
            cursor = value_end(text, equals + 1);
        } else {
            cursor = equals + 1;
        }
    }
    out.push_str(&text[cursor..]);
    out
}

/// Walk back from an `=` to the start of the name in front of it.
fn name_start(text: &str, equals: usize) -> usize {
    let mut start = equals;
    for (index, ch) in text[..equals].char_indices().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            start = index;
        } else {
            break;
        }
    }
    start
}

/// Where the value of an assignment ends: at a separator that cannot appear
/// inside a token, or at the end of the string.
fn value_end(text: &str, from: usize) -> usize {
    text[from..]
        .find(['&', ';', ',', '"', '\'', '#', ' '])
        .map_or(text.len(), |offset| from + offset)
}

/// Replace runs of characters that match a known credential shape.
fn redact_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let bytes = text.as_bytes();
    while cursor < bytes.len() {
        // A prefix only counts at a token boundary, so `ask-...` inside a word
        // is left alone.
        let at_boundary = cursor == 0 || !bytes[cursor - 1].is_ascii_alphanumeric();
        if at_boundary {
            if let Some(length) = token_length(&text[cursor..]) {
                out.push_str(REDACTED);
                cursor += length;
                continue;
            }
            if let Some(length) = bearer_token_length(&text[cursor..], &mut out) {
                out.push_str(REDACTED);
                cursor += length;
                continue;
            }
        }
        let ch = text[cursor..].chars().next().unwrap_or('\u{0}');
        out.push(ch);
        cursor += ch.len_utf8();
    }
    out
}

/// Length of a known credential shape at the start of `rest`, if any.
fn token_length(rest: &str) -> Option<usize> {
    for (prefix, minimum) in TOKEN_SHAPES {
        let Some(body) = rest.strip_prefix(prefix) else {
            continue;
        };
        let length = body
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
            .count();
        if length >= *minimum {
            return Some(prefix.len() + length);
        }
    }
    None
}

/// Length of the token after a `Bearer ` prefix, if `rest` starts with one.
///
/// The scheme word is copied to `out` first, so an `Authorization: Bearer x`
/// header stays readable while the credential disappears.
fn bearer_token_length(rest: &str, out: &mut String) -> Option<usize> {
    let lower = rest.get(..7)?.to_ascii_lowercase();
    if lower != "bearer " {
        return None;
    }
    let body = rest[7..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
        .count();
    if body < 8 {
        return None;
    }
    out.push_str(&rest[..7]);
    Some(7 + body)
}

/// Render control characters as escapes so a value cannot add lines to a
/// message, or start a new "line" that looks like it came from SURE.
fn escape_control_characters(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '\n' {
            out.push_str("\\n");
        } else if ch == '\r' {
            out.push_str("\\r");
        } else if ch == '\t' {
            out.push_str("\\t");
        } else if ch.is_control() {
            out.push_str(&format!("\\u{{{:04x}}}", ch as u32));
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_password_in_a_url_does_not_survive() {
        let redacted = redact_for_diagnostic("https://alice:hunter2@api.example.com/v1");
        assert!(!redacted.contains("hunter2"), "{redacted}");
        assert!(!redacted.contains("alice"), "{redacted}");
        assert_eq!(redacted, "https://***@api.example.com/v1");
    }

    #[test]
    fn a_url_without_credentials_is_left_readable() {
        // Diagnostics are for people. Redacting a harmless endpoint would make
        // the message useless and teach users to ignore the *** marker.
        assert_eq!(
            redact_for_diagnostic("https://api.example.com/v1"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            redact_for_diagnostic("https://api.example.com/v1?stream=true"),
            "https://api.example.com/v1?stream=true"
        );
    }

    #[test]
    fn an_assigned_secret_is_replaced_but_the_name_is_kept() {
        let redacted = redact_for_diagnostic("api_key=abcdef1234567890");
        assert_eq!(redacted, "api_key=***");

        let in_query = redact_for_diagnostic("https://h/v1?api_key=abcdef1234567890&x=1");
        assert!(!in_query.contains("abcdef1234567890"), "{in_query}");
        assert!(in_query.contains("api_key=***"), "{in_query}");
        assert!(in_query.contains("x=1"), "{in_query}");

        // Known gap, stated rather than implied: a flag whose value is a
        // separate argument is not covered. Treating `name value` as an
        // assignment would redact "is" out of "--api-key is required", and the
        // structural rule — the config model has no credential field — is what
        // actually protects this file, not the pattern list.
        let separated = redact_for_diagnostic("--api-key abcdef1234567890");
        assert_eq!(separated, "--api-key abcdef1234567890");
    }

    #[test]
    fn known_token_shapes_are_replaced() {
        for secret in [
            "sk-abcdefghijklmnopqrstuvwxyz01",
            "ghp_abcdefghijklmnopqrstuvwxyz01",
            "github_pat_abcdefghijklmnopqrstuv",
            "AKIAIOSFODNN7EXAMPLE",
            "xoxb-123456789012-abcdefghijkl",
            "ya29.abcdefghijklmnopqrstuvwx",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        ] {
            let redacted = redact_for_diagnostic(secret);
            assert_eq!(redacted, REDACTED, "{secret} was not recognised");
        }
    }

    #[test]
    fn an_authorization_header_keeps_its_scheme_word() {
        let redacted = redact_for_diagnostic("Authorization: Bearer abcdef1234567890");
        assert!(!redacted.contains("abcdef1234567890"), "{redacted}");
        assert!(redacted.contains("Bearer"), "{redacted}");
    }

    #[test]
    fn ordinary_prose_is_not_mangled() {
        // False positives are a real cost: a diagnostic nobody can read is a
        // diagnostic nobody acts on. `SECRET_REDACTION.md` asks for these to be
        // tested as well as the patterns themselves.
        for harmless in [
            "https://api.example.com/v1/models",
            "run the project's own commands",
            "inspect_only",
            "a task-free afternoon",
            "risk assessment",
            "the tokenizer settings",
            "project_intent.spec_path",
            "must_fix",
            "session_state_file",
            "public_key_notes",
        ] {
            assert_eq!(
                redact_for_diagnostic(harmless),
                harmless,
                "{harmless} should have been left alone"
            );
        }
    }

    #[test]
    fn a_too_short_run_after_a_prefix_is_not_a_token() {
        // `sk-1` is not a credential and redacting it would be noise.
        assert_eq!(redact_for_diagnostic("sk-1"), "sk-1");
        assert_eq!(redact_for_diagnostic("AKIA123"), "AKIA123");
    }

    #[test]
    fn a_prefix_inside_a_word_is_not_a_token_boundary() {
        let word = "risk-adjusted-abcdefghijklmnopqrstuvwxyz";
        assert_eq!(redact_for_diagnostic(word), word);
    }

    #[test]
    fn a_value_cannot_forge_extra_lines_in_a_message() {
        let redacted = redact_for_diagnostic("first\nSURE: nothing was checked\r\n\tend");
        assert!(!redacted.contains('\n'), "{redacted:?}");
        assert!(!redacted.contains('\r'), "{redacted:?}");
        assert_eq!(redacted, "first\\nSURE: nothing was checked\\r\\n\\tend");
    }

    #[test]
    fn credential_shaped_names_are_recognised_in_every_spelling() {
        for name in [
            "api_key",
            "apiKey",
            "API-KEY",
            "api key",
            "secret",
            "client_secret",
            "password",
            "PASSWD",
            "access_token",
            "github_token",
            "credentials",
            "private_key",
            "authorization",
        ] {
            assert!(
                looks_like_credential_name(name),
                "{name} should look like a credential"
            );
        }
    }

    #[test]
    fn ordinary_setting_names_are_not_flagged_as_credentials() {
        // Every name `sure.yaml` accepts, plus a few plausible neighbours. If a
        // real setting ever matches, users would be told their harmless setting
        // is a secret and would learn to distrust the warning.
        for name in [
            "privacy",
            "protection",
            "execution",
            "analysis",
            "project_intent",
            "checks",
            "report",
            "mode",
            "full_recording",
            "telemetry",
            "allow_dependency_install",
            "allow_network",
            "provider",
            "endpoint",
            "model",
            "goal",
            "spec_path",
            "existing_tests",
            "start_local_services",
            "browser_probe",
            "format",
            "author",
            "description",
            "notebook_name",
            "output_directory",
        ] {
            assert!(
                !looks_like_credential_name(name),
                "{name} is not a credential and must not be reported as one"
            );
        }
    }
}
