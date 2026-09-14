//! The one hash SURE computes, and the framing that keeps it unambiguous.
//!
//! # Why the framing is not decoration
//!
//! The obvious way to hash a list of things is to hash them one after another.
//! That is wrong, and it is wrong in the direction this product cares about: the
//! two files `ab` and `c` hash the same as the two files `a` and `bc`, so a
//! project edited from one state into the other would keep its fingerprint, and
//! evidence about the first state would be reported as current for the second.
//! Nothing about the run would look wrong.
//!
//! So every field is written as its length, then its bytes. A field boundary can
//! then never be moved, and two different lists of things are two different byte
//! strings. [`the_framing_separates_a_split_in_a_different_place`] is the test
//! that holds this.
//!
//! # Why a domain tag
//!
//! The first field of every hash is a name for the kind of hash it is. Two
//! different kinds of fingerprint therefore cannot produce the same value even
//! by coincidence, and a change to what goes into a digest is a change to its
//! name — so an old fingerprint and a new one over the same project can never be
//! equal. The tag carries a version number for exactly that reason.
//!
//! # Why SHA-256
//!
//! A non-cryptographic hash is faster and would be the wrong choice here. Two
//! different project states sharing a digest is a false green, and the project
//! being fingerprinted is written by a coding agent — an adversary choosing the
//! collision is in scope, not a hypothetical.
//!
//! `docs/architecture/FINGERPRINTING.md` is the rule; this module is where it
//! becomes bytes.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest as _, Sha256};

/// How much of a file is read at a time.
///
/// Large enough that the per-read overhead disappears on a big file, small
/// enough that hashing a project does not allocate in proportion to it.
const CHUNK: usize = 64 * 1024;

/// Bytes as lowercase hexadecimal.
///
/// Lowercase and zero-padded, so a digest is the same 64 characters whatever it
/// contains — a fingerprint that is sometimes 63 characters long would compare
/// unequal to itself, and a caller comparing digests as text has enough to get
/// wrong already.
pub(crate) fn hex(bytes: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut text = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// A hash under construction, in which every field says how long it is.
///
/// See the module comment for why. The builder methods take `&mut self` and
/// return it so a digest can be written as one expression, which is how the
/// fields stay in the order somebody reading the code sees.
pub(crate) struct Digest {
    hasher: Sha256,
}

impl Digest {
    /// Start a hash of a named kind.
    ///
    /// The name is the first field, so it cannot be confused with data. A caller
    /// that adds a field of its own starting with the name's bytes still gets a
    /// different hash, because the name's *length* is written first.
    pub(crate) fn new(domain: &str) -> Self {
        let mut digest = Self {
            hasher: Sha256::new(),
        };
        digest.field(domain);
        digest
    }

    /// Add one field: how long it is, then the bytes.
    pub(crate) fn field(&mut self, bytes: impl AsRef<[u8]>) -> &mut Self {
        let bytes = bytes.as_ref();
        self.hasher.update((bytes.len() as u64).to_be_bytes());
        self.hasher.update(bytes);
        self
    }

    /// Add a field that may be absent.
    ///
    /// Not the same hash as an empty field, which matters because an absent
    /// dirty digest means "nothing tracked changed" and an empty one would mean
    /// "a change list that happens to be empty" — the same distinction
    /// [`sure_domain::vocabulary::GitState`] makes with `Option`.
    pub(crate) fn optional(&mut self, value: Option<&str>) -> &mut Self {
        match value {
            Some(value) => self.field([1u8]).field(value),
            None => self.field([0u8]),
        }
    }

    /// The digest, as lowercase hexadecimal.
    pub(crate) fn finish(self) -> String {
        hex(self.hasher.finalize())
    }
}

/// What hashing a file produced.
#[derive(Debug)]
pub(crate) struct Hashed {
    /// The digest, as lowercase hexadecimal.
    pub(crate) hex: String,
    /// How many bytes were read.
    pub(crate) bytes: u64,
}

/// Why a file could not be hashed.
///
/// Neither variant names the file. The caller passed the path in and still has
/// it — and it is the caller's spelling of it, relative to the project, that
/// belongs in a message a person reads rather than the joined-up path this
/// function happened to open. Carrying both would be two answers to one
/// question.
#[derive(Debug)]
pub(crate) enum HashError {
    /// The file could not be opened or read.
    Io(std::io::Error),
    /// The file is bigger than the budget allows.
    TooLarge,
}

/// Hash a file's contents, reading at most `limit` bytes.
///
/// A file larger than the limit is an error rather than a hash of its first
/// `limit` bytes. The difference is everything: a digest of part of a file is a
/// digest that two different files can share, and answering with one would make
/// the fingerprint quietly claim to cover something it did not read.
///
/// **A link is followed.** `File::open` follows one, and the caller is the one
/// who knows whether the link or its target is the thing worth hashing. Every
/// caller in this module decides before calling.
pub(crate) fn hash_file(path: &Path, limit: u64) -> Result<Hashed, HashError> {
    let file = File::open(path).map_err(HashError::Io)?;
    // One byte past the limit, so a file of exactly `limit + 1` bytes is seen to
    // be over it rather than read to the end and silently truncated. The
    // addition saturates rather than wrapping, because a wrap would turn the
    // largest possible limit into a limit of zero.
    let mut reader = file.take(limit.saturating_add(1));
    let mut buffer = vec![0u8; CHUNK];
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;

    loop {
        let read = reader.read(&mut buffer).map_err(HashError::Io)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > limit {
            return Err(HashError::TooLarge);
        }
        hasher.update(&buffer[..read]);
    }

    Ok(Hashed {
        hex: hex(hasher.finalize()),
        bytes: total,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// One byte string, hashed and written out.
    ///
    /// Written here rather than kept as a function in the module, because the
    /// module has no caller for one: `hash_file` streams, and nothing in SURE
    /// hashes a buffer that is already in memory.
    fn digest_of(bytes: &[u8]) -> String {
        hex(Sha256::digest(bytes))
    }

    /// The published SHA-256 vectors.
    ///
    /// Testing a dependency's arithmetic is not the point; testing that *this*
    /// code feeds it the right bytes is. A transposed field, a digest written
    /// big-endian where the format is little, or a hex encoder that drops a
    /// leading zero all produce a plausible-looking 64-character string, and
    /// only a known answer says which one happened. The empty message is here
    /// because it is the one input a buffer bug survives.
    #[test]
    fn the_published_sha256_vectors_come_out_right() {
        let cases = [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(
                digest_of(input.as_bytes()),
                expected,
                "SHA-256 of {input:?}"
            );
        }

        // One million `a`, which is the vector that crosses many block
        // boundaries and would catch a chunking mistake that the short ones
        // cannot.
        assert_eq!(
            digest_of(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn a_digest_is_always_the_same_length() {
        // The zero-padding rule, which the vectors above cannot show: they are
        // all values whose first half-byte happens to be non-zero.
        for length in 0..512 {
            let text = digest_of(&vec![0u8; length]);
            assert_eq!(text.len(), 64, "{length} bytes produced {text}");
            assert!(
                text.chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "{text} is not lowercase hexadecimal"
            );
        }
    }

    #[test]
    fn the_framing_separates_a_split_in_a_different_place() {
        // The reason every field carries its length. Without it these two are
        // the same bytes into the hasher, and a project that moved a character
        // across a file boundary would keep its fingerprint.
        let split_one = {
            let mut digest = Digest::new("test");
            digest.field("ab").field("c");
            digest.finish()
        };
        let split_two = {
            let mut digest = Digest::new("test");
            digest.field("a").field("bc");
            digest.finish()
        };
        assert_ne!(split_one, split_two);

        // And the same split twice is the same hash, or nothing could be
        // compared at all.
        let again = {
            let mut digest = Digest::new("test");
            digest.field("ab").field("c");
            digest.finish()
        };
        assert_eq!(split_one, again);
    }

    #[test]
    fn a_digest_of_one_kind_is_never_a_digest_of_another() {
        // The domain tag, tested with the same field contents on both sides so
        // that only the tag differs.
        let mut git = Digest::new("sure.git-fingerprint.v1");
        git.field("the same bytes");
        let mut content = Digest::new("sure.content-fingerprint.v1");
        content.field("the same bytes");
        assert_ne!(git.finish(), content.finish());
    }

    #[test]
    fn an_absent_field_is_not_an_empty_one() {
        let of = |value: Option<&str>| {
            let mut digest = Digest::new("test");
            digest.optional(value);
            digest.finish()
        };
        let (absent, empty, present) = (of(None), of(Some("")), of(Some("a digest")));
        assert_ne!(absent, empty);
        assert_ne!(absent, present);
        assert_ne!(empty, present);
    }

    #[test]
    fn a_file_hashes_to_the_same_value_as_its_bytes() {
        // The streaming path and the in-memory path, on the same content. Two
        // ways to compute one number is two ways to disagree, and the file path
        // is the one no vector covers.
        let root = sure_testkit::repository_root().join("target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("digest-fixture-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();

        let hashed = hash_file(&path, 1024).unwrap();
        // The published vector for "abc", written out rather than computed by
        // the helper above, so that this compares the streaming path against the
        // answer the world agrees on and not against another of SURE's own
        // functions.
        assert_eq!(
            hashed.hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hashed.bytes, 3);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_file_over_the_limit_is_refused_rather_than_truncated() {
        let root = sure_testkit::repository_root().join("target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("digest-limit-{}", std::process::id()));
        std::fs::write(&path, vec![b'x'; 100]).unwrap();

        // A file exactly at the limit is fine.
        assert!(hash_file(&path, 100).is_ok());
        // One byte over it is not, and neither is a file far over it. Both are
        // the same answer: two different files must not share a digest.
        for limit in [99, 10, 0] {
            match hash_file(&path, limit) {
                Err(HashError::TooLarge) => {}
                other => panic!("a file of 100 bytes was accepted with limit {limit}: {other:?}"),
            }
        }

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error_and_not_an_empty_hash() {
        // The false-green shape: hashing "no file" into the hash of nothing
        // would make a deleted file and an empty file the same state.
        let missing = sure_testkit::repository_root()
            .join("target/tmp")
            .join("this-file-does-not-exist-anywhere");
        assert!(matches!(hash_file(&missing, 1024), Err(HashError::Io(_))));
    }
}
