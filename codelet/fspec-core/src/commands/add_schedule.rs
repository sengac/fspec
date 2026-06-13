//! `add-schedule` — Rust port of `src/commands/schedule/add-schedule.ts` (RPC-191).
//!
//! Adds a new schedule (agent or shell) to `spec/schedules.json`. Validates the
//! schedule name (slug), cron expression (5-field standard cron), timezone
//! (IANA), and job-type-specific fields BEFORE any file write, then inserts the
//! new entry and writes atomically. The schedules file is auto-created with the
//! default `{version:'1.0.0', schedules:{}}` shape when missing (parity with the
//! TS `ensureSchedulesFile`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/add_schedule.rs` is JSON marshalling only — no domain
//! logic.
//!
//! ## On-disk shape
//!
//! The `SchedulesData` file-model is kept LOCAL to this module (mirroring
//! `commands/list_schedules.rs`, which keeps its own file-model local) rather
//! than promoted to the shared `types/mod.rs`. This keeps the schedule commands
//! parallel-safe and avoids cross-worker coupling. The `schedules` map is an
//! [`IndexMap`] so insertion order round-trips, and `#[serde(flatten)] extra`
//! preserves any unknown top-level fields.
//!
//! Each schedule entry's field declaration order (parity with TS object-literal
//! insertion order at `add-schedule.ts:106-130`) is:
//! `name, cron, timezone, overlapPolicy, status, lastRunAt, lastRunStatus,
//! createdAt, jobType, [role, prompt | command]`.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::utils::cron_validate::validate_default_5field;
use crate::utils::timezone_validate::validate_timezone as validate_iana_timezone;

/// CLI arguments accepted by `add-schedule`. Mirrors the TS
/// `AddScheduleOptions` interface at `src/types/schedule.ts:82-94`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddScheduleArgs {
    name: String,
    cron: String,
    timezone: String,
    job_type: String,
    #[serde(default)]
    overlap_policy: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    command: Option<String>,
}

/// Local file-model for `spec/schedules.json`. Kept local to this module
/// (NOT in the shared `types/mod.rs`) to stay parallel-safe.
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
struct AddScheduleResult {
    success: bool,
    schedule: Value,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddScheduleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-schedule",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ── Validation (ALL before any file write; order mirrors TS) ──────────

    // 1. Schedule name slug.
    validate_schedule_name(&args.name)?;

    // 2. Cron expression: exactly 5 whitespace-separated fields, then a
    //    standard cron parse via the cron-validate default-preset port.
    validate_cron(&args.cron)?;

    // 3. Timezone: valid IANA timezone (trimmed).
    validate_timezone(&args.timezone)?;

    // 4. jobType + type-specific required fields.
    match args.job_type.as_str() {
        "agent" => {
            let has_role = args.role.as_deref().map(str::is_empty) == Some(false);
            let has_prompt = args.prompt.as_deref().map(str::is_empty) == Some(false);
            if !has_role || !has_prompt {
                return Err(FspecCoreError::InvalidArgs {
                    command: "add-schedule",
                    reason: "Agent schedules require both role and prompt".to_string(),
                });
            }
        }
        "shell" => {
            let has_command = args.command.as_deref().map(str::is_empty) == Some(false);
            if !has_command {
                return Err(FspecCoreError::InvalidArgs {
                    command: "add-schedule",
                    reason: "Shell schedules require a command".to_string(),
                });
            }
        }
        other => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-schedule",
                reason: format!("Invalid jobType: {other}. Must be 'agent' or 'shell'."),
            });
        }
    }

    // ── Load (auto-create with default when missing) ──────────────────────
    let path = project_root.join("spec").join("schedules.json");
    let mut data = load_or_default(&path)?;

    // 5. Duplicate check (no write occurs on duplicate).
    if data.schedules.contains_key(&args.name) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-schedule",
            reason: format!("Schedule '{}' already exists", args.name),
        });
    }

    // ── Build the new entry with explicit field declaration order ─────────
    let overlap = args.overlap_policy.clone().unwrap_or_else(|| "skip".to_string());
    let now = iso8601_now();

    let mut entry = Map::new();
    entry.insert("name".to_string(), Value::String(args.name.clone()));
    entry.insert("cron".to_string(), Value::String(args.cron.clone()));
    entry.insert("timezone".to_string(), Value::String(args.timezone.clone()));
    entry.insert("overlapPolicy".to_string(), Value::String(overlap));
    entry.insert("status".to_string(), Value::String("active".to_string()));
    entry.insert("lastRunAt".to_string(), Value::Null);
    entry.insert("lastRunStatus".to_string(), Value::Null);
    entry.insert("createdAt".to_string(), Value::String(now));

    if args.job_type == "agent" {
        entry.insert("jobType".to_string(), Value::String("agent".to_string()));
        entry.insert(
            "role".to_string(),
            Value::String(args.role.clone().unwrap_or_default()),
        );
        entry.insert(
            "prompt".to_string(),
            Value::String(args.prompt.clone().unwrap_or_default()),
        );
    } else {
        entry.insert("jobType".to_string(), Value::String("shell".to_string()));
        entry.insert(
            "command".to_string(),
            Value::String(args.command.clone().unwrap_or_default()),
        );
    }

    let entry = Value::Object(entry);
    // `args.name` is owned and unused after this point, so move it into the map
    // (clippy::redundant_clone). `entry` IS reused below for the result, so its
    // clone here is required.
    data.schedules.insert(args.name, entry.clone());

    // ── Single atomic write (NO trailing newline; parity with TS
    //    fileManager.transaction → JSON.stringify(data, null, 2)). ─────────
    write_json_atomic(&path, &data)?;

    let result = AddScheduleResult {
        success: true,
        schedule: entry,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-schedule",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Read+parse `spec/schedules.json`; return the default empty structure if the
/// file is missing. (Auto-create on write happens via `write_json_atomic`.)
fn load_or_default(path: &Path) -> Result<SchedulesData, FspecCoreError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "schedules.json".to_string(),
            reason: e.to_string(),
        }),
        Err(_) => Ok(SchedulesData::default()),
    }
}

/// Regex-equivalent slug validation: `^[a-z0-9]+(-[a-z0-9]+)*$` after trim.
/// Mirrors `validateScheduleName` in `add-schedule.ts:35-46`: an empty/missing
/// name yields the distinct `Schedule name is required` message (TS guards
/// `!name` before the slug regex).
fn validate_schedule_name(name: &str) -> Result<(), FspecCoreError> {
    if name.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-schedule",
            reason: "Schedule name is required".to_string(),
        });
    }
    let trimmed = name.trim();
    if is_slug(trimmed) {
        return Ok(());
    }
    Err(FspecCoreError::InvalidArgs {
        command: "add-schedule",
        reason: format!(
            "Invalid schedule name '{name}'. Names must be lowercase, hyphenated slugs (e.g., 'nightly-review', 'daily-sync')."
        ),
    })
}

/// Mirror of `^[a-z0-9]+(-[a-z0-9]+)*$`: one-or-more lowercase-alnum groups
/// separated by single hyphens, no leading/trailing/double hyphens.
fn is_slug(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let groups: Vec<&str> = s.split('-').collect();
    groups
        .iter()
        .all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}

/// Validate a 5-field standard cron expression. Mirrors `validateCronExpression`
/// in `src/utils/validators/cron.ts`:
///   1. Reject empty/whitespace-only input with the distinct
///      "Cron expression is required and must be a string" message.
///   2. Enforce exactly 5 whitespace-run-separated fields (the TS
///      `split(/\s+/).length === 5` pre-check) with the "expected 5 fields"
///      message.
///   3. Delegate to the `cron-validate` `default`-preset port, prefixing the
///      joined field errors with "Invalid cron expression: " and appending the
///      "(Input cron: '<trimmed>')" suffix exactly as cron-validate does.
fn validate_cron(expression: &str) -> Result<(), FspecCoreError> {
    // TS guards `!expression` before trimming — an empty string yields the
    // dedicated "required" message (cron.ts:33-38).
    if expression.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-schedule",
            reason: "Cron expression is required and must be a string".to_string(),
        });
    }

    let trimmed = expression.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-schedule",
            reason: format!(
                "Invalid cron expression: expected 5 fields (minute hour dayOfMonth month dayOfWeek), got {}",
                parts.len()
            ),
        });
    }

    match validate_default_5field(trimmed) {
        Ok(()) => Ok(()),
        Err(field_errors) => {
            // cron-validate appends "(Input cron: '<cronString>')" to EACH
            // collected error (index.js:82-85), then validateCronExpression
            // joins them with "; " (cron.ts:69-71).
            let joined = field_errors
                .iter()
                .map(|e| format!("{e} (Input cron: '{trimmed}')"))
                .collect::<Vec<_>>()
                .join("; ");
            Err(FspecCoreError::InvalidArgs {
                command: "add-schedule",
                reason: format!("Invalid cron expression: {joined}"),
            })
        }
    }
}

/// Validate an IANA timezone string (trimmed) against the Node-enumerated list,
/// mirroring `validateTimezone` in `src/utils/validators/timezone.ts` —
/// including the "Did you mean" suggestion text.
fn validate_timezone(timezone: &str) -> Result<(), FspecCoreError> {
    validate_iana_timezone(timezone).map_err(|reason| FspecCoreError::InvalidArgs {
        command: "add-schedule",
        reason,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn slug_accepts_valid_names() {
        assert!(is_slug("nightly-review"));
        assert!(is_slug("daily-tests"));
        assert!(is_slug("a"));
        assert!(is_slug("weekly-deps-2"));
    }

    #[test]
    fn slug_rejects_invalid_names() {
        assert!(!is_slug("My Schedule"));
        assert!(!is_slug("Nightly"));
        assert!(!is_slug("-leading"));
        assert!(!is_slug("trailing-"));
        assert!(!is_slug("double--hyphen"));
        assert!(!is_slug(""));
    }

    #[test]
    fn cron_rejects_wrong_field_count() {
        let err = validate_cron("0 2 * *").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("expected 5 fields") && msg.contains("got 4"), "{msg}");
    }

    #[test]
    fn cron_accepts_standard_5_field() {
        assert!(validate_cron("0 2 * * *").is_ok());
        assert!(validate_cron("30 6 * * 1-5").is_ok());
    }

    #[test]
    fn timezone_accepts_valid_and_rejects_invalid() {
        assert!(validate_timezone("UTC").is_ok());
        assert!(validate_timezone("America/New_York").is_ok());
        let err = validate_timezone("Not/AZone").unwrap_err();
        assert!(err.to_string().contains("Invalid timezone"));
    }

    #[test]
    fn cron_value_errors_match_ts() {
        let err = validate_cron("99 2 * * *").unwrap_err().to_string();
        assert!(
            err.contains("Number '99' of minutes field is bigger than upper limit '59'.")
                && err.contains("(Input cron: '99 2 * * *')"),
            "{err}"
        );
    }

    #[test]
    fn cron_empty_yields_required_message() {
        let err = validate_cron("").unwrap_err().to_string();
        assert!(err.contains("Cron expression is required"), "{err}");
    }

    #[test]
    fn args_parse_camel_case() {
        let a: AddScheduleArgs = serde_json::from_str(
            r#"{"name":"x","cron":"0 2 * * *","timezone":"UTC","jobType":"shell","command":"echo"}"#,
        )
        .unwrap();
        assert_eq!(a.name, "x");
        assert_eq!(a.job_type, "shell");
        assert_eq!(a.command.as_deref(), Some("echo"));
    }
}
