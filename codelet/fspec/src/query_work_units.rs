//! `query-work-units` shell-facing CLI bridge (RPC-263).
//!
//! Feature: spec/features/query-work-units-cli-subcommand.feature
//!
//! Two-front-doors pattern (architecture note on RPC-263):
//!   - Shell argv         → clap → this module → fspec_core::commands::query_work_units::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_work_units::run
//!
//! Both call sites pass a JSON-encoded `QueryWorkUnitsArgs` shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from CWD
//! (parity with the TypeScript `process.cwd()` default) and serialises the
//! six CLI-exposed clap fields into the dispatcher JSON shape. No filter
//! or rendering logic is duplicated here.
//!
//! TS quirk preserved: when `--format=json` the CLI prints the rendered
//! JSON envelope to stdout; for any other format the CLI prints NOTHING
//! (TS `Commander.js` action only calls `output.log` when
//! `options.format === 'json'`). See feature scenario "CLI query-work-units
//! --format=text prints NOTHING to stdout per TS quirk".
//!
//! Exit-code contract:
//!   - 0 on success (regardless of format).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` and contains the canonical
//!     substring `Failed to query work units:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_work_units;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set
/// (`src/commands/query-work-units.ts:239-277`). Only the six CLI-exposed
/// flags are modelled here — function-level options (sort/order/output/
/// hasQuestions/questionsFor/showCycleTime/workUnitId/json) are dispatcher-
/// only per the cli-subcommand feature file.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub status: Option<String>,
    pub prefix: Option<String>,
    pub epic: Option<String>,
    pub r#type: Option<String>,
    pub tag: Option<String>,
    /// `"text"` (default), `"json"`, or `"csv"`. Passed through verbatim.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `query-work-units` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    let mut obj = serde_json::Map::new();
    if let Some(v) = args.status.as_ref() {
        obj.insert("status".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.prefix.as_ref() {
        obj.insert("prefix".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.epic.as_ref() {
        obj.insert("epic".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.r#type.as_ref() {
        obj.insert("type".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.tag.as_ref() {
        obj.insert("tag".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.format.as_ref() {
        obj.insert("format".to_string(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    let is_json = matches!(args.format.as_deref(), Some("json"));

    match query_work_units::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // TS quirk: print JSON to stdout ONLY when format=='json'.
            // For text/csv/table the Commander action does NOT log
            // (the CSV side-effect is the file write, performed by
            // fspec_core itself).
            if is_json {
                // The rendered string is a pretty-printed JSON envelope
                // produced by fspec_core (TS parity: 2-space indent via
                // JSON.stringify(result, null, 2)).
                println!("{rendered}");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗ Query failed:', ...)` path.
            // Strip the FspecCoreError::InvalidArgs wrapper prefix so the
            // printed message matches the bare TS Error.message (which
            // already carries the canonical `Failed to query work units:`
            // prefix wrapped by fspec_core).
            let msg = match &err {
                FspecCoreError::InvalidArgs { reason, .. } => reason.clone(),
                _ => err.to_string(),
            };
            eprintln!("✗ Query failed: {msg}");
            Ok(1)
        }
    }
}
