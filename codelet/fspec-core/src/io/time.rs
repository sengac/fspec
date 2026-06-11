//! ISO-8601 timestamp helpers — shared utility for the ported fspec
//! commands.
//!
//! Originally each ported command (`create_epic.rs`,
//! `create_prefix.rs`, `update_prefix.rs`, `add_dependencies.rs`,
//! `clear_dependencies.rs`, `remove_dependency.rs`, `register_tag.rs`,
//! `delete_tag.rs`, `update_tag.rs`, …) carried its own private
//! `iso8601_now()` + `epoch_to_ymdhms()` pair. The DRY-violation was
//! documented in the per-command rustdoc but never lifted. This module
//! is the lift.
//!
//! The implementation:
//! * Uses only `std::time::SystemTime` — no `chrono` dependency, keeps
//!   `fspec-core`'s footprint tight.
//! * Captures **millisecond** precision (`SystemTime::duration_since(UNIX_EPOCH).as_millis()`),
//!   matching TS `new Date().toISOString()` byte-for-byte. The earlier
//!   per-command helpers truncated to whole seconds and always emitted
//!   `.000Z`, which broke parity for any consumer comparing timestamp
//!   ordering across two writes in the same second.
//! * Renders the civil date via Howard Hinnant's `days_from_civil`
//!   inverse — pure stdlib, branch-light, well-tested in the rest of
//!   the workspace.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current UTC time as an ISO-8601 / RFC-3339 string with
/// millisecond precision, e.g. `"2026-06-10T18:04:14.025Z"`.
///
/// Falls back to the Unix epoch if the system clock cannot be read
/// (`SystemTime::duration_since` returns `Err` only when the clock is
/// before UNIX epoch, which is essentially never observable in
/// practice). The fallback keeps this function infallible so callers
/// don't have to propagate clock errors through every command.
pub fn iso8601_now() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format_iso8601_millis(millis)
}

/// Format a Unix-epoch millisecond count as a TS-parity ISO-8601 string.
///
/// Extracted so unit tests can exercise the formatter without sampling
/// the system clock.
pub fn format_iso8601_millis(epoch_millis: u128) -> String {
    let secs = (epoch_millis / 1_000) as u64;
    let ms = (epoch_millis % 1_000) as u32;
    let (year, month, day, h, m, s) = epoch_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{ms:03}Z")
}

/// Convert a Unix timestamp (whole seconds since UTC epoch) to
/// `(year, month, day, hour, min, sec)`. Days-since-epoch → civil date
/// via Howard Hinnant's algorithm.
pub fn epoch_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let seconds_of_day = (secs % 86_400) as u32;
    let h = seconds_of_day / 3_600;
    let m = (seconds_of_day % 3_600) / 60;
    let s = seconds_of_day % 60;

    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = (y + if month <= 2 { 1 } else { 0 }) as i32;

    (year, month, d, h, m, s)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn epoch_to_ymdhms_handles_epoch_zero() {
        let (y, mo, d, h, m, s) = epoch_to_ymdhms(0);
        assert_eq!((y, mo, d, h, m, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_to_ymdhms_handles_a_known_date() {
        // 2024-01-15T12:34:56Z → 1_705_322_096 seconds since epoch.
        // (verified via `date -d @1705322096 -u`)
        let (y, mo, d, h, m, s) = epoch_to_ymdhms(1_705_322_096);
        assert_eq!((y, mo, d, h, m, s), (2024, 1, 15, 12, 34, 56));
    }

    #[test]
    fn format_iso8601_millis_preserves_millisecond_precision() {
        // 2024-01-15T12:34:56.025Z → 1_705_322_096_025 millis.
        let s = format_iso8601_millis(1_705_322_096_025);
        assert_eq!(s, "2024-01-15T12:34:56.025Z");
    }

    #[test]
    fn format_iso8601_millis_zero_pads_milliseconds() {
        let s = format_iso8601_millis(1_705_322_096_001);
        assert!(s.ends_with(".001Z"), "got: {s}");

        let s = format_iso8601_millis(1_705_322_096_000);
        assert!(s.ends_with(".000Z"), "got: {s}");

        let s = format_iso8601_millis(1_705_322_096_999);
        assert!(s.ends_with(".999Z"), "got: {s}");
    }

    #[test]
    fn iso8601_now_has_canonical_24_byte_shape() {
        let s = iso8601_now();
        assert_eq!(s.len(), 24, "iso8601_now must be 24 bytes; got: {s}");
        assert!(s.ends_with('Z'));
        assert_eq!(s.as_bytes()[4], b'-');
        assert_eq!(s.as_bytes()[7], b'-');
        assert_eq!(s.as_bytes()[10], b'T');
        assert_eq!(s.as_bytes()[13], b':');
        assert_eq!(s.as_bytes()[16], b':');
        assert_eq!(s.as_bytes()[19], b'.');
    }
}
