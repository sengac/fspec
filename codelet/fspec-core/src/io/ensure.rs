//! `ensure-files` — load-or-init helpers for the canonical fspec state files.
//!
//! Rust port of the relevant subset of `src/utils/ensure-files.ts`.
//! Today this provides:
//!
//! - [`ensure_work_units_file`] — load `spec/work-units.json` (auto-create
//!   with the schema-version default if missing).
//! - [`ensure_prefixes_file`] — load `spec/prefixes.json` (auto-create empty).
//! - [`read_prefixes_or_empty`] — RPC-248 read-only twin of
//!   [`ensure_prefixes_file`]: returns `Ok(empty)` on ENOENT instead of
//!   auto-creating. Used by `list-prefixes` (which the TS implementation
//!   does NOT route through `ensurePrefixesFile`).
//! - [`read_work_units_or_empty`] — RPC-248 read-only twin of
//!   [`ensure_work_units_file`]: returns `Ok(empty)` on BOTH ENOENT and
//!   parse error, matching TypeScript's bare `catch {}` at
//!   `src/commands/list-prefixes.ts:57-63`.
//!
//! All helpers funnel through [`crate::io::locked_file`] for atomic I/O and
//! through [`crate::io::project_root::find_or_create_spec_directory`] for
//! consistent project-root discovery — so every ported command shares the
//! same canonical resolution.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::FspecCoreError;
use crate::io::locked_file::read_or_init_json;
use crate::io::project_root::find_or_create_spec_directory;
use crate::types::tags::TagsData;
use crate::types::work_unit::{EpicsData, PrefixesData, WorkUnitsData};

/// Load (or initialize) `spec/work-units.json` for the project rooted at `cwd`.
///
/// Mirrors `ensureWorkUnitsFile` in `src/utils/ensure-files.ts:17-56`. If the
/// file is missing it is created with the canonical initial structure
/// (version `0.7.1`, all 7 Kanban state arrays empty). If the file exists but
/// is malformed, a [`FspecCoreError::ParseJson`] is returned whose `Display`
/// contains the substring `"Failed to parse work-units.json"` for parity
/// with the TS error message.
pub fn ensure_work_units_file(cwd: &Path) -> Result<WorkUnitsData, FspecCoreError> {
    let spec = find_or_create_spec_directory(cwd)?;
    let path = spec.join("work-units.json");
    let default = WorkUnitsData::initial(iso8601_now());
    read_or_init_json(&path, &default, "work-units.json")
}

/// Load (or initialize) `spec/prefixes.json` for the project rooted at `cwd`.
///
/// Mirrors `ensurePrefixesFile` in `src/utils/ensure-files.ts:64-73`.
pub fn ensure_prefixes_file(cwd: &Path) -> Result<PrefixesData, FspecCoreError> {
    let spec = find_or_create_spec_directory(cwd)?;
    let path = spec.join("prefixes.json");
    let default = PrefixesData::initial();
    read_or_init_json(&path, &default, "prefixes.json")
}

/// Load (or initialize) `spec/tags.json` for the project rooted at `cwd`.
///
/// Mirrors `ensureTagsFile` in `src/utils/ensure-files.ts:75-200` (truncated)
/// — auto-creates the file with the canonical 9-category default
/// (Phase / Component / Feature Group / Technical / Platform / Priority /
/// Status / Testing / Automation) when missing, escalates malformed JSON via
/// [`FspecCoreError::ParseJson`] with `file = "tags.json"`.
///
/// Used by `list-tags` (RPC-251) and intended for reuse by `register-tag`,
/// `tag-stats`, `validate-tags`, and other future tag-aware commands.
pub fn ensure_tags_file(cwd: &Path) -> Result<TagsData, FspecCoreError> {
    let spec = find_or_create_spec_directory(cwd)?;
    let path = spec.join("tags.json");
    let default = TagsData::initial();
    read_or_init_json(&path, &default, "tags.json")
}

/// Read `spec/prefixes.json` WITHOUT auto-creating it.
///
/// RPC-248: the TypeScript `list-prefixes` command (see
/// `src/commands/list-prefixes.ts:44-54`) reads `spec/prefixes.json`
/// directly via `readFile` and treats `ENOENT` as "no prefixes
/// registered yet" — returning an empty list instead of creating the
/// file. This helper mirrors that semantic: if the file does not exist,
/// we return [`PrefixesData::initial()`] without touching the disk; if
/// the file exists but cannot be parsed, we ESCALATE the parse error
/// (parity with the TS `throw` at line 53, which only swallows the
/// ENOENT branch).
///
/// Use [`ensure_prefixes_file`] when you want the load-or-init behaviour
/// (e.g. `list-work-units` calls it purely for parity / side-effects).
pub fn read_prefixes_or_empty(cwd: &Path) -> Result<PrefixesData, FspecCoreError> {
    let path = cwd.join("spec").join("prefixes.json");
    if !path.exists() {
        return Ok(PrefixesData::initial());
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "read_prefixes_or_empty",
        source,
    })?;
    serde_json::from_str::<PrefixesData>(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "prefixes.json".to_string(),
        reason: e.to_string(),
    })
}

/// Read `spec/work-units.json` WITHOUT auto-creating it AND silently
/// returning an empty store on ANY failure (ENOENT or parse error).
///
/// RPC-248: the TypeScript `list-prefixes` command (see
/// `src/commands/list-prefixes.ts:57-63`) reads `spec/work-units.json`
/// inside a bare `try { ... } catch {}` block — meaning ENOENT *and* any
/// `JSON.parse` failure are silently swallowed so the prefix listing
/// renders with zero-progress counts instead of failing the whole
/// command. This helper preserves that semantic.
///
/// Use [`ensure_work_units_file`] when you want the load-or-init behaviour
/// AND want parse errors to escalate (e.g. `list-work-units` per RPC-253
/// rule [4]).
pub fn read_work_units_or_empty(cwd: &Path) -> Result<WorkUnitsData, FspecCoreError> {
    let path = cwd.join("spec").join("work-units.json");
    if !path.exists() {
        return Ok(WorkUnitsData::initial(iso8601_now()));
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        // Treat any I/O error here as "empty store" — matching TS's bare
        // `catch {}` which swallows both ENOENT (already handled above)
        // and unexpected file-system errors during the listing path.
        Err(_) => return Ok(WorkUnitsData::initial(iso8601_now())),
    };
    match serde_json::from_str::<WorkUnitsData>(&raw) {
        Ok(data) => Ok(data),
        Err(_) => Ok(WorkUnitsData::initial(iso8601_now())),
    }
}

/// Read `spec/epics.json` WITHOUT auto-creating it.
///
/// RPC-243: the TypeScript `list-epics` command reads `spec/epics.json`
/// directly via `readFile` and treats `ENOENT` as "no epics registered
/// yet" — returning an empty list instead of creating the file. This
/// helper mirrors that semantic: ENOENT → [`EpicsData::initial()`]
/// (no disk write); on-disk parse failure ESCALATES via
/// [`FspecCoreError::ParseJson`] with `file = "epics.json"`.
///
/// This is the read-only twin of (an as-yet-unwritten)
/// `ensure_epics_file` — list-epics deliberately does NOT auto-create
/// the file, matching the TS read-only path.
pub fn read_epics_or_empty(cwd: &Path) -> Result<EpicsData, FspecCoreError> {
    let path = cwd.join("spec").join("epics.json");
    if !path.exists() {
        return Ok(EpicsData::initial());
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "read_epics_or_empty",
        source,
    })?;
    serde_json::from_str::<EpicsData>(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "epics.json".to_string(),
        reason: e.to_string(),
    })
}

/// Best-effort RFC-3339 timestamp without a heavy time dep. Used purely for
/// the `meta.lastUpdated` field of newly-created files, which is informational.
fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Convert seconds-since-epoch → "YYYY-MM-DDThh:mm:ss.000Z" via the
    // standard library only. We use `chrono`-style formatting via manual
    // computation since fspec-core deliberately avoids the chrono dependency
    // for Phase 1.
    let (year, month, day, h, m, s) = epoch_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.000Z")
}

/// Convert a Unix timestamp to (year, month, day, hour, min, sec) in UTC.
/// Days-since-epoch → civil date via Howard Hinnant's algorithm.
fn epoch_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let seconds_of_day = (secs % 86_400) as u32;
    let h = seconds_of_day / 3_600;
    let m = (seconds_of_day % 3_600) / 60;
    let s = seconds_of_day % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
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
    use tempfile::TempDir;

    #[test]
    fn ensure_work_units_creates_canonical_initial_when_missing() {
        let tmp = TempDir::new().unwrap();
        let data = ensure_work_units_file(tmp.path()).unwrap();
        assert_eq!(data.version.as_deref(), Some("0.7.1"));
        assert!(data.work_units.is_empty());
        assert!(tmp.path().join("spec/work-units.json").exists());
    }

    #[test]
    fn ensure_work_units_returns_parse_json_for_malformed_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("spec")).unwrap();
        std::fs::write(tmp.path().join("spec/work-units.json"), "{ not json").unwrap();
        let err = ensure_work_units_file(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to parse work-units.json"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn ensure_prefixes_creates_empty_file_when_missing() {
        let tmp = TempDir::new().unwrap();
        let data = ensure_prefixes_file(tmp.path()).unwrap();
        assert!(data.prefixes.is_empty());
        assert!(tmp.path().join("spec/prefixes.json").exists());
    }

    #[test]
    fn epoch_to_ymdhms_handles_known_dates() {
        // Epoch 0 = 1970-01-01T00:00:00 UTC.
        assert_eq!(epoch_to_ymdhms(0), (1970, 1, 1, 0, 0, 0));
        // 2000-01-01T00:00:00 UTC = 946684800
        assert_eq!(epoch_to_ymdhms(946_684_800), (2000, 1, 1, 0, 0, 0));
    }

    // ---- RPC-248 read-only twins ----

    #[test]
    fn read_prefixes_or_empty_returns_empty_on_enoent_without_creating() {
        let tmp = TempDir::new().unwrap();
        let data = read_prefixes_or_empty(tmp.path()).unwrap();
        assert!(data.prefixes.is_empty());
        assert!(
            !tmp.path().join("spec/prefixes.json").exists(),
            "must NOT auto-create spec/prefixes.json"
        );
        assert!(
            !tmp.path().join("spec").exists(),
            "must NOT auto-create spec/ directory"
        );
    }

    #[test]
    fn read_prefixes_or_empty_escalates_parse_error() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("spec")).unwrap();
        std::fs::write(tmp.path().join("spec/prefixes.json"), "{ not json").unwrap();
        let err = read_prefixes_or_empty(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to parse prefixes.json"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn read_work_units_or_empty_returns_empty_on_enoent() {
        let tmp = TempDir::new().unwrap();
        let data = read_work_units_or_empty(tmp.path()).unwrap();
        assert!(data.work_units.is_empty());
        assert!(
            !tmp.path().join("spec/work-units.json").exists(),
            "must NOT auto-create spec/work-units.json"
        );
    }

    #[test]
    fn read_work_units_or_empty_swallows_parse_error_silently() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("spec")).unwrap();
        std::fs::write(tmp.path().join("spec/work-units.json"), "{ not json").unwrap();
        // Parity with TS bare `catch {}` — must NOT escalate.
        let data = read_work_units_or_empty(tmp.path()).unwrap();
        assert!(data.work_units.is_empty());
    }
}
