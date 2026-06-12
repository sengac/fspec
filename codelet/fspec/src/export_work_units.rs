//! `export-work-units` shell-facing CLI bridge (RPC-229).
//!
//! Feature: spec/features/export-work-units-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::export_work_units::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::export_work_units::run
//!
//! ## Framing A — the broken TS success line, mirrored verbatim
//!
//! The TypeScript Commander action (`src/commands/export-work-units.ts:57-81`)
//! logs `✓ Exported ${result.count} work units to ${result.outputFile}` — but
//! `exportWorkUnits` only ever returns `{ success: true }`, so `result.count`
//! and `result.outputFile` are `undefined`. The shell therefore prints
//! `✓ Exported undefined work units to undefined` on success. This Rust bridge
//! reproduces that broken success line exactly. The file write itself (the
//! real side-effect) is performed by fspec_core.
//!
//! Exit-code contract:
//!   - 0 on success (file written), with the broken `undefined`/`undefined`
//!     success line on stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to export work units:` mirroring the
//!     TS `output.error('✗ Failed to export work units:', message)` path.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::export_work_units;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js registration
/// at `src/commands/export-work-units.ts:50-81`. `format` + `output` are
/// positional; `--status` is accepted but ignored by the core (TS parity).
#[derive(Debug)]
pub struct CliArgs {
    pub format: String,
    pub output: String,
    pub status: Option<String>,
}

/// Entry point invoked from `main.rs` for the `export-work-units` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by
    // fspec_core::commands::export_work_units::run. `status` is threaded
    // through for shape parity but the core ignores it (TS quirk).
    let mut body = serde_json::Map::new();
    body.insert("format".to_string(), json!(args.format));
    body.insert("output".to_string(), json!(args.output));
    if let Some(status) = args.status.as_ref() {
        body.insert("status".to_string(), json!(status));
    }
    let args_json = serde_json::Value::Object(body).to_string();

    match export_work_units::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            // Framing A: the TS success line references result.count and
            // result.outputFile, both undefined on the {success:true} payload.
            println!("✓ Exported undefined work units to undefined");
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to export work units: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
