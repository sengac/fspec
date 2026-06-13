//! `remove-schedule` — Rust port of `src/commands/schedule/remove-schedule.ts` (RPC-280).
//!
//! Removes a schedule from `spec/schedules.json`. Reads the file directly (does
//! NOT auto-create it — parity with the TS `removeSchedule`, which calls
//! `getSchedulesFilePath` and opens the file directly via
//! `fileManager.transaction`); a missing file yields the default empty
//! schedules map so the not-found branch fires. On success the entry is removed
//! with [`IndexMap::shift_remove`] (preserving the insertion order of the
//! surviving schedules — NOT `swap_remove`) and the file is written atomically.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/remove_schedule.rs` is JSON marshalling only.
//!
//! The `SchedulesData` file-model is kept LOCAL to this module (mirroring
//! `commands/list_schedules.rs`) rather than promoted to the shared
//! `types/mod.rs`, keeping the schedule commands parallel-safe.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// CLI arguments accepted by `remove-schedule`. Mirrors the TS
/// `ScheduleNameOptions` interface at `src/types/schedule.ts:107-110`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveScheduleArgs {
    name: String,
}

/// Local file-model for `spec/schedules.json` (kept local — see module docs).
#[derive(Debug, Deserialize, Serialize)]
struct SchedulesData {
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    schedules: IndexMap<String, Value>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl Default for SchedulesData {
    fn default() -> Self {
        Self {
            version: default_version(),
            schedules: IndexMap::new(),
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct RemoveScheduleResult {
    success: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveScheduleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-schedule",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("schedules.json");

    // Read directly; a missing file → default empty map so the not-found
    // branch fires (parity with the TS transaction read returning the default).
    let mut data = load_or_default(&path)?;

    // Not-found → error, no write occurs.
    if !data.schedules.contains_key(&args.name) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-schedule",
            reason: format!("Schedule '{}' does not exist", args.name),
        });
    }

    // shift_remove preserves the insertion order of the surviving schedules.
    data.schedules.shift_remove(&args.name);

    // Single atomic write (NO trailing newline; parity with TS
    // fileManager.transaction → JSON.stringify(data, null, 2)).
    write_json_atomic(&path, &data)?;

    let result = RemoveScheduleResult { success: true };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-schedule",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Read+parse `spec/schedules.json`; return the default empty structure if the
/// file is missing.
fn load_or_default(path: &Path) -> Result<SchedulesData, FspecCoreError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "schedules.json".to_string(),
            reason: e.to_string(),
        }),
        Err(_) => Ok(SchedulesData::default()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_name() {
        let a: RemoveScheduleArgs = serde_json::from_str(r#"{"name":"nightly-review"}"#).unwrap();
        assert_eq!(a.name, "nightly-review");
    }

    #[test]
    fn shift_remove_preserves_order() {
        let mut data = SchedulesData::default();
        data.schedules.insert("ZED".to_string(), Value::Null);
        data.schedules.insert("AAA".to_string(), Value::Null);
        data.schedules.insert("MID".to_string(), Value::Null);
        data.schedules.shift_remove("AAA");
        let keys: Vec<&String> = data.schedules.keys().collect();
        assert_eq!(keys, vec!["ZED", "MID"]);
    }
}
