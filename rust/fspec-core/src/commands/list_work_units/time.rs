//! Time values of the board-query filters (CLI-017): an ISO-8601 date or
//! timestamp, a duration back from now, or `now`.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};

use super::clock::BoardClock;

/// Which end of a range a value bounds. A date-only `Until` value means the
/// end of that local day.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum End {
    Since,
    Until,
}

/// An inclusive start and exclusive end.
#[derive(Clone, Copy, Default)]
pub(super) struct Range {
    pub(super) since: Option<DateTime<Utc>>,
    pub(super) until: Option<DateTime<Utc>>,
}

impl Range {
    pub(super) fn is_set(&self) -> bool {
        self.since.is_some() || self.until.is_some()
    }

    pub(super) fn contains(&self, at: &DateTime<Utc>) -> bool {
        self.since.is_none_or(|s| *at >= s) && self.until.is_none_or(|u| *at < u)
    }
}

fn invalid(flag: &str, text: &str) -> String {
    format!(
        "Invalid time '{text}' for {flag}. Use YYYY-MM-DD, an ISO-8601 timestamp, a duration back from now (Nm, Nh, Nd or Nw, e.g. 30m, 6h, 2d, 1w) or now"
    )
}

fn duration(text: &str) -> Option<Duration> {
    let unit = text.chars().last()?;
    let digits = &text[..text.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: i64 = digits.parse().ok()?;
    match unit {
        'm' => Duration::try_minutes(n),
        'h' => Duration::try_hours(n),
        'd' => Duration::try_days(n),
        'w' => Duration::try_weeks(n),
        _ => None,
    }
}

const LOCAL_FORMATS: &[&str] = &["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"];

/// Parse one time value of `flag`.
pub(super) fn parse(
    flag: &str,
    text: &str,
    end: End,
    clock: &BoardClock,
) -> Result<DateTime<Utc>, String> {
    if text == "now" {
        return Ok(clock.now);
    }
    if let Some(back) = duration(text) {
        return clock
            .now
            .checked_sub_signed(back)
            .ok_or_else(|| invalid(flag, text));
    }
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        let day = match end {
            End::Since => date,
            End::Until => date.succ_opt().ok_or_else(|| invalid(flag, text))?,
        };
        let midnight = day
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| invalid(flag, text))?;
        return clock
            .local_to_utc(&midnight)
            .map_err(|e| format!("{} ({e})", invalid(flag, text)));
    }
    if let Ok(t) = DateTime::parse_from_rfc3339(text) {
        return Ok(t.with_timezone(&Utc));
    }
    for format in LOCAL_FORMATS {
        if let Ok(local) = NaiveDateTime::parse_from_str(text, format) {
            return clock
                .local_to_utc(&local)
                .map_err(|e| format!("{} ({e})", invalid(flag, text)));
        }
    }
    Err(invalid(flag, text))
}

/// The range of a `--<kind>-since` / `--<kind>-until` pair.
pub(super) fn range(
    kind: &str,
    since: Option<&str>,
    until: Option<&str>,
    clock: &BoardClock,
) -> Result<Range, String> {
    let bound = |text: Option<&str>, suffix: &str, end: End| {
        text.map(|t| parse(&format!("--{kind}-{suffix}"), t, end, clock))
            .transpose()
    };
    Ok(Range {
        since: bound(since, "since", End::Since)?,
        until: bound(until, "until", End::Until)?,
    })
}
