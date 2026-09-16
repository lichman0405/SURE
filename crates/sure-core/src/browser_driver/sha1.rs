//! SHA-1, written here for exactly one caller, and checked against published
//! answers rather than against itself.
//!
//! **Nothing SURE stores, compares or fingerprints is a SHA-1.** This module is
//! not reachable from the domain, the protocol or any check, and no digest it
//! produces is ever written down. It exists because RFC 6455 defines the
//! WebSocket handshake's `Sec-WebSocket-Accept` response header as
//! `base64(SHA-1(key ++ GUID))` over a key the client chose, and a client that
//! cannot compute that value cannot tell a WebSocket server from anything else
//! that happens to answer on the port it connected to.
//!
//! # Why a hand-rolled digest is the right call here, when it was refused elsewhere
//!
//! The workspace manifest refuses to hand-write SHA-256 for fingerprints, and
//! the reason it gives is precise: *"the failure mode of a home-rolled SHA-256
//! is a plausible-looking wrong digest, which is silent."* **That reason does not
//! hold here, and the difference is worth stating rather than assuming.**
//!
//! A fingerprint's digest is computed once and stored, and it is compared
//! against a value computed by a *different* run — possibly under a different
//! implementation. Nothing on that path can tell a wrong digest from a right
//! one, because there is only one implementation in the conversation and the
//! value it produced is the value it is compared to. **This digest never leaves
//! the handshake it is computed in, and it is compared against a value produced
//! by somebody else on the other end of a socket.** A wrong SHA-1 here fails the
//! very next line of the very next handshake, with the two base64 strings in the
//! error, on every connection, forever — there is no version of this that is
//! quiet.
//!
//! The second half of the argument is the vectors. [`tests`] asserts the three
//! published FIPS 180-4 answers, and [`super::websocket`] asserts the one answer
//! RFC 6455 publishes for this exact construction. Those are values this
//! implementation did not choose.
//!
//! # What this is not
//!
//! **SHA-1 is broken for collision resistance and this module does not change
//! that.** It is used here because a protocol this code has to speak defines the
//! value that way, and refusing to speak the protocol is not a security
//! improvement. Nothing in this workspace may use [`sha1`] for anything else,
//! and the way that is kept true is that the module is private to
//! [`super`] and named in one call site.

/// The SHA-1 digest of `bytes`, as FIPS 180-4 defines it.
///
/// **Padding first, then blocks.** The message is extended with a single `1`
/// bit, enough zero bits that its length is 56 modulo 64, and then the original
/// length in bits as a big-endian `u64` — which is why the length is captured
/// before the padding is appended rather than read off the padded message.
///
/// `wrapping_add` throughout is not defensive style: SHA-1 is defined over
/// addition modulo 2^32, so a wrapping add *is* the operation, and an
/// implementation that let it overflow-panic would be wrong on exactly the
/// inputs whose carry chains are longest.
#[must_use]
pub fn sha1(bytes: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let bits = (bytes.len() as u64).wrapping_mul(8);

    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bits.to_be_bytes());

    let (blocks, remainder) = padded.as_chunks::<64>();
    debug_assert!(
        remainder.is_empty(),
        "the padding above always produces whole blocks"
    );
    for block in blocks {
        let mut schedule = [0u32; 80];
        for (word, bytes) in block.as_chunks::<4>().0.iter().zip(schedule.iter_mut()) {
            *bytes = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..80 {
            schedule[index] = (schedule[index - 3]
                ^ schedule[index - 8]
                ^ schedule[index - 14]
                ^ schedule[index - 16])
                .rotate_left(1);
        }

        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, word) in schedule.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999_u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }

        for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }

    let mut digest = [0u8; 20];
    let (words, remainder) = digest.as_chunks_mut::<4>();
    debug_assert!(
        remainder.is_empty(),
        "five words of four bytes are the whole of a digest"
    );
    for (word, chunk) in state.iter().zip(words.iter_mut()) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The hex of a digest, so that a failure prints the value rather than an
    /// array a reader has to decode.
    fn hex(digest: [u8; 20]) -> String {
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// **The published answers, and the first two are NIST's own.** A digest
    /// implementation that is wrong in a way its author did not think of still
    /// has to reproduce these three, and the third is the one that exercises
    /// the padding rule: 56 bytes is the length at which the message no longer
    /// fits before the length field, so a `1` bit and a whole extra block are
    /// appended before it.
    #[test]
    fn the_published_sha1_vectors_are_reproduced_exactly() {
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(sha1(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    /// **The long one, and it is the only vector here that walks the schedule
    /// more than once.** A million `a`s is 15,625 blocks, so a bug in the
    /// message-schedule expansion — which is where a hand-written SHA-1 is
    /// wrong when it is wrong — shows up on block two and not on block one.
    #[test]
    fn a_million_characters_of_the_same_letter_hash_to_the_published_answer() {
        let million = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(sha1(&million)),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }

    /// **The padding boundary, where a hand-written SHA-1 is most often wrong,
    /// and these three answers are computed rather than remembered.**
    ///
    /// They are not published vectors: they are the digests **`hashlib` in
    /// Python 3 — OpenSSL's SHA-1, a different implementation from this one —
    /// produces for these three inputs**, which is the property that makes them
    /// worth asserting. 55 bytes is the longest message whose `1` bit and length
    /// field still fit in the same block; 56 is the shortest that forces the
    /// length field into a second one; 64 is a whole block on its own, where the
    /// padding is a second block that is almost entirely zeros. **The middle one
    /// is the boundary and the other two are one byte either side of it**, so a
    /// padding rule that is off by one byte cannot satisfy all three.
    ///
    /// **The 64-byte answer here was wrong the first time it was written down**,
    /// from memory, and it is recorded rather than quietly corrected because
    /// that is the failure this file's own documentation is about: the wrong
    /// value was plausible, it was the right shape and the right length, and
    /// nothing about reading it back would have caught it. What caught it was
    /// computing the four vectors with a second implementation before the first
    /// `cargo test` ran.
    #[test]
    fn the_padding_boundary_is_the_same_on_the_far_side_of_a_block() {
        assert_eq!(
            hex(sha1(&[0u8; 55])),
            "8e8832c642a6a38c74c17fc92ccedc266c108e6c"
        );
        assert_eq!(
            hex(sha1(&[0u8; 56])),
            "9438e360f578e12c0e0e8ed28e2c125c1cefee16"
        );
        assert_eq!(
            hex(sha1(&[0u8; 64])),
            "c8d7d0ef0eedfa82d2ea1aa592845b9a6d4b02b7"
        );
    }
}
