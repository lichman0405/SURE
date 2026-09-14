//! What SURE says to a caller that names the protocol version it speaks.
//!
//! # Why this is one function and not a check at each call site
//!
//! Two places need to know whether SURE and a caller can talk: [`EventEnvelope`]
//! reading a document that has already arrived, and the CLI answering a caller
//! that asks *before* it sends anything. Written separately, they would drift —
//! and the drift would be invisible, because each would keep answering its own
//! question correctly. An adapter that SURE's CLI told "yes" and then refused at
//! the document would look like a broken adapter, which is the wrong place to
//! send somebody looking.
//!
//! So [`negotiate`] is the whole of the version rule, and both callers use it.
//! A test in `event.rs` holds the two to the same answer for every version
//! either could be given.
//!
//! # The rule is exact equality, and why there is no compatibility table
//!
//! This build reads one version: its own. It does not read an older one and it
//! does not guess at a newer one. A protocol version changes exactly when an
//! older reader would get the format *wrong* — adding an optional field does not
//! change it — so a version mismatch means a document that would be misread, and
//! a misread event becomes wrong evidence about a project. Refusing is the only
//! honest answer.
//!
//! There is no table of "older versions this build can still read" because there
//! has only ever been one version, and a table with a single row is a policy
//! invented to look thorough. When a second version exists, the list of versions
//! this build can still read arrives with it, and this module is where it goes.
//!
//! # Which side has to change
//!
//! The two directions are not the same answer, and a caller that has just been
//! refused needs to know which one it is. A caller speaking a *newer* protocol
//! was written against a SURE that does not exist yet here, so SURE is what has
//! to be updated. A caller speaking an *older* one is out of date and updating
//! SURE will not help. [`Handshake`]'s `Display` is that sentence, and it is the
//! only place it is written — a caller that prints its own version of it can
//! print a different one.
//!
//! [`EventEnvelope`]: crate::event::EventEnvelope

use std::fmt;

use crate::PROTOCOL_VERSION;

/// The outcome of comparing a caller's protocol version with this build's.
///
/// Carries both numbers rather than only the caller's, so that a renderer never
/// has to reach for a constant to say what happened, and so that a test can
/// build either direction without changing what this build speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handshake {
    /// The caller speaks the version this build speaks.
    Agreed {
        /// The one version both sides speak.
        version: u32,
    },
    /// The caller speaks a version below this build's. Updating SURE will not
    /// help; the caller is what has to move.
    CallerIsOlder {
        /// What the caller says it speaks.
        caller: u32,
        /// What this build speaks.
        sure: u32,
    },
    /// The caller speaks a version above this build's. This build cannot be
    /// taught it from here, so this is a SURE that has to be updated.
    CallerIsNewer {
        /// What the caller says it speaks.
        caller: u32,
        /// What this build speaks.
        sure: u32,
    },
}

/// Compare a caller's protocol version with this build's.
///
/// The whole of the version rule. See the module documentation for why a
/// mismatch is a refusal rather than something to work around.
#[must_use]
pub const fn negotiate(caller: u32) -> Handshake {
    if caller == PROTOCOL_VERSION {
        Handshake::Agreed {
            version: PROTOCOL_VERSION,
        }
    } else if caller < PROTOCOL_VERSION {
        Handshake::CallerIsOlder {
            caller,
            sure: PROTOCOL_VERSION,
        }
    } else {
        Handshake::CallerIsNewer {
            caller,
            sure: PROTOCOL_VERSION,
        }
    }
}

impl Handshake {
    /// Whether the two can talk.
    ///
    /// The single predicate the exit status, the frame's `outcome` and the
    /// choice of stream are all decided by, so that a script reading the status
    /// and a person reading the message cannot be told different things.
    #[must_use]
    pub const fn is_agreed(&self) -> bool {
        matches!(self, Self::Agreed { .. })
    }

    /// The protocol version this build speaks.
    #[must_use]
    pub const fn sure_speaks(&self) -> u32 {
        match self {
            Self::Agreed { version } => *version,
            Self::CallerIsOlder { sure, .. } | Self::CallerIsNewer { sure, .. } => *sure,
        }
    }

    /// The version the caller said it speaks, or `None` when nobody asked.
    ///
    /// `None` for [`Handshake::Agreed`] rather than the agreed number, because
    /// the agreed number is this build's and reporting it twice under two names
    /// invites a reader to treat them as two facts.
    #[must_use]
    pub const fn caller_speaks(&self) -> Option<u32> {
        match self {
            Self::Agreed { .. } => None,
            Self::CallerIsOlder { caller, .. } | Self::CallerIsNewer { caller, .. } => {
                Some(*caller)
            }
        }
    }
}

impl fmt::Display for Handshake {
    /// The sentence a person reads, and the only copy of it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agreed { version } => write!(
                f,
                "This build speaks harness protocol {version}, and so does the caller. \
                 They can talk."
            ),
            Self::CallerIsNewer { caller, sure } => write!(
                f,
                "The caller speaks harness protocol {caller} and this build speaks \
                 {sure}. SURE does not guess at a newer protocol, because an event it \
                 half-reads becomes wrong evidence about a project. Update SURE — this \
                 build cannot take what a newer caller sends."
            ),
            Self::CallerIsOlder { caller, sure } => write!(
                f,
                "The caller speaks harness protocol {caller} and this build speaks \
                 {sure}. SURE does not translate an older protocol, because an event it \
                 half-reads becomes wrong evidence about a project. Update the caller — \
                 a newer SURE will not accept protocol {caller} either."
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_caller_speaking_this_builds_version_is_the_only_agreement() {
        // Stated as a range rather than as one case, so that "agreed" cannot
        // quietly become "agreed or close enough". A version below this build's
        // is not a subset of it: the format changed for a reason.
        for caller in 0..=PROTOCOL_VERSION + 3 {
            let handshake = negotiate(caller);
            assert_eq!(
                handshake.is_agreed(),
                caller == PROTOCOL_VERSION,
                "protocol {caller} was judged {}",
                if handshake.is_agreed() {
                    "usable"
                } else {
                    "unusable"
                }
            );
        }
    }

    #[test]
    fn a_mismatch_says_which_side_has_to_move() {
        // The direction is the whole use of the message. A caller told only
        // "incompatible" will try the wrong fix first, and the wrong fix here is
        // installing a newer SURE, which changes nothing.
        // `saturating_sub` rather than `- 1` so that this cannot wrap before it
        // can report: at version 0 there is no older version, and the assertion
        // below is where that shows up.
        let older = negotiate(PROTOCOL_VERSION.saturating_sub(1));
        let newer = negotiate(PROTOCOL_VERSION + 1);

        assert!(matches!(older, Handshake::CallerIsOlder { .. }));
        assert!(matches!(newer, Handshake::CallerIsNewer { .. }));
        assert!(older.to_string().contains("Update the caller"), "{older}");
        assert!(newer.to_string().contains("Update SURE"), "{newer}");
        assert!(
            !older.to_string().contains("Update SURE —"),
            "an older caller was told to update SURE, which would not help: {older}"
        );
    }

    #[test]
    fn every_sentence_names_both_versions() {
        // A refusal that does not say what the other side speaks leaves the
        // person reading it with nothing to act on. The numbers are the one
        // thing they can compare with what they installed.
        for caller in [
            0,
            PROTOCOL_VERSION.saturating_sub(1),
            PROTOCOL_VERSION + 1,
            99,
        ] {
            let handshake = negotiate(caller);
            let sentence = handshake.to_string();
            assert!(
                sentence.contains(&caller.to_string()),
                "the sentence does not name the caller's version: {sentence}"
            );
            assert!(
                sentence.contains(&format!("speaks {}", PROTOCOL_VERSION))
                    || sentence.contains(&format!("protocol {PROTOCOL_VERSION}")),
                "the sentence does not name this build's version: {sentence}"
            );
        }
    }

    #[test]
    fn the_accessors_agree_with_the_variant() {
        let agreed = negotiate(PROTOCOL_VERSION);
        assert!(agreed.is_agreed());
        assert_eq!(agreed.sure_speaks(), PROTOCOL_VERSION);
        assert_eq!(agreed.caller_speaks(), None);

        let refused = negotiate(PROTOCOL_VERSION + 5);
        assert!(!refused.is_agreed());
        assert_eq!(refused.sure_speaks(), PROTOCOL_VERSION);
        assert_eq!(refused.caller_speaks(), Some(PROTOCOL_VERSION + 5));
    }

    #[test]
    fn the_agreed_sentence_does_not_read_like_a_complaint() {
        // `outcome` and the status decide that the run was fine, and the message
        // a person sees has to say the same thing. A success that reads like a
        // warning is how a user learns to ignore the command.
        let agreed = negotiate(PROTOCOL_VERSION).to_string();
        assert!(agreed.contains("can talk"), "{agreed}");
        assert!(!agreed.contains("Update"), "{agreed}");
        assert!(!agreed.contains("will not"), "{agreed}");
    }
}
