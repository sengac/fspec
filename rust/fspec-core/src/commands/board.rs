//! `board` — Rust port of `src/commands/display-board.ts` (RPC-199).
//!
//! Computes a Kanban-board view of every work unit grouped by lifecycle
//! state, plus a story-point summary (`<N> points in progress, <M> points
//! completed`).
//!
//! ## Framing-A decision (RPC-199, APPROVED)
//!
//! The TypeScript CLI renders the default (`--format=text`) board via an Ink
//! React TUI (`src/components/BoardDisplay`). The Rust standalone binary is
//! headless — it serves a plain-text default rendering instead of a TUI, with
//! `--format=json` producing the machine-readable `{columns, board, summary}`
//! shape. This mirrors the `list-*` precedent (text default + json opt-in).
//! The structured payload returned here is identical for both formats; the
//! CLI bridge chooses the rendering.
//!
//! ## Result envelope
//!
//! Returns `Ok(json)` with the full TS `BoardResult` shape:
//! `{columns, board, summary}`. `columns[status]` is an array of
//! `{id, title, estimate}`; `board[status]` is an array of work-unit IDs;
//! `summary` is the points line. The dispatcher derives `success=true` from
//! the `Ok`. A missing `spec/foundation.json` yields `Err(FoundationMissing)`
//! (parity with the TS `checkFoundationExists` guard) → exit 1.
//!
//! ## Two-front-doors invariant (RPC-003 §7/§11)
//!
//! Both the LLM dispatcher AND the standalone binary's `fspec board` clap
//! subcommand call this single `run`. No point-summing or column-building
//! lives in the CLI bridge.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::{check_foundation_exists, ensure_work_units_file};
use crate::types::work_unit::WorkUnitsData;

/// CLI / dispatcher arguments accepted by `board`.
///
/// Parity with the TS Commander.js registration at
/// `src/commands/display-board.ts:90-96`: `--format <format>` (text|json,
/// default text) and `--limit <limit>` (default 25). The structured payload
/// computed here is format- and limit-agnostic; both options affect rendering
/// only and are owned by the CLI bridge, so they are not modelled in the
/// dispatcher args.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct BoardArgs {
    /// Accepted for forward-compat / parity; ignored by the structured path.
    #[allow(dead_code)]
    format: Option<String>,
}

/// Dispatcher entry point. `project_root` is supplied by both front doors.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    // Parse (tolerant) — the args don't affect the payload but we still reject
    // malformed JSON for a clear error.
    let _args: BoardArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "board",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Foundation guard (parity with displayBoard's checkFoundationExists at
    // src/commands/display-board.ts:38-41). Emits the verbatim foundation-
    // missing message via FoundationMissing → CLI exit 1.
    check_foundation_exists(project_root, "fspec board")?;

    // Auto-create work-units.json if missing (parity with ensureWorkUnitsFile).
    let data: WorkUnitsData = ensure_work_units_file(project_root)?;

    let (columns, board, in_progress, completed) = build_board(&data);

    let summary = format!("{in_progress} points in progress, {completed} points completed");

    let payload = json!({
        "columns": columns,
        "board": board,
        "summary": summary,
    });
    serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "board",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Iterate every state array, building the `columns`/`board` maps and summing
/// story points. Mirrors `src/commands/display-board.ts:48-79`:
///   * a work unit's `estimate` (when truthy) adds to `completedPoints` when
///     its state is `done`, else to `inProgressPoints`.
///   * the seven states are visited in their on-disk insertion order
///     (backlog → specifying → testing → implementing → validating → done →
///     blocked) so the emitted `columns`/`board` key order matches TS.
fn build_board(data: &WorkUnitsData) -> (Value, Value, u64, u64) {
    let mut columns: Map<String, Value> = Map::new();
    let mut board: Map<String, Value> = Map::new();
    let mut in_progress_points: u64 = 0;
    let mut completed_points: u64 = 0;

    let states = &data.states;
    let ordered: [(&str, &Vec<String>); 7] = [
        ("backlog", &states.backlog),
        ("specifying", &states.specifying),
        ("testing", &states.testing),
        ("implementing", &states.implementing),
        ("validating", &states.validating),
        ("done", &states.done),
        ("blocked", &states.blocked),
    ];

    for (status, ids) in ordered {
        let mut column_entries: Vec<Value> = Vec::with_capacity(ids.len());
        for id in ids {
            let (title, estimate) = lookup_work_unit(data, id);
            let mut entry = Map::new();
            entry.insert("id".to_string(), Value::String(id.clone()));
            if let Some(t) = title {
                entry.insert("title".to_string(), Value::String(t));
            }
            if let Some(e) = estimate {
                entry.insert("estimate".to_string(), json!(e));
            }
            column_entries.push(Value::Object(entry));

            // Point accumulation: only truthy estimates contribute (parity
            // with `if (wu.estimate)` — 0 / missing is skipped).
            if let Some(e) = estimate {
                if e > 0 {
                    if status == "done" {
                        completed_points += e;
                    } else {
                        in_progress_points += e;
                    }
                }
            }
        }
        columns.insert(status.to_string(), Value::Array(column_entries));
        board.insert(
            status.to_string(),
            Value::Array(ids.iter().cloned().map(Value::String).collect()),
        );
    }

    (
        Value::Object(columns),
        Value::Object(board),
        in_progress_points,
        completed_points,
    )
}

/// Resolve a work unit's `title` and `estimate` from the data map. `estimate`
/// is read from the unit's `extra` bag (the Rust `WorkUnit` keeps it there as
/// an untyped passthrough) as a non-negative integer point value.
fn lookup_work_unit(data: &WorkUnitsData, id: &str) -> (Option<String>, Option<u64>) {
    match data.work_units.get(id) {
        Some(wu) => {
            let title = if wu.title.is_empty() {
                None
            } else {
                Some(wu.title.clone())
            };
            let estimate = wu.extra.get("estimate").and_then(Value::as_u64);
            (title, estimate)
        }
        // A state-array ID with no matching workUnits entry: the TS code would
        // throw (`wu.id` on undefined). We degrade to id-only to avoid a panic;
        // the board still lists the orphan id under `board[status]`.
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn data_from(json_str: &str) -> WorkUnitsData {
        serde_json::from_str(json_str).unwrap()
    }

    #[test]
    fn summary_sums_done_and_in_progress() {
        let data = data_from(
            r#"{
              "version": "0.7.1",
              "workUnits": {
                "A-1": { "id": "A-1", "title": "Login", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" },
                "A-2": { "id": "A-2", "title": "Logout", "status": "implementing", "estimate": 3, "createdAt": "x", "updatedAt": "x" }
              },
              "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": ["A-2"], "validating": [],
                "done": ["A-1"], "blocked": []
              }
            }"#,
        );
        let (_c, _b, in_progress, completed) = build_board(&data);
        assert_eq!(in_progress, 3);
        assert_eq!(completed, 5);
    }

    #[test]
    fn columns_carry_id_title_estimate() {
        let data = data_from(
            r#"{
              "version": "0.7.1",
              "workUnits": {
                "A-1": { "id": "A-1", "title": "Login", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" }
              },
              "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [],
                "done": ["A-1"], "blocked": []
              }
            }"#,
        );
        let (columns, board, _i, _c) = build_board(&data);
        let done = &columns["done"][0];
        assert_eq!(done["id"], "A-1");
        assert_eq!(done["title"], "Login");
        assert_eq!(done["estimate"], 5);
        assert_eq!(board["done"][0], "A-1");
    }

    #[test]
    fn missing_estimate_does_not_contribute() {
        let data = data_from(
            r#"{
              "version": "0.7.1",
              "workUnits": {
                "A-1": { "id": "A-1", "title": "No estimate", "status": "implementing", "createdAt": "x", "updatedAt": "x" }
              },
              "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": ["A-1"], "validating": [],
                "done": [], "blocked": []
              }
            }"#,
        );
        let (_c, _b, in_progress, completed) = build_board(&data);
        assert_eq!(in_progress, 0);
        assert_eq!(completed, 0);
    }

    #[test]
    fn empty_states_yield_zero_summary() {
        let data = data_from(
            r#"{ "version": "0.7.1", "workUnits": {}, "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            } }"#,
        );
        let (_c, _b, in_progress, completed) = build_board(&data);
        assert_eq!(in_progress, 0);
        assert_eq!(completed, 0);
    }
}
