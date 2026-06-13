//! `show-deleted` — Rust port of `src/commands/show-deleted.ts` (RPC-301).
//!
//! Displays all soft-deleted items (rules, examples, questions, and
//! architecture notes) on a work unit. Reads `spec/work-units.json`
//! via the shared [`crate::io::ensure::ensure_work_units_file`] helper
//! (load-or-init parity with TS), then walks the four soft-delete arrays
//! on the requested work unit and flattens them into a single
//! `deletedItems` array in the canonical concatenation order:
//! `rules → examples → questions → architectureNotes`.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

/// CLI arguments accepted by `show-deleted`.
///
/// The TS Commander.js registration at `src/commands/show-deleted.ts:72-77`
/// declares a single REQUIRED positional `<workUnitId>` and NO `.option(...)`
/// calls. We additionally expose a `format` field at the dispatcher surface
/// so `{"workUnitId":"AUTH-001","format":"json"}` produces a structured
/// pretty-printed payload (matching the tool-call protocol used elsewhere in
/// the dispatcher), while the default text rendering matches the TS
/// chalk-coloured CLI output (ANSI stripped — non-TTY parity contract).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowDeletedArgs {
    /// Required work-unit identifier (e.g. `AUTH-001`).
    work_unit_id: Option<String>,
    /// `"text"` (default) or `"json"`.
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowDeletedArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-deleted",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args.work_unit_id.ok_or(FspecCoreError::InvalidArgs {
        command: "show-deleted",
        reason: "missing required argument: workUnitId".to_string(),
    })?;

    // Load-or-init parity with TS (`src/commands/show-deleted.ts:32`).
    // ensure_work_units_file AUTO-CREATES the file when missing — this is
    // the behaviour asserted by the
    // `auto_creates_work_units_json_and_fails_when_unit_missing` scenario.
    let data = ensure_work_units_file(project_root)?;

    // Validate work unit exists — error message MUST match TS
    // `Work unit '<id>' does not exist` (src/commands/show-deleted.ts:36).
    let work_unit =
        data.work_units
            .get(&work_unit_id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "show-deleted",
                reason: format!("Work unit '{work_unit_id}' does not exist"),
            })?;

    // Walk the four soft-delete arrays in the canonical order. Each array
    // is stored on the WorkUnit's `extra` Map (we deliberately do NOT model
    // these fields strictly on `WorkUnit` — they round-trip through
    // `extra` so newly-added fields don't need a Rust schema bump).
    let deleted_items = collect_deleted_items(&work_unit.extra);
    let total_deleted = deleted_items.len();

    match args.format.as_deref() {
        Some("json") => {
            let result = json!({
                "success": true,
                "workUnitId": work_unit_id,
                "deletedItems": deleted_items,
                "totalDeleted": total_deleted,
            });
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "show-deleted",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text (matches TS Commander.js shell output).
        _ => Ok(render_text(&work_unit_id, &deleted_items)),
    }
}

/// In-memory shape of a single soft-deleted item — mirrors the TS
/// `DeletedItem` interface (`src/commands/show-deleted.ts:13-17`).
///
/// `#[serde(skip_serializing_if = "Option::is_none")]` on `deleted_at`
/// matches the TS behaviour where `JSON.stringify` omits `undefined`
/// fields: when the source item carries no `deletedAt` timestamp, the
/// emitted JSON drops the field entirely instead of writing `null`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeletedItem {
    id: u64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
}

/// Collect deleted items from a WorkUnit's flatten'ed `extra` map.
///
/// Mirrors `src/commands/show-deleted.ts:42-63`: walks rules, examples,
/// questions, architectureNotes (in that exact order), filters by
/// `deleted === true`, and projects only `{ id, text, deletedAt }`.
fn collect_deleted_items(extra: &serde_json::Map<String, Value>) -> Vec<DeletedItem> {
    const ARRAYS_IN_ORDER: &[&str] = &["rules", "examples", "questions", "architectureNotes"];
    let mut out: Vec<DeletedItem> = Vec::new();
    for key in ARRAYS_IN_ORDER {
        let Some(arr) = extra.get(*key).and_then(Value::as_array) else {
            // Absent or non-array field → skip (parity with `|| []` default).
            continue;
        };
        for entry in arr {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            // `deleted: true` is the truthiness gate (TS `.filter(r => r.deleted)`).
            // Missing field or `false` both fail the filter.
            if obj.get("deleted").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let id = match obj.get("id").and_then(Value::as_u64) {
                Some(v) => v,
                None => continue,
            };
            let text = match obj.get("text").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => continue,
            };
            let deleted_at = obj
                .get("deletedAt")
                .and_then(Value::as_str)
                .map(str::to_string);
            out.push(DeletedItem {
                id,
                text,
                deleted_at,
            });
        }
    }
    out
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/show-deleted.ts:81-100`).
///
/// The TS implementation prints:
///   `\nDeleted items in <id> (<n> total):`
///   `  [<id>] <text> (deleted: <iso>)`     ← (deleted: …) suffix only when present
///   `  [<id>] <text>`
///   (empty trailing line)
///
/// The empty-case sentinel is `No deleted items found`.
fn render_text(work_unit_id: &str, items: &[DeletedItem]) -> String {
    if items.is_empty() {
        return "No deleted items found".to_string();
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!(
        "Deleted items in {} ({} total):\n",
        work_unit_id,
        items.len()
    ));
    for item in items {
        match &item.deleted_at {
            Some(ts) => out.push_str(&format!(
                "  [{}] {} (deleted: {})\n",
                item.id, item.text, ts
            )),
            None => out.push_str(&format!("  [{}] {}\n", item.id, item.text)),
        }
    }
    out.push('\n');
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
    use serde_json::json;

    fn extra_with(
        rules: Value,
        examples: Value,
        questions: Value,
        arch: Value,
    ) -> serde_json::Map<String, Value> {
        let mut m = serde_json::Map::new();
        if !rules.is_null() {
            m.insert("rules".into(), rules);
        }
        if !examples.is_null() {
            m.insert("examples".into(), examples);
        }
        if !questions.is_null() {
            m.insert("questions".into(), questions);
        }
        if !arch.is_null() {
            m.insert("architectureNotes".into(), arch);
        }
        m
    }

    #[test]
    fn args_parse_with_defaults() {
        let a: ShowDeletedArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn collect_walks_arrays_in_canonical_order() {
        let extra = extra_with(
            json!([{"id":0,"text":"R","deleted":true}]),
            json!([{"id":0,"text":"E","deleted":true}]),
            json!([{"id":0,"text":"Q","deleted":true}]),
            json!([{"id":0,"text":"N","deleted":true}]),
        );
        let items = collect_deleted_items(&extra);
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, vec!["R", "E", "Q", "N"]);
    }

    #[test]
    fn collect_filters_deleted_false_and_missing() {
        let extra = extra_with(
            json!([
                {"id":0,"text":"keep","deleted":true},
                {"id":1,"text":"skip-false","deleted":false},
                {"id":2,"text":"skip-missing"}
            ]),
            Value::Null,
            Value::Null,
            Value::Null,
        );
        let items = collect_deleted_items(&extra);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "keep");
    }

    #[test]
    fn collect_handles_absent_arrays() {
        let m = serde_json::Map::new();
        let items = collect_deleted_items(&m);
        assert!(items.is_empty());
    }

    #[test]
    fn render_text_empty_returns_sentinel() {
        assert_eq!(render_text("AUTH-001", &[]), "No deleted items found");
    }

    #[test]
    fn render_text_omits_deleted_suffix_when_timestamp_absent() {
        let items = vec![DeletedItem {
            id: 7,
            text: "No ts".into(),
            deleted_at: None,
        }];
        let out = render_text("AUTH-001", &items);
        assert!(out.contains("  [7] No ts\n"));
        assert!(!out.contains("deleted:"));
    }
}
