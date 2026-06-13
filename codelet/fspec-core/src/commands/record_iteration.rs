//! `record-iteration` — Rust port of `src/commands/record-iteration.ts` (RPC-264).
//!
//! Increments a work unit's `iterations` counter (treating an absent counter
//! as `0`), refreshes its `updatedAt` timestamp, and persists the mutated
//! `spec/work-units.json` via a single atomic write. Returns the JSON envelope
//! `{ "success": true, "iterations": <new-count> }`.
//!
//! ## TS source of truth (`src/commands/record-iteration.ts:21-56`)
//!
//! ```ts
//! const content = await readFile(workUnitsFile, 'utf-8');
//! const data: WorkUnitsData = JSON.parse(content);
//! if (!data.workUnits[options.workUnitId]) {
//!   throw new Error(`Work unit ${options.workUnitId} not found`);
//! }
//! const workUnit = data.workUnits[options.workUnitId];
//! workUnit.iterations = (workUnit.iterations || 0) + 1;
//! workUnit.updatedAt = new Date().toISOString();
//! await writeFile(workUnitsFile, JSON.stringify(data, null, 2));
//! return { success: true, iterations: workUnit.iterations };
//! ```
//!
//! Every error is wrapped with the TS-canonical prefix `Failed to record
//! iteration:` so the dispatcher and CLI surfaces share that exact substring
//! (mirrors the `catch` at `src/commands/record-iteration.ts:50-55`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/record_iteration.rs` is JSON marshalling only — and
//! preserves the **broken** TS Commander shell (Framing A): the Commander
//! action wires `name`/`start`/`end` and NEVER passes `workUnitId`, so the
//! function reads an undefined id and ALWAYS fails with `Work unit undefined
//! not found`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

/// CLI / dispatcher arguments accepted by `record-iteration`. Mirrors the TS
/// `recordIteration` options object (`src/commands/record-iteration.ts:21-24`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordIterationArgs {
    /// The work unit whose `iterations` counter should be incremented. The
    /// field is `Option<String>` so the broken-shell Framing A path (the TS
    /// Commander action never wires `workUnitId`) deserialises to `None`,
    /// surfacing the canonical `Work unit undefined not found` error rather
    /// than a serde missing-field failure.
    #[serde(default)]
    work_unit_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RecordIterationResult {
    success: bool,
    iterations: u64,
}

/// Wrap any inner error message with the TS-canonical prefix used by both
/// the dispatcher error path and the CLI stderr path
/// (`src/commands/record-iteration.ts:52`).
fn wrap_failure(inner: &str) -> String {
    format!("Failed to record iteration: {inner}")
}

/// Dispatcher entry point. Two-front-doors invariant: the CLI bridge and the
/// LLM dispatcher both call this function with a JSON-encoded args payload and
/// a project_root path.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RecordIterationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "record-iteration",
            reason: wrap_failure(&format!("failed to parse args: {e}")),
        })?;

    // TS Framing A: `options.workUnitId` is `undefined` because the Commander
    // action never wires it, so `data.workUnits[undefined]` is missing and
    // the function throws `Work unit undefined not found`. Mirror the literal
    // string `undefined` for parity.
    let work_unit_id = args.work_unit_id.as_deref().unwrap_or("undefined");

    let work_units_path = project_root.join("spec").join("work-units.json");

    // TS reads the file directly (no ensure helper); surface any IO error
    // through the canonical wrapper.
    let raw =
        std::fs::read_to_string(&work_units_path).map_err(|e| FspecCoreError::InvalidArgs {
            command: "record-iteration",
            reason: wrap_failure(&format_io_error(&e, &work_units_path.display().to_string())),
        })?;

    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
            command: "record-iteration",
            reason: wrap_failure(&format!("Unexpected token in JSON: {e}")),
        })?;

    // Validate the work unit exists (mirrors src/commands/record-iteration.ts:34-36).
    let wu = match data.work_units.get_mut(work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "record-iteration",
                reason: wrap_failure(&format!("Work unit {work_unit_id} not found")),
            });
        }
    };

    let now = iso8601_now();

    // Increment: `iterations = (iterations || 0) + 1` (TS treats a non-numeric
    // or absent value as 0 via the `|| 0` short-circuit).

    let current = wu
        .extra
        .get("iterations")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next = current + 1;
    wu.extra.insert("iterations".to_string(), Value::from(next));
    wu.updated_at = now;

    // Single atomic write (TS `writeFile(workUnitsFile, JSON.stringify(data, null, 2))`).
    write_json_atomic(&work_units_path, &data)?;

    let result = RecordIterationResult {
        success: true,
        iterations: next,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "record-iteration",
        reason: wrap_failure(&format!("failed to serialize result: {e}")),
    })
}

/// Format a `std::io::Error` into the canonical TS Node `Error.message` shape
/// for filesystem read failures: `ENOENT: no such file or directory, open
/// '<path>'`. Mirrors the `format_io_error` helper in `query_work_units.rs`.
fn format_io_error(e: &std::io::Error, path: &str) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        format!("ENOENT: no such file or directory, open '{path}'")
    } else {
        format!("{e}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RecordIterationArgs = serde_json::from_str(r#"{"workUnitId":"AUTH-001"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
    }

    #[test]
    fn args_default_to_none_when_work_unit_id_absent() {
        // Framing A: the broken TS shell never wires workUnitId.
        let a: RecordIterationArgs = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(a.work_unit_id, None);
    }
}
