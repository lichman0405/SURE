//! Base64, standard alphabet, padding, **encode only**.
//!
//! The WebSocket handshake needs two encodings and no decodings:
//! `Sec-WebSocket-Key` is sixteen random bytes written as text, and
//! `Sec-WebSocket-Accept` is a SHA-1 digest written as text. Both go in the
//! direction this module implements. Nothing arrives from the peer encoded, so
//! there is no decoder here to be wrong — and a decoder that is never called is
//! a decoder that is never tested, which is the reason not to write one "for
//! later".
//!
//! # Why not the `base64` crate
//!
//! The same reason [`super::sha1`] gives at length, and it is a smaller version
//! of it. The workspace manifest asks every dependency to earn its place, and
//! what this one buys is forty lines of a table and two shifts. The deciding
//! fact is not the size, though: it is that **the value this produces is checked
//! against a foreign implementation on the next line of the handshake.** A
//! server that did not receive the key this client meant to send responds with
//! something else, or fails the handshake outright, and the two strings are in
//! the error.
//!
//! [`encode`] is checked against RFC 4648's own published vectors, and
//! [`super::websocket`] checks the one answer RFC 6455 publishes for the
//! construction the handshake actually uses.

/// RFC 4648 section 4's alphabet: `A`–`Z`, `a`–`z`, `0`–`9`, `+`, `/`.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The pad character, which RFC 4648 section 4 requires when the input length is
/// not a multiple of three.
const PAD: char = '=';

/// `bytes`, encoded with the standard alphabet and padded.
///
/// **Three bytes in, four characters out**, most significant group first, which
/// is why the shifts run 18, 12, 6, 0: the first character carries the top six
/// bits of the 24-bit triple. A final group of one or two bytes is padded with
/// zeros to the same width and then written with as many `=` as bytes are
/// missing — one for a single leftover byte, two for a pair.
///
/// The signature takes `&[u8]` rather than `&str` because the key is random
/// bytes and not text, and a function that took a string would force the caller
/// to invent an encoding for arbitrary bytes before this one could encode them.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        // `chunks(3)` never yields an empty slice, so the first byte is always
        // there. The other two are read as absent rather than indexed, because
        // the last group is exactly where they run out.
        let first = u32::from(group[0]);
        let second = u32::from(group.get(1).copied().unwrap_or(0));
        let third = u32::from(group.get(2).copied().unwrap_or(0));
        let triple = (first << 16) | (second << 8) | third;

        encoded.push(char::from(ALPHABET[(triple >> 18) as usize & 0x3f]));
        encoded.push(char::from(ALPHABET[(triple >> 12) as usize & 0x3f]));
        for (index, shift) in [(1, 6), (2, 0)] {
            if group.len() > index {
                encoded.push(char::from(ALPHABET[(triple >> shift) as usize & 0x3f]));
            } else {
                encoded.push(PAD);
            }
        }
    }
    encoded
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// **RFC 4648 section 10's vectors, which are the ones the standard
    /// publishes so that an implementation can be checked against something
    /// other than itself.** The lengths are chosen rather than arbitrary: `""`,
    /// `"f"` and `"fo"` are the three shapes a final group can have, and the
    /// rest are the same shapes again with more than one group in front of
    /// them, which is what catches a carry that only goes wrong after the first
    /// triple.
    #[test]
    fn the_rfc_vectors_are_reproduced_exactly() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(input.as_bytes()), expected, "encoding {input:?}");
        }
    }

    /// **The lengths where a group runs out, against a second implementation.**
    ///
    /// The seven vectors above pin the three group shapes at a single group.
    /// What they do not pin is that the shapes stay right when a group *ends* an
    /// input several groups long — which is where a hand-written encoder with a
    /// carry or an off-by-one gets it wrong once and then gets it wrong
    /// silently.
    ///
    /// These answers were computed by **Python 3's `base64.b64encode`, a
    /// different implementation written by somebody else**, and are written out
    /// here so that running the test does not need Python. The input is `i %
    /// 251` rather than a constant, so a shift that drops a byte's low bits
    /// changes the answer: 251 is coprime with 256 and not a multiple of 64, so
    /// consecutive positions differ in every six-bit group.
    ///
    /// **The one that got away is recorded rather than quietly dropped.** The
    /// triple `fb ef be` was written down first as `"+--+"`, from the same
    /// memory that produced a wrong SHA-1 digest in [`super::sha1`], and the
    /// second implementation says `"++++"` — all four six-bit groups are 62, so
    /// the four characters are identical, which is exactly the shape a reader
    /// does not predict and a memory fills in with something that looks more
    /// plausible. It is kept here as a vector because it is the case a person
    /// reading the code gets wrong, not the case the code gets wrong.
    #[test]
    fn the_lengths_where_a_group_ends_are_a_second_implementation_s_answers() {
        for (length, expected) in [
            (0, ""),
            (1, "AA=="),
            (2, "AAE="),
            (3, "AAEC"),
            (4, "AAECAw=="),
            (5, "AAECAwQ="),
            (6, "AAECAwQF"),
        ] {
            let input: Vec<u8> = (0..length).map(|index| index as u8).collect();
            assert_eq!(encode(&input), expected, "length {length}");
        }

        let long: Vec<u8> = (0..62).map(|index| index % 251).collect();
        assert_eq!(
            encode(&long),
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0="
        );
        let longer: Vec<u8> = (0..64).map(|index| index % 251).collect();
        assert_eq!(
            encode(&longer),
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+Pw=="
        );
        let longest: Vec<u8> = (0..65).map(|index| index % 251).collect();
        assert_eq!(
            encode(&longest),
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0A="
        );
    }

    /// **Every input length from zero to 130 at once, against a second
    /// implementation**, so that a rule that is right at the lengths above and
    /// wrong somewhere between them is still caught.
    ///
    /// A digest rather than 131 strings, and the trade is stated: this says *some
    /// length is wrong*, not *which one*, and the test above is what names one.
    /// What it buys is that the sweep is exhaustive — every length in the range,
    /// not a sample — which is the property a hand-written encoder needs checked
    /// and the one a curated list cannot check.
    ///
    /// `059200…` is the SHA-256, computed by **Python 3's `hashlib`**, of the
    /// concatenation of `b64encode(bytes(i % 251 for i in range(n)))` for `n`
    /// from 0 to 130 inclusive — **11,528 bytes of a foreign implementation's
    /// output**. `sha2` is already a dependency of this crate for the project
    /// fingerprint, so checking it here costs nothing and uses an implementation
    /// that is not this one.
    #[test]
    fn every_length_from_zero_to_one_hundred_and_thirty_agrees_with_a_second_implementation() {
        use sha2::{Digest, Sha256};

        let mut digest = Sha256::new();
        let mut total = 0;
        for length in 0..=130usize {
            let input: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let encoded = encode(&input);
            total += encoded.len();
            digest.update(encoded.as_bytes());
        }
        assert_eq!(
            total, 11_528,
            "the sweep did not produce the expected volume"
        );
        let digest: String = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            digest,
            "0592009241ffaa5e2904089fb2f907a72551386861abddfa636fc630fdf604b3"
        );
    }

    /// **The two characters that tell the standard alphabet from the URL-safe
    /// one**, which every vector above would survive a swap of.
    ///
    /// The standard alphabet puts `+` at 62 and `/` at 63; the URL-safe alphabet
    /// puts `-` and `_` there. A WebSocket key is allowed to contain either, and
    /// a server that received `-` where `+` was meant computes a different
    /// accept value and refuses the handshake — so this is not cosmetic.
    #[test]
    fn the_last_two_characters_of_the_alphabet_are_the_standard_ones() {
        assert_eq!(encode(&[0xfb, 0xef, 0xbe]), "++++");
        assert_eq!(encode(&[0xff, 0xff, 0xff]), "////");
        assert_eq!(encode(&[0xfb, 0xff, 0xff]), "+///");
        assert_eq!(encode(&[0x00, 0x00, 0x3e]), "AAA+");
        assert_eq!(encode(&[0x00, 0x00, 0x3f]), "AAA/");
    }
}
