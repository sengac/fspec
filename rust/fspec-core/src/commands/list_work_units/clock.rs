//! The clock and local time zone a board query reads (CLI-017).
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! `run` queries with [`BoardClock::system`]; tests call `run_at` with
//! [`BoardClock::fixed`] so relative times and local days never depend on
//! the wall clock or the machine's zone (TEST-042).

use chrono::{DateTime, Utc};

/// The time zone local times and local days are computed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardZone {
    /// The machine's local zone (honours `TZ`).
    Local,
    /// A named IANA zone, e.g. `Australia/Brisbane`.
    Named(chrono_tz::Tz),
}

/// "Now" and the local zone for one board query.
#[derive(Debug, Clone, Copy)]
pub struct BoardClock {
    pub now: DateTime<Utc>,
    pub zone: BoardZone,
}

impl BoardClock {
    /// The system clock in the machine's local zone.
    pub fn system() -> Self {
        Self {
            now: Utc::now(),
            zone: BoardZone::Local,
        }
    }

    /// A fixed clock in a given zone.
    pub fn fixed(now: DateTime<Utc>, zone: BoardZone) -> Self {
        Self { now, zone }
    }
}

use chrono::{FixedOffset, LocalResult, NaiveDate, NaiveDateTime, TimeZone};

impl BoardClock {
    /// The fixed offset the local zone has at `at`.
    fn offset_at(&self, at: &DateTime<Utc>) -> FixedOffset {
        match self.zone {
            BoardZone::Local => at.with_timezone(&chrono::Local).fixed_offset().timezone(),
            BoardZone::Named(tz) => at.with_timezone(&tz).fixed_offset().timezone(),
        }
    }

    /// `at` in the local zone, e.g. `2026-11-07T16:41:03+10:00`.
    pub(super) fn local_text(&self, at: &DateTime<Utc>) -> String {
        at.with_timezone(&self.offset_at(at))
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string()
    }

    /// The local calendar day of `at`.
    pub(super) fn local_date(&self, at: &DateTime<Utc>) -> NaiveDate {
        at.with_timezone(&self.offset_at(at)).date_naive()
    }

    /// A local wall-clock time as UTC. An ambiguous time (a repeated hour
    /// when daylight saving ends) is its earlier instant; a time that does
    /// not exist in the zone (skipped when daylight saving starts) is an
    /// error.
    pub(super) fn local_to_utc(&self, local: &NaiveDateTime) -> Result<DateTime<Utc>, String> {
        let resolved = match self.zone {
            BoardZone::Local => map_result(chrono::Local.from_local_datetime(local)),
            BoardZone::Named(tz) => map_result(tz.from_local_datetime(local)),
        };
        resolved.ok_or_else(|| format!("{local} does not exist in the local time zone"))
    }
}

fn map_result<Tz: TimeZone>(r: LocalResult<DateTime<Tz>>) -> Option<DateTime<Utc>> {
    match r {
        LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => Some(t.with_timezone(&Utc)),
        LocalResult::None => None,
    }
}
