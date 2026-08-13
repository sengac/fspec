//! `list-schedules` — Rust port of `src/commands/schedule/list-schedules.ts` (RPC-250).
//!
//! Reads `spec/schedules.json` and returns a `{schedules, columns}` payload
//! suitable for the LLM dispatcher and the standalone fspec Rust binary's
//! clap subcommand. Both call sites converge on this single
//! `pub async fn run` so the agent-loop and the shell CLI share one
//! source-of-truth (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Behaviour parity with TypeScript (`src/commands/schedule/list-schedules.ts:30-70`):
//!
//! * Missing `spec/schedules.json` → `{schedules: [], columns: [...]}` with
//!   no auto-create (parity with the TS `existsSync(file)` short-circuit
//!   at `list-schedules.ts:36-49`).
//! * Successful read AND successful JSON parse →
//!   `{schedules: [<entry>, ...], columns: [...]}`. Schedule entries are
//!   surfaced verbatim from `data.schedules` via `Object.values()` —
//!   insertion order is preserved (we model the on-disk `schedules`
//!   field as `IndexMap<String, serde_json::Value>`).
//! * Malformed JSON → ALSO swallowed (parity with the TS
//!   `fileManager.readJSON<SchedulesData>(file, defaultData)` semantics,
//!   which falls back to the supplied default on parse failure).
//!
//! The `columns` array is a HARD-CODED CONSTANT
//! `["name","cron","timezone","type","status","lastRun","nextRun"]`,
//! emitted on every code path (parity with TS `list-schedules.ts:39-47`
//! and `:60-68` — both the missing-file branch and the happy-path
//! branch construct the same literal array).

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;

/// CLI arguments accepted by `list-schedules`. The TS Commander.js
/// registration (`src/commands/schedule/list-schedules.ts:95-104`)
/// declares a single `--json` flag. The dispatcher path uses the
/// `format` key (`"text"` default, `"json"` for the structured shape)
/// to mirror the list_hooks::run protocol.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListSchedulesArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// Deserialised projection of `spec/schedules.json`. Only the
/// `schedules` map is read — the `version` field (and any future
/// top-level fields) are ignored. `IndexMap` preserves declaration
/// order of schedule keys so `list-schedules` does NOT alphabetise
/// them (parity with the TS `Object.values(data.schedules)` iteration
/// order, which honours object-literal insertion order on every
/// supported V8 / Node version).
///
/// The value side is `serde_json::Value` so we re-emit each entry
/// verbatim without modelling the full ScheduleEntry union — parity
/// with the TS pass-through of `Object.values(data.schedules)`.
#[derive(Debug, Deserialize)]
struct SchedulesFile {
    #[serde(default)]
    schedules: IndexMap<String, serde_json::Value>,
}

/// Response shape returned to the dispatcher. Mirrors the TS
/// `ListSchedulesResult` interface at `src/types/schedule.ts:122-125`.
///
/// Field declaration order (`schedules`, then `columns`) is preserved
/// on the wire because we use `#[derive(Serialize)]` rather than
/// routing through `json!{}` (which would alphabetise via
/// `serde_json::Map`'s default BTreeMap backing). The JSON pretty
/// print therefore starts with `{\n  "schedules": [],\n  "columns": [...]}`
/// — asserted by `scenario_json_format_two_space_indent_for_empty_case`.
#[derive(Debug, Serialize)]
struct ListSchedulesResult {
    schedules: Vec<serde_json::Value>,
    columns: Vec<&'static str>,
}

/// The canonical, hard-coded columns array. Both the missing-file
/// branch and the happy-path branch emit this exact list. Order is
/// load-bearing — the documented help-example table header relies on
/// this sequence.
///
/// NOTE: COLUMNS labels JSON-payload column metadata, NOT the
/// underlying ScheduleEntry field names. In particular, the headers
/// `lastRun` and `nextRun` are DERIVED at text-render time from the
/// canonical ScheduleEntry fields `lastRunAt` (raw ISO string or `-`)
/// and `status` (`active` → `"See cron"`, otherwise `"Paused"`)
/// respectively — see `render_text` below. JSON consumers iterating
/// `schedules[*]` should therefore reference the canonical
/// ScheduleEntry fields (`status`, `lastRunAt`, ...), not the column
/// labels.
const COLUMNS: &[&str] = &[
    "name", "cron", "timezone", "type", "status", "lastRun", "nextRun",
];

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListSchedulesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-schedules",
            reason: format!("failed to parse args: {e}"),
        })?;

    let result = load_schedules(project_root);

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-schedules",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text.
        _ => Ok(render_text(&result)),
    }
}

/// Read `spec/schedules.json` and convert it into a
/// [`ListSchedulesResult`].
///
/// This is the "swallow everything" branch — both
/// `std::fs::read_to_string` failures (ENOENT or otherwise) AND
/// `serde_json::from_str` parse failures map to the canonical empty
/// payload `{schedules: [], columns: [...]}`. The columns array is
/// emitted on every path (parity with the TS literal at
/// `list-schedules.ts:39-47` and `:60-68`).
fn load_schedules(project_root: &Path) -> ListSchedulesResult {
    let path = project_root.join("spec").join("schedules.json");

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return empty_result(),
    };

    let parsed: SchedulesFile = match serde_json::from_str(&raw) {
        Ok(p) => p,
        Err(_) => return empty_result(),
    };

    let schedules = parsed.schedules.into_iter().map(|(_, v)| v).collect();

    ListSchedulesResult {
        schedules,
        columns: COLUMNS.to_vec(),
    }
}

/// Build the canonical empty response. Used by both branches of the
/// swallow path (missing file AND parse error). Always emits the
/// hard-coded columns array.
fn empty_result() -> ListSchedulesResult {
    ListSchedulesResult {
        schedules: Vec::new(),
        columns: COLUMNS.to_vec(),
    }
}

/// Render the text format documented in the `list-schedules` help-
/// example.
///
/// For the empty case (zero schedules) the function emits the exact
/// two-line sentinel block from TS `list-schedules.ts:111-114`:
///
/// ```text
/// No schedules configured.
/// Use `fspec add-schedule` to create a schedule.
/// ```
///
/// For the populated case the function emits a tab-separated header
/// row followed by a 100-character dashed rule, one tab-separated
/// data row per schedule, a blank line, and the `Total: N schedule(s)`
/// summary (parity with `list-schedules.ts:117-141`). chalk colouring
/// is intentionally omitted on the Rust path because (1) the
/// integration tests assert byte-for-byte lines and (2) the dispatcher
/// path is consumed by structured callers that already use
/// `format=json`.
fn render_text(result: &ListSchedulesResult) -> String {
    if result.schedules.is_empty() {
        return String::from(
            "No schedules configured.\nUse `fspec add-schedule` to create a schedule.\n",
        );
    }

    let mut out = String::new();
    out.push_str("Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run\n");
    out.push_str(&"-".repeat(100));
    out.push('\n');

    for schedule in &result.schedules {
        let name = schedule.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let cron = schedule.get("cron").and_then(|v| v.as_str()).unwrap_or("");
        let timezone = schedule
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let job_type = schedule
            .get("jobType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = schedule
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // lastRunAt: null → "-"; otherwise the raw ISO string.
        // The TS implementation passes the string through
        // `new Date(...).toLocaleString()` for human display; on the
        // Rust path we surface the raw timestamp string verbatim
        // because (a) the dispatcher tests do not depend on a
        // specific locale and (b) toLocaleString() output is not
        // deterministic across platforms.
        let last_run = match schedule.get("lastRunAt") {
            Some(v) if v.is_null() => "-".to_string(),
            Some(v) => v.as_str().unwrap_or("-").to_string(),
            None => "-".to_string(),
        };
        // nextRun: TS picks "See cron" for active, "Paused" otherwise.
        let next_run = if status == "active" {
            "See cron"
        } else {
            "Paused"
        };

        out.push_str(name);
        out.push('\t');
        out.push_str(cron);
        out.push('\t');
        out.push_str(timezone);
        out.push('\t');
        out.push_str(job_type);
        out.push('\t');
        out.push_str(status);
        out.push('\t');
        out.push_str(&last_run);
        out.push('\t');
        out.push_str(next_run);
        out.push('\n');
    }

    out.push('\n');
    out.push_str(&format!("Total: {} schedule(s)\n", result.schedules.len()));

    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ListSchedulesArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListSchedulesArgs = serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn columns_constant_matches_expected() {
        assert_eq!(
            COLUMNS,
            &["name", "cron", "timezone", "type", "status", "lastRun", "nextRun"]
        );
    }

    #[test]
    fn empty_result_carries_canonical_columns() {
        let r = empty_result();
        assert_eq!(r.schedules.len(), 0);
        assert_eq!(r.columns.len(), 7);
        assert_eq!(r.columns[0], "name");
        assert_eq!(r.columns[6], "nextRun");
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        let r = empty_result();
        let out = render_text(&r);
        assert!(out.lines().any(|l| l == "No schedules configured."));
        assert!(out
            .lines()
            .any(|l| l == "Use `fspec add-schedule` to create a schedule."));
    }

    #[test]
    fn render_text_populated_uses_help_example_layout() {
        let r = ListSchedulesResult {
            schedules: vec![serde_json::json!({
                "name": "nightly-build",
                "cron": "0 2 * * *",
                "timezone": "UTC",
                "jobType": "shell",
                "status": "active",
                "lastRunAt": null
            })],
            columns: COLUMNS.to_vec(),
        };
        let out = render_text(&r);
        assert!(out
            .lines()
            .any(|l| l == "Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run"));
        assert!(out
            .lines()
            .any(|l| l.starts_with("nightly-build\t0 2 * * *\tUTC\tshell\tactive\t")));
        assert!(out.lines().any(|l| l == "Total: 1 schedule(s)"));
    }
}
