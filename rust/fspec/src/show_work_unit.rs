//! `show-work-unit` shell-facing CLI bridge (RPC-308).
//!
//! Feature: spec/features/show-work-unit-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ShowWorkUnit` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::show_work_unit::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here):
//!   - Shell argv         → clap → this module → fspec_core::commands::show_work_unit::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::show_work_unit::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/show-work-unit.ts:67`). The
//! clap subcommand exposes one REQUIRED positional `<WORK_UNIT_ID>` and one
//! `-f, --format <format>` flag — matching the TS Commander.js registration
//! at `src/commands/show-work-unit.ts:468-475`.
//!
//! No projection, filtering, reminder generation, or feature-file scan
//! logic is duplicated here — the bridge's only computation is JSON arg
//! marshalling. The CLI-delegation test
//! `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher`
//! scans this file for forbidden substrings (camelCase output field names,
//! warning copy, helper symbol names) that would indicate inlined logic.
//!
//! Exit-code contract:
//!   - 0 on success — both `text` and `json` rendering branches.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written
//!     to stderr prefixed with `Error:` (parity with the TS chalk-red error
//!     path). The dispatcher wraps domain errors in `InvalidArgs { reason }`
//!     — strip that wrapper so the printed message matches the bare TS
//!     `Error.message` (e.g. `Work unit 'UNKNOWN-999' does not exist`).
//!   - 2 (clap's own usage error) when the required positional is omitted —
//!     clap validates before this module is reached, so we never see that
//!     case here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_work_unit;
use codelet_fspec_core::FspecCoreError;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface for
/// `show-work-unit` (`src/commands/show-work-unit.ts:468-475`). The TS
/// registration declares one required positional `<workUnitId>` and a
/// single `-f, --format <format>` flag.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the work-unit identifier to display.
    pub work_unit_id: String,
    /// Optional rendering mode. `None` → use fspec_core's text default;
    /// `Some("json")` → emit pretty-printed JSON.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `show-work-unit` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-
    // driven invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The dispatcher arg keys are camelCase — verified by the
    // `ShowWorkUnitArgs` deserializer in
    // `rust/fspec-core/src/commands/show_work_unit.rs`. Only thread
    // the format key through when the CLI flag was supplied so the
    // dispatcher's default-arm (omitted → text) drives unflagged
    // invocations.
    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".into(), json!(args.work_unit_id));
    if let Some(fmt) = args.format.as_deref() {
        obj.insert("format".into(), json!(fmt));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match show_work_unit::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Text format embeds its own leading + trailing newline
            // structure (see the text renderer in fspec-core); print
            // as-is. The pretty-printed JSON branch does not end with a
            // newline, so we append one for shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)` path:
            // the dispatcher wraps domain errors in
            // `FspecCoreError::InvalidArgs { reason }` — strip that wrapper
            // so the printed message matches the bare TS `Error.message`
            // (e.g. `Work unit '<id>' does not exist`).
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("Error: {reason}");
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
