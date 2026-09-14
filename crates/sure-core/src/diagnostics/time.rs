//! When a diagnostic happened.
//!
//! UTC only, milliseconds only, formatted as RFC 3339. No time zone handling
//! and no calendar arithmetic beyond the civil date, because a diagnostic is
//! read next to a harness event and a check result, and all three have to agree
//! on an instant without anyone agreeing on a locale.
//!
//! The date conversion is done here rather than by a date library. It is
//! Howard Hinnant's `civil_from_days`, which is twenty lines and exactly
//! specified, against a dependency that would be carried for the rest of the
//! product's life. `docs/architecture/DIAGNOSTICS.md` records the trade and
//! when to revisit it — the moment a local time zone or calendar arithmetic is
//! needed, this type is the wrong tool and should be replaced rather than
//! extended.
//!
//! Values are supplied by the caller rather than read from the clock inside
//! [`super::Diagnostic`], so a test can assert on a fixed instant. `now` is the
//! only function here that reads the clock.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// An instant, in milliseconds since 1970-01-01T00:00:00Z, in UTC.
///
/// Negative values are instants before that, and are handled rather than
/// refused: a machine with a wrong clock is a real machine, and a diagnostic
/// with a strange timestamp is more useful than a missing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

/// Milliseconds in a second, a minute, an hour and a day.
const MS_PER_SECOND: i64 = 1000;
const SECONDS_PER_DAY: i64 = 86_400;

impl Timestamp {
    /// The current instant.
    ///
    /// A clock set before 1970 yields a negative instant rather than a panic or
    /// a wrap-around to a time in the future.
    #[must_use]
    pub fn now() -> Self {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(since) => Self(since.as_millis() as i64),
            Err(error) => {
                let before = error.duration();
                Self(-(before.as_millis() as i64))
            }
        }
    }

    /// An instant given directly.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// The instant as milliseconds since the epoch.
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    /// RFC 3339, UTC, with milliseconds: `2026-09-14T09:10:56.827Z`.
    ///
    /// Always the same width, so lines can be compared and a log can be read
    /// down the column.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `div_euclid` and `rem_euclid` floor rather than truncate, which is
        // what makes a pre-1970 instant land on the right day. Truncating would
        // put -1 ms at 23:59:59.999 on the *same* day instead of the one
        // before.
        let seconds = self.0.div_euclid(MS_PER_SECOND);
        let millis = self.0.rem_euclid(MS_PER_SECOND);
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY);

        let (year, month, day) = civil_from_days(days);
        let hour = second_of_day / 3600;
        let minute = second_of_day % 3600 / 60;
        let second = second_of_day % 60;

        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
        )
    }
}

/// The civil date for a count of days since 1970-01-01.
///
/// Howard Hinnant's algorithm, which is valid for the whole `i64` range and has
/// no branch for leap years at the call site: the era/`yoe` arithmetic absorbs
/// them.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Days from 0000-03-01, which starts an era that begins on a leap day, so
    // February is not a special case.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    // January and February belong to the next year in this arrangement.
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_the_epoch() {
        assert_eq!(
            Timestamp::from_millis(0).to_string(),
            "1970-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn known_instants_are_rendered_correctly() {
        // Values with meanings that can be checked by hand rather than against
        // this implementation's own previous output.
        for (millis, expected) in [
            (946_684_800_000, "2000-01-01T00:00:00.000Z"),
            (1_234_567_890_000, "2009-02-13T23:31:30.000Z"),
            (1_700_000_000_000, "2023-11-14T22:13:20.000Z"),
            // The leap day itself: everything after it in the year depends on
            // the arithmetic getting February right.
            (1_709_164_800_000, "2024-02-29T00:00:00.000Z"),
            (1_709_251_200_000, "2024-03-01T00:00:00.000Z"),
        ] {
            assert_eq!(
                Timestamp::from_millis(millis).to_string(),
                expected,
                "{millis} was rendered wrong"
            );
        }
    }

    #[test]
    fn a_century_that_is_not_a_leap_year_is_rendered_as_one() {
        // 1900 was not a leap year, and 2000 was. An implementation that only
        // divides by four gets 2100 wrong in the other direction, so this is
        // checked at both.
        assert_eq!(
            Timestamp::from_millis(-2_203_891_200_000).to_string(),
            "1900-03-01T00:00:00.000Z"
        );
        assert_eq!(
            Timestamp::from_millis(4_107_542_400_000).to_string(),
            "2100-03-01T00:00:00.000Z"
        );
    }

    #[test]
    fn an_instant_before_the_epoch_renders_as_the_day_before() {
        // The whole reason for `div_euclid`: truncating division would put
        // -1 ms one day later than it belongs.
        assert_eq!(
            Timestamp::from_millis(-1).to_string(),
            "1969-12-31T23:59:59.999Z"
        );
        assert_eq!(
            Timestamp::from_millis(-86_400_000).to_string(),
            "1969-12-31T00:00:00.000Z"
        );
    }

    #[test]
    fn milliseconds_are_carried_rather_than_truncated() {
        assert_eq!(
            Timestamp::from_millis(1_234_567_890_123).to_string(),
            "2009-02-13T23:31:30.123Z"
        );
    }

    #[test]
    fn every_rendered_instant_has_the_same_width() {
        // A log is read down the column, and a variable-width timestamp is the
        // reason a line of output does not line up.
        for millis in [0, 1, 999, -1, 1_700_000_000_000, 2_081_433_600_000] {
            let text = Timestamp::from_millis(millis).to_string();
            assert_eq!(text.len(), 24, "{text} is not the fixed width");
            assert!(text.ends_with('Z'), "{text} is not marked as UTC");
        }
    }

    #[test]
    fn consecutive_days_are_consecutive_dates() {
        // A sweep rather than a spot check: 1200 days covers two leap days and
        // the end of a month twelve times over.
        let start = Timestamp::from_millis(1_700_000_000_000);
        let mut previous = String::new();
        for day in 0..1200 {
            let millis = start.as_millis() + day * 86_400_000;
            let text = Timestamp::from_millis(millis).to_string();
            assert_ne!(text, previous, "two days rendered the same");
            assert!(
                text > previous || previous.is_empty(),
                "dates went backwards"
            );
            previous = text;
        }
    }

    #[test]
    fn now_is_a_plausible_instant() {
        // Not a clock test: just that `now` reads the clock rather than
        // returning a constant, and that it does not panic on this machine.
        let now = Timestamp::now();
        assert!(now.as_millis() > 1_600_000_000_000, "{now}");
    }
}
