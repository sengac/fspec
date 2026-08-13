//! `add-scenario` shell-facing CLI bridge (RPC-190).
//!
//! Feature: spec/features/add-scenario-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::add_scenario::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_scenario::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-scenario.ts:27`).
//! No domain logic in this bridge — JSON marshalling + result-field printing
//! only.
//!
//! Exit-code contract (parity with TS at
//! `src/commands/add-scenario.ts:148-172`):
//!   - 0 on success; prints an optional `⚠ <warning>` line then
//!     `✓ Added scenario "<name>"` (all on stdout).
//!   - 1 when the core returns an inner `success:false` envelope; the
//!     `error` is written to stderr and the optional `Suggestion:` line
//!     is written to stdout (parity with TS `output.error`/`output.log`).
//!   - 1 on any hard [`codelet_fspec_core::FspecCoreError`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_scenario;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-scenario.ts:175-184`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature: String,
    pub scenario: String,
}

/// Entry point invoked from `main.rs` for the `add-scenario` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let scenario_name = args.scenario.clone();
    let body = json!({
        "feature": args.feature,
        "scenario": args.scenario,
    });
    let args_json = body.to_string();

    match add_scenario::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value =
                serde_json::from_str(&data_json).context("parse core response as JSON")?;

            if parsed.get("success").and_then(Value::as_bool) == Some(true) {
                if let Some(w) = parsed.get("warning").and_then(Value::as_str) {
                    println!("⚠ {w}");
                }
                println!("✓ Added scenario \"{scenario_name}\"");
                Ok(0)
            } else {
                if let Some(e) = parsed.get("error").and_then(Value::as_str) {
                    eprintln!("Error: {e}");
                }
                if let Some(s) = parsed.get("suggestion").and_then(Value::as_str) {
                    println!("Suggestion: {s}");
                }
                Ok(1)
            }
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
