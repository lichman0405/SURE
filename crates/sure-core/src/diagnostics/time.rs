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

impl Timestamp {
    /// Parse an RFC 3339 timestamp into a UTC millisecond instant.
    ///
    /// Accepts `YYYY-MM-DDTHH:MM:SS[.fff...]Z` and offsets `±HH:MM`.
    /// Fractional seconds may have any number of digits; only the first three
    /// (milliseconds) are kept. Returns `None` if the text is not a valid
    /// RFC 3339 timestamp or the instant does not fit in `i64` milliseconds.
    #[must_use]
    pub fn parse_rfc3339(s: &str) -> Option<Self> {
        let s = s.trim();

        // Split the date/time portion from the time-zone indicator. The
        // indicator is the *last* occurrence of one of these characters, because
        // '-' also appears as a date separator.
        let tz_start = s.rfind(['Z', '+', '-'].as_slice())?;
        let (datetime, tz) = s.split_at(tz_start);
        if tz.is_empty() {
            return None;
        }

        let tz_seconds: i64 = if let Some(rest) = tz.strip_prefix('Z') {
            if !rest.is_empty() {
                return None;
            }
            0
        } else {
            let sign: i64 = if tz.starts_with('+') { 1 } else { -1 };
            let rest = &tz[1..];
            if rest.len() != 5 || rest.as_bytes()[2] != b':' {
                return None;
            }
            let hours = rest[..2].parse::<i64>().ok()?;
            let minutes = rest[3..5].parse::<i64>().ok()?;
            if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
                return None;
            }
            sign * (hours * 3_600 + minutes * 60)
        };

        // Separate optional fractional seconds.
        let (date_time, fraction_ms) = if let Some(dot) = datetime.find('.') {
            let (dt, frac_with_dot) = datetime.split_at(dot);
            let frac_digits = &frac_with_dot[1..];
            if frac_digits.is_empty() {
                return None;
            }
            let mut ms = 0_i64;
            for (i, c) in frac_digits.chars().enumerate() {
                if i >= 3 {
                    break;
                }
                let digit = c.to_digit(10)? as i64;
                ms = ms * 10 + digit;
            }
            let missing_digits = 3_usize.saturating_sub(frac_digits.len());
            let ms = ms * 10_i64.pow(u32::try_from(missing_digits).ok()?);
            (dt, ms)
        } else {
            (datetime, 0)
        };

        // date_time must now be exactly "YYYY-MM-DDTHH:MM:SS".
        let bytes = date_time.as_bytes();
        if bytes.len() != 19
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes[10] != b'T'
            || bytes[13] != b':'
            || bytes[16] != b':'
        {
            return None;
        }

        let year = date_time[..4].parse::<i64>().ok()?;
        let month = date_time[5..7].parse::<i64>().ok()?;
        let day = date_time[8..10].parse::<i64>().ok()?;
        let hour = date_time[11..13].parse::<i64>().ok()?;
        let minute = date_time[14..16].parse::<i64>().ok()?;
        let second = date_time[17..19].parse::<i64>().ok()?;

        if !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || !(0..=23).contains(&hour)
            || !(0..=59).contains(&minute)
            || !(0..=59).contains(&second)
        {
            return None;
        }

        let days = civil_to_days(year, month, day);
        let local_seconds = days
            .checked_mul(SECONDS_PER_DAY)?
            .checked_add(hour.checked_mul(3_600)?)?
            .checked_add(minute.checked_mul(60)?)?
            .checked_add(second)?;
        let utc_seconds = local_seconds.checked_sub(tz_seconds)?;
        let millis = utc_seconds
            .checked_mul(MS_PER_SECOND)?
            .checked_add(fraction_ms)?;
        Some(Self::from_millis(millis))
    }
}

/// Convert a civil calendar date to the number of days since 1970-01-01.
///
/// This is the inverse of [`civil_from_days`]; together they round-trip every
/// date that fits in an `i64` day count. The algorithm places January and
/// February in the previous year so that February's leap-day handling is
/// absorbed by the year/era arithmetic.
fn civil_to_days(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let month = if month <= 2 { month + 12 } else { month };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = (153 * (month - 3) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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

    #[test]
    fn rfc3339_round_trips_through_display() {
        let text = "2026-09-14T09:10:56.827Z";
        let ts = Timestamp::parse_rfc3339(text).expect("valid timestamp parses");
        assert_eq!(ts.to_string(), text);
    }

    #[test]
    fn rfc3339_parses_offsets_to_utc() {
        // 2026-09-14T09:10:56.827+02:00 is two hours earlier in UTC.
        let with_offset = Timestamp::parse_rfc3339("2026-09-14T09:10:56.827+02:00").unwrap();
        let utc = Timestamp::parse_rfc3339("2026-09-14T07:10:56.827Z").unwrap();
        assert_eq!(with_offset, utc);

        let negative = Timestamp::parse_rfc3339("2026-09-14T07:10:56.827-05:00").unwrap();
        let expected = Timestamp::parse_rfc3339("2026-09-14T12:10:56.827Z").unwrap();
        assert_eq!(negative, expected);
    }

    #[test]
    fn rfc3339_parses_varied_fraction_lengths() {
        let no_frac = Timestamp::parse_rfc3339("2026-09-14T09:10:56Z").unwrap();
        let three_frac = Timestamp::parse_rfc3339("2026-09-14T09:10:56.000Z").unwrap();
        assert_eq!(no_frac, three_frac);

        let one_frac = Timestamp::parse_rfc3339("2026-09-14T09:10:56.100Z").unwrap();
        let two_frac = Timestamp::parse_rfc3339("2026-09-14T09:10:56.1000Z").unwrap();
        assert_eq!(one_frac, two_frac);
    }

    #[test]
    fn rfc3339_rejects_invalid_inputs() {
        assert!(Timestamp::parse_rfc3339("").is_none());
        assert!(Timestamp::parse_rfc3339("not-a-date").is_none());
        assert!(Timestamp::parse_rfc3339("2026-09-14 09:10:56Z").is_none());
        assert!(Timestamp::parse_rfc3339("2026-09-14T09:10:56").is_none());
        assert!(Timestamp::parse_rfc3339("2026-09-14T25:10:56Z").is_none());
        assert!(Timestamp::parse_rfc3339("2026-09-14T09:10:56+25:00").is_none());
    }

    #[test]
    fn rfc3339_round_trips_known_pre_epoch_instant() {
        let ts = Timestamp::from_millis(-1);
        let back = Timestamp::parse_rfc3339(&ts.to_string()).expect("rendered timestamp parses");
        assert_eq!(ts, back);
    }
}
