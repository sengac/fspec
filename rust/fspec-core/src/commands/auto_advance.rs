//! `auto-advance` — Rust port of `src/commands/auto-advance.ts` (RPC-198).
//!
//! Advances a SINGLE work unit through a fixed state-transition table when the
//! supplied `from` state + `event` match a defined transition, then persists
//! the mutated `spec/work-units.json` via one atomic write. Returns the JSON
//! envelope `{ "success": true, "newState": "<to>" }`.
//!
//! ## TS source of truth (`src/commands/auto-advance.ts:38-114`)
//!
//! The `autoAdvance({ workUnitId, from, event })` function (NOT the
//! `registerAutoAdvanceCommand` Commander variant) is the behavioural
//! contract exercised by the dispatcher tests. Its fixed transition table is:
//!
//! ```ts
//! const STATE_TRANSITIONS = [
//!   { from: 'testing',    event: 'tests-pass',      to: 'implementing' },
//!   { from: 'validating', event: 'validation-pass', to: 'done', recordCompletion: true },
//! ];
//! ```
//!
//! Ordering of checks mirrors TS exactly:
//!   1. work-unit existence  → `Work unit {id} not found`
//!   2. transition lookup    → `No transition defined for {from} + {event}`
//!   3. state match          → `Work unit is in {status} state, expected {from}`
//!
//! Every error is wrapped with the TS-canonical prefix `Failed to
//! auto-advance:` (the `catch` at `src/commands/auto-advance.ts:108-113`).
//!
//! ## Two-front-doors
//!
//! Both the LLM dispatcher AND the standalone binary's clap subcommand call
//! this single function. The CLI bridge at `rust/fspec/src/auto_advance.rs`
//! is JSON marshalling only — and preserves the **broken** TS Commander shell
//! (Framing A): the Commander action wires only `--dry-run` and NEVER passes
//! `workUnitId`/`from`/`event`, so the function reads an undefined id and
//! ALWAYS fails with `Work unit undefined not found`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::io_error::format_io_error;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnitStates, WorkUnitStatus, WorkUnitsData};

/// CLI / dispatcher arguments accepted by `auto-advance`. Mirrors the TS
/// `autoAdvance` options object (`src/commands/auto-advance.ts:38-43`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoAdvanceArgs {
    /// The work unit to advance. `Option<String>` so the broken-shell Framing
    /// A path (the TS Commander action never wires `workUnitId`) deserialises
    /// to `None`, surfacing the canonical `Work unit undefined not found`
    /// error rather than a serde missing-field failure.
    #[serde(default)]
    work_unit_id: Option<String>,
    /// Current state before the transition.
    #[serde(default)]
    from: Option<String>,
    /// Event that triggers the transition.
    #[serde(default)]
    event: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoAdvanceResult {
    success: bool,
    new_state: String,
}

/// A single entry of the fixed TS `STATE_TRANSITIONS` table.
struct Transition {
    from: &'static str,
    event: &'static str,
    to: WorkUnitStatus,
    record_completion: bool,
}

const STATE_TRANSITIONS: &[Transition] = &[
    Transition {
        from: "testing",
        event: "tests-pass",
        to: WorkUnitStatus::Implementing,
        record_completion: false,
    },
    Transition {
        from: "validating",
        event: "validation-pass",
        to: WorkUnitStatus::Done,
        record_completion: true,
    },
];

/// Wrap any inner error message with the TS-canonical prefix used by both the
/// dispatcher error path and the CLI stderr path
/// (`src/commands/auto-advance.ts:110`).
fn wrap_failure(inner: &str) -> String {
    format!("Failed to auto-advance: {inner}")
}

/// Mutable accessor for one of the 7 typed state arrays by its lowercase name.
/// Returns `None` for an unknown state (mirrors the TS `if (data.states[...])`
/// guard which simply skips when the array is absent).
fn state_vec_mut<'a>(states: &'a mut WorkUnitStates, name: &str) -> Option<&'a mut Vec<String>> {
    match name {
        "backlog" => Some(&mut states.backlog),
        "specifying" => Some(&mut states.specifying),
        "testing" => Some(&mut states.testing),
        "implementing" => Some(&mut states.implementing),
        "validating" => Some(&mut states.validating),
        "done" => Some(&mut states.done),
        "blocked" => Some(&mut states.blocked),
        _ => None,
    }
}

/// Dispatcher entry point. Two-front-doors invariant: the CLI bridge and the
/// LLM dispatcher both call this function with a JSON-encoded args payload and
/// a project_root path.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AutoAdvanceArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&format!("failed to parse args: {e}")),
        })?;

    // Framing A: `options.workUnitId` is `undefined` because the Commander
    // action never wires it, so `data.workUnits[undefined]` is missing and the
    // function throws `Work unit undefined not found`. Mirror the literal
    // string `undefined` for parity.
    let work_unit_id = args.work_unit_id.as_deref().unwrap_or("undefined");
    let from = args.from.as_deref().unwrap_or("undefined");
    let event = args.event.as_deref().unwrap_or("undefined");

    let work_units_path = project_root.join("spec").join("work-units.json");

    let raw =
        std::fs::read_to_string(&work_units_path).map_err(|e| FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&format_io_error(&e, &work_units_path.display().to_string())),
        })?;

    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&crate::io::json_error::parse_json_reason(&raw, &e)),
        })?;

    // (1) Existence check FIRST (TS auto-advance.ts:53-55).
    if !data.work_units.contains_key(work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&format!("Work unit {work_unit_id} not found")),
        });
    }

    // (2) Find matching transition (TS auto-advance.ts:60-68).
    let transition = STATE_TRANSITIONS
        .iter()
        .find(|t| t.from == from && t.event == event);
    let transition = match transition {
        Some(t) => t,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "auto-advance",
                reason: wrap_failure(&format!("No transition defined for {from} + {event}")),
            });
        }
    };

    // (3) Verify current state matches `from` (TS auto-advance.ts:71-75).
    let current_status = data.work_units[work_unit_id].status.as_str().to_string();
    if current_status != from {
        return Err(FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&format!(
                "Work unit is in {current_status} state, expected {from}"
            )),
        });
    }

    let to_str = transition.to.as_str().to_string();
    let now = iso8601_now();

    // Remove from old state array (TS auto-advance.ts:78-82).
    if let Some(vec) = state_vec_mut(&mut data.states, from) {
        vec.retain(|id| id != work_unit_id);
    }
    // Add to new state array (TS auto-advance.ts:85-88).
    if let Some(vec) = state_vec_mut(&mut data.states, &to_str) {
        vec.push(work_unit_id.to_string());
    }

    // Update the work unit (TS auto-advance.ts:91-97).
    let wu = data
        .work_units
        .get_mut(work_unit_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "auto-advance",
            reason: wrap_failure(&format!("Work unit {work_unit_id} not found")),
        })?;
    wu.status = transition.to;
    wu.updated_at = now.clone();
    if transition.record_completion {
        wu.extra
            .insert("completedAt".to_string(), serde_json::Value::String(now));
    }

    // Single atomic write (TS uses fileManager.transaction()).
    write_json_atomic(&work_units_path, &data)?;

    let result = AutoAdvanceResult {
        success: true,
        new_state: to_str,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "auto-advance",
        reason: wrap_failure(&format!("failed to serialize result: {e}")),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AutoAdvanceArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","from":"testing","event":"tests-pass"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.from.as_deref(), Some("testing"));
        assert_eq!(a.event.as_deref(), Some("tests-pass"));
    }

    #[test]
    fn args_default_to_none_for_framing_a() {
        let a: AutoAdvanceArgs = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(a.work_unit_id, None);
        assert_eq!(a.from, None);
        assert_eq!(a.event, None);
    }
}
