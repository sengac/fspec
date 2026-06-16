//! `generate-summary-report` shell-facing CLI bridge (RPC-235).
//!
//! Feature: spec/features/generate-summary-report-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::generate_summary_report::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::generate_summary_report::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/generate-summary-report.ts:35`).
//!
//! Unset `--format` / `--output` are omitted from the JSON so fspec_core's
//! `#[serde(default)]` arms fire and the TS defaults (`markdown` format,
//! `spec/summary-report.<ext>` output) apply.
//!
//! Exit-code contract:
//!   - 0 on success; the `✓ Report generated: <file>` message is written to
//!     stdout (parity with the TS `output.log` path at
//!     `src/commands/generate-summary-report.ts:135`).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to generate report:` (parity with
//!     the TS chalk-red error path at
//!     `src/commands/generate-summary-report.ts:136-138`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_summary_report;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/generate-summary-report.ts:124-128`): two options
/// `--format <format>` and `--output <file>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub format: Option<String>,
    pub output: Option<String>,
}

/// Entry point invoked from `main.rs` for the `generate-summary-report` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    if let Some(v) = args.format.as_ref() {
        obj.insert("format".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.output.as_ref() {
        obj.insert("output".to_string(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    match generate_summary_report::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // TS: output.log(`✓ Report generated: ${result.outputFile}`).
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to generate report: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
