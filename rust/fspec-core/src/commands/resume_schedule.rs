//! `resume-schedule` — Rust port of `src/commands/schedule/resume-schedule.ts`
//! (RPC-292).
//!
//! Sets a schedule's `status` back to `"active"` in `spec/schedules.json`. A
//! previously paused schedule begins triggering again on its cron expression.
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand converge on this single `run` function (RPC-003 §7/§11
//! two-front-doors).
//!
//! ## Behaviour parity with TypeScript (`resume-schedule.ts:54-72`)
//!
//! * Missing schedule → error `Schedule '<name>' does not exist`.
//! * Already-active schedule → error `Schedule '<name>' is already active`.
//! * Otherwise the target's `status` becomes `"active"`; every other field of
//!   the target AND every sibling schedule is preserved verbatim, and the
//!   insertion order of `schedules` is retained (`IndexMap`).
//!
//! ## APPROVED missing-file divergence (orchestration-state.md)
//!
//! The TS implementation reads `spec/schedules.json` through a transaction
//! that auto-creates a default `{schedules: {}}` and then indexes
//! `data.schedules[name]` — for a genuinely missing/empty file this yields the
//! SAME `Schedule '<name>' does not exist` error once the empty map is
//! consulted. The Rust port reads the file directly (NO auto-create); a
//! missing/empty file is modelled as an empty `schedules` map, so the
//! requested name is absent and the clean `does not exist` error is returned —
//! we deliberately do NOT reproduce any TypeError crash path.
//!
//! ## On-disk write
//!
//! The single mutation is flushed with [`write_json_atomic`] (no trailing
//! newline — parity with the `FileManager` `JSON.stringify(...)` writers used
//! by the schedule commands).

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// CLI arguments accepted by `resume-schedule`. Mirrors the TS
/// `ScheduleNameOptions` shape (`src/types/schedule.ts`) — a single required
/// schedule `name`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResumeScheduleArgs {
    name: String,
}

/// LOCAL projection of `spec/schedules.json`, kept module-private per the
/// batch-14 ownership rule (NOT added to the shared `types/mod.rs`). Mirrors
/// the `SchedulesData` shape used by `list_schedules`, but additionally
/// round-trips the top-level `version` field and any unknown top-level keys
/// (via `#[serde(flatten)]`) so a pause never drops data the writer did not
/// model.
///
/// `schedules` is an `IndexMap` so the on-disk declaration order of schedule
/// keys is preserved across the read→mutate→write cycle.
#[derive(Debug, Default, Deserialize, Serialize)]
struct SchedulesData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<Value>,
    #[serde(default)]
    schedules: IndexMap<String, Value>,
    #[serde(flatten)]
    extra: IndexMap<String, Value>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so the
/// same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ResumeScheduleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "resume-schedule",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("schedules.json");

    // Read-directly, NO auto-create. A missing or empty file is modelled as an
    // empty `schedules` map so the name lookup below yields the clean
    // "does not exist" error (the APPROVED divergence).
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    let mut data: SchedulesData = if raw.trim().is_empty() {
        SchedulesData::default()
    } else {
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "spec/schedules.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?
    };

    // Validate the schedule exists (parity with resume-schedule.ts:62-64).
    let schedule =
        data.schedules
            .get_mut(&args.name)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "resume-schedule",
                reason: format!("Schedule '{}' does not exist", args.name),
            })?;

    // Validate it is not already active (parity with resume-schedule.ts:66-68).
    let current_status = schedule.get("status").and_then(Value::as_str).unwrap_or("");
    if current_status == "active" {
        return Err(FspecCoreError::InvalidArgs {
            command: "resume-schedule",
            reason: format!("Schedule '{}' is already active", args.name),
        });
    }

    // Mutate ONLY the status field, preserving every sibling field verbatim.
    let obj = schedule
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "resume-schedule",
            reason: format!("Schedule '{}' is not a JSON object", args.name),
        })?;
    obj.insert("status".to_string(), Value::String("active".to_string()));

    // Single atomic write (no trailing newline).
    write_json_atomic(&path, &data)?;

    let result = json!({
        "success": true,
        "message": format!("✓ Schedule '{}' resumed successfully", args.name),
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "resume-schedule",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_name() {
        let a: ResumeScheduleArgs = serde_json::from_str(r#"{"name":"nightly"}"#).unwrap();
        assert_eq!(a.name, "nightly");
    }

    #[test]
    fn schedules_data_roundtrips_version_and_order() {
        let raw =
            r#"{"version":"1.0.0","schedules":{"b":{"status":"active"},"a":{"status":"active"}}}"#;
        let data: SchedulesData = serde_json::from_str(raw).unwrap();
        // Insertion order preserved: b before a.
        let keys: Vec<&String> = data.schedules.keys().collect();
        assert_eq!(keys, vec!["b", "a"]);
        assert_eq!(data.version, Some(Value::String("1.0.0".to_string())));
    }
}
