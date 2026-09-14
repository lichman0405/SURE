//! The complete variant list of every frozen enum.
//!
//! A frozen vocabulary is only frozen if something can visit all of it. Code
//! that iterates a subset — a schema check, a CLI help page, a fixture matrix —
//! silently loses coverage the moment a variant is added, and that loss looks
//! exactly like success.
//!
//! `variants!` puts the list next to the enum, and the wire-contract test turns
//! it into a compile-time obligation: the test matches on each enum with no
//! wildcard arm, so a variant that is missing from the list stops the build
//! rather than quietly going untested.
//!
//! The macro is public because the same obligation applies to enums outside this
//! crate — a config value list that a "use one of: ..." message is generated
//! from must not be able to drift away from the enum it describes.

/// Declare `ALL` for a unit-variant enum.
///
/// Only unit variants are supported. An enum with a payload cannot have a
/// complete value list without inventing values for its fields, and inventing
/// them would make the list a guess rather than a fact.
#[macro_export]
macro_rules! variants {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident),+ $(,)? }
    ) => {
        impl $name {
            $(#[$meta])*
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}

pub use variants;
