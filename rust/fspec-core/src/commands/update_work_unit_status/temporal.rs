//! Temporal-ordering gate for `update-work-unit-status`: feature and test files must be written after the
//! work unit entered the phase that owns them.

use std::path::Path;

use serde_json::Value;

use super::prefill::linked_feature_files;
use super::transitions::msg;
use crate::error::FspecCoreError;

pub(super) fn check_temporal_ordering(
    project_root: &Path,
    data: &crate::types::work_unit::WorkUnitsData,
    id: &str,
    new_status: &str,
) -> Result<(), FspecCoreError> {
    // FEAT-011 parity: compare file mtimes against the FIRST time the work unit
    // entered the gating state (`findStateHistoryEntry` returns the earliest
    // matching entry), mirroring `checkFileCreatedAfter`.
    //   - specifying→testing      → feature files vs entering `specifying`
    //   - testing→implementing     → test files vs entering `testing`
    let (file_type, state_name): (&str, &str) = match new_status {
        "testing" => ("feature", "specifying"),
        "implementing" => ("test", "testing"),
        _ => return Ok(()),
    };

    let wu = match data.work_units.get(id) {
        Some(w) => w,
        None => return Ok(()),
    };
    // FIRST stateHistory entry whose state matches (TS uses Array.find).
    let entry_ts = wu
        .extra
        .get("stateHistory")
        .and_then(Value::as_array)
        .and_then(|arr| {
            arr.iter()
                .find(|h| h.get("state").and_then(Value::as_str) == Some(state_name))
                .and_then(|h| h.get("timestamp").and_then(Value::as_str))
                .map(str::to_string)
        });
    let entry_ts = match entry_ts {
        Some(t) => t,
        None => return Ok(()),
    };
    // BUG-386: an unreadable entry time cannot pass the gate vacuously.
    let after_millis = parse_iso_millis(&entry_ts).ok_or_else(|| {
        msg(format!(
            "Cannot check {id}'s temporal ordering: its {state_name} entry timestamp {entry_ts:?} is not an ISO-8601 UTC time"
        ))
    })?;

    let files: Vec<std::path::PathBuf> = if file_type == "feature" {
        linked_feature_files(project_root, id)
            .map_err(|err| msg(format!("Cannot check the feature files of {id}: {err}")))?
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    } else {
        // BUG-435: coverage-mapped test files in any language, plus the
        // legacy TypeScript glob.
        let unreadable = |err: String| msg(format!("Cannot check the test files of {id}: {err}"));
        let mut files = find_test_files(project_root, id).map_err(unreadable)?;
        for path in
            super::temporal_mapped::mapped_test_files(project_root, id).map_err(unreadable)?
        {
            if !files.contains(&path) {
                files.push(path);
            }
        }
        files
    };

    // (file-display, file-mtime-iso, gap-minutes) for each violation.
    let mut violations: Vec<(String, String, i64)> = Vec::new();
    for path in &files {
        let file_millis = file_mtime_millis(path)
            .map_err(|err| msg(format!("Cannot check {id}'s temporal ordering: {err}")))?;
        if file_millis < after_millis {
            let file_iso = file_mtime_iso(path)
                .map_err(|err| msg(format!("Cannot check {id}'s temporal ordering: {err}")))?;
            let gap = (((after_millis - file_millis) as f64) / 1000.0 / 60.0).round() as i64;
            violations.push((path.display().to_string(), file_iso, gap));
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    let violation_details = violations
        .iter()
        .map(|(file, file_iso, gap)| {
            format!(
                "  - {file}\n    File modified: {file_iso}\n    Entered {state_name}: {entry_ts}\n    Gap: {gap} minutes BEFORE state entry"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let files_label = if file_type == "feature" {
        "Feature files"
    } else {
        "Test files"
    };
    let creation_rule = if file_type == "feature" {
        "Feature files must be created AFTER entering specifying state"
    } else {
        "Test files must be created AFTER entering testing state"
    };

    Err(msg(format!(
        r#"ACDD temporal ordering violation detected!

{files_label} were created/modified BEFORE entering {state_name} state.
This indicates retroactive completion (doing work first, then walking through states as theater).

Violations:
{violation_details}

ACDD requires work to be done IN each state, not BEFORE entering it:
  - {creation_rule}
  - Timestamps prove when work was actually done

To fix:
  1. If this is reverse ACDD or importing existing work: Use --skip-temporal-validation flag
  2. If this is a mistake: Delete {id} and restart with proper ACDD workflow
  3. If recovering from error: Move work unit back to {state_name} state and update files

For more info: See FEAT-011 "Prevent retroactive state walking""#
    )))
}

/// Recursively collect `src/**/__tests__/**/*.test.ts` files whose contents
/// reference the work-unit id (mirrors `findWorkUnitFiles(_, 'test', _)`).
///
/// BUG-386: a missing `src` means no test files; any other directory, entry
/// or test file that cannot be read is an error naming the path.
pub(super) fn find_test_files(
    project_root: &Path,
    id: &str,
) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    let root = project_root.join("src");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read {}: {error}", dir.display())),
        };
        for entry in entries {
            let path = entry
                .map_err(|error| format!("cannot read an entry of {}: {error}", dir.display()))?
                .path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !name.ends_with(".test.ts") {
                continue;
            }
            // Require a `__tests__` ancestor component (matches the glob).
            let in_tests_dir = path
                .components()
                .any(|c| c.as_os_str() == std::ffi::OsStr::new("__tests__"));
            if !in_tests_dir {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read test file {}: {error}", path.display()))?;
            if content.contains(id) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// File mtime as integer UNIX-epoch milliseconds.
pub(super) fn file_mtime_millis(path: &Path) -> Result<i64, String> {
    let dur = file_age(path)?;
    i64::try_from(dur.as_millis())
        .map_err(|_| format!("modification time of {} is out of range", path.display()))
}

/// Time since the UNIX epoch of `path`'s modification (BUG-386: an error
/// naming the path when it cannot be read).
fn file_age(path: &Path) -> Result<std::time::Duration, String> {
    let unreadable = |error: &dyn std::fmt::Display| {
        format!(
            "cannot read the modification time of {}: {error}",
            path.display()
        )
    };
    let mtime = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map_err(|error| unreadable(&error))?;
    mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| unreadable(&error))
}

/// Parse a `YYYY-MM-DDTHH:MM:SS(.sss)?Z` timestamp to UNIX-epoch milliseconds.
/// Returns `None` for any shape the temporal validator did not itself produce.
pub(super) fn parse_iso_millis(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hh: i64 = s.get(11..13)?.parse().ok()?;
    let mm: i64 = s.get(14..16)?.parse().ok()?;
    let ss: i64 = s.get(17..19)?.parse().ok()?;
    let millis: i64 = if s.len() >= 23 && bytes.get(19) == Some(&b'.') {
        s.get(20..23)?.parse().ok()?
    } else {
        0
    };
    let days = days_from_civil(year, month, day);
    Some(((days * 86_400 + hh * 3_600 + mm * 60 + ss) * 1_000) + millis)
}

/// Days since the UNIX epoch for a civil (proleptic Gregorian) date — the
/// inverse of [`epoch_secs_to_iso`]'s civil-from-days step (Howard Hinnant).
pub(super) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// File mtime as an ISO-8601 UTC string (lexicographically comparable).
pub(super) fn file_mtime_iso(path: &Path) -> Result<String, String> {
    let dur = file_age(path)?;
    Ok(epoch_secs_to_iso(dur.as_secs(), dur.subsec_millis()))
}

/// Convert UNIX epoch seconds to an ISO-8601 UTC timestamp (no external deps).
pub(super) fn epoch_secs_to_iso(secs: u64, millis: u32) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}
