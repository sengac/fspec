//! `export-work-units` — Rust port of `src/commands/export-work-units.ts` (RPC-229).
//!
//! Reads `spec/work-units.json` directly, takes `Object.values(data.workUnits)`
//! (insertion order preserved), and — for `format === "json"` — writes a
//! 2-space pretty-printed JSON array of the full work-unit objects to the
//! `output` path. Any other format (including `csv`) throws the canonical
//! `Unsupported format: <format>` error. Returns the JSON envelope
//! `{ "success": true }`.
//!
//! ## TS source of truth (`src/commands/export-work-units.ts:21-48`)
//!
//! ```ts
//! const content = await readFile(workUnitsFile, 'utf-8');
//! const data: WorkUnitsData = JSON.parse(content);
//! const workUnits = Object.values(data.workUnits);
//! if (options.format === 'json') {
//!   await writeFile(options.output, JSON.stringify(workUnits, null, 2));
//! } else {
//!   throw new Error(`Unsupported format: ${options.format}`);
//! }
//! return { success: true };
//! ```
//!
//! The `--status` / `--epic` CLI flags are accepted but **ignored** — the TS
//! function signature exposes only `format` + `output`, so any filter flags
//! threaded in via the Commander wrapper are silently dropped. This parity
//! quirk is asserted by the "status filter ignored" dispatcher scenario.
//!
//! Every error is wrapped with the TS-canonical prefix `Failed to export work
//! units:` so the dispatcher and CLI surfaces share that exact substring
//! (mirrors the `catch` at `src/commands/export-work-units.ts:42-47`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/export_work_units.rs` is JSON marshalling only — and
//! preserves the **broken** TS Commander shell (Framing A): the success log
//! references `result.count` and `result.outputFile` which are undefined
//! (the function only returns `{ success: true }`), so the shell prints
//! `Exported undefined work units to undefined`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::io_error::format_io_error;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

/// CLI / dispatcher arguments accepted by `export-work-units`. Mirrors the TS
/// `exportWorkUnits` options object (`src/commands/export-work-units.ts:21-25`)
/// plus the dispatcher-only `status` filter (accepted-but-ignored, see module
/// docs).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportWorkUnitsArgs {
    format: String,
    output: String,
    /// Accepted for CLI/dispatcher shape parity but never used — the TS
    /// function ignores it.
    #[serde(default)]
    #[allow(dead_code)]
    status: Option<String>,
    /// Accepted for CLI shape parity but never used.
    #[serde(default)]
    #[allow(dead_code)]
    epic: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportWorkUnitsResult {
    success: bool,
}

/// Wrap any inner error message with the TS-canonical prefix used by both
/// the dispatcher error path and the CLI stderr path
/// (`src/commands/export-work-units.ts:44`).
fn wrap_failure(inner: &str) -> String {
    format!("Failed to export work units: {inner}")
}

/// Dispatcher entry point. Two-front-doors invariant: the CLI bridge and the
/// LLM dispatcher both call this function with a JSON-encoded args payload and
/// a project_root path.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ExportWorkUnitsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-work-units",
            reason: wrap_failure(&format!("failed to parse args: {e}")),
        })?;

    let work_units_path = project_root.join("spec").join("work-units.json");

    // TS reads the file directly (no ensure helper); surface any IO error
    // through the canonical wrapper.
    let raw =
        std::fs::read_to_string(&work_units_path).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-work-units",
            reason: wrap_failure(&format_io_error(&e, &work_units_path.display().to_string())),
        })?;

    let data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-work-units",
            reason: wrap_failure(&format!("Unexpected token in JSON: {e}")),
        })?;

    // `Object.values(data.workUnits)` — insertion order preserved by IndexMap.
    let work_units: Vec<&WorkUnit> = data.work_units.values().collect();

    if args.format == "json" {
        // `JSON.stringify(workUnits, null, 2)` — 2-space pretty. The WorkUnit
        // manual Serialize impl preserves on-disk key order, so this is
        // byte-parity with the TS export for the standard fixtures.
        let body =
            serde_json::to_string_pretty(&work_units).map_err(|e| FspecCoreError::InvalidArgs {
                command: "export-work-units",
                reason: wrap_failure(&format!("failed to serialize work units: {e}")),
            })?;
        std::fs::write(&args.output, body).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-work-units",
            reason: wrap_failure(&format_io_error(&e, &args.output)),
        })?;
    } else {
        return Err(FspecCoreError::InvalidArgs {
            command: "export-work-units",
            reason: wrap_failure(&format!("Unsupported format: {}", args.format)),
        });
    }

    let result = ExportWorkUnitsResult { success: true };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "export-work-units",
        reason: wrap_failure(&format!("failed to serialize result: {e}")),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: ExportWorkUnitsArgs =
            serde_json::from_str(r#"{"format":"json","output":"out.json"}"#).unwrap();
        assert_eq!(a.format, "json");
        assert_eq!(a.output, "out.json");
    }

    #[test]
    fn args_tolerate_ignored_status_filter() {
        let a: ExportWorkUnitsArgs =
            serde_json::from_str(r#"{"format":"json","output":"out.json","status":"done"}"#)
                .unwrap();
        assert_eq!(a.status.as_deref(), Some("done"));
    }
}
