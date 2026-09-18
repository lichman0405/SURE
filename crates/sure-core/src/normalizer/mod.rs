//! Source-specific event normalizers.
//!
//! Each integration adapter (Cursor, Claude Code, etc.) produces raw events in
//! its own shape. The normalizers in this module convert those raw events into
//! the SURE event protocol (`EventEnvelope`).

pub mod cursor;
