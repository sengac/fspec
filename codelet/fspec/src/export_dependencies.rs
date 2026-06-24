//! `export-dependencies` shell-facing CLI bridge (RPC-227).
//!
//! Feature: spec/features/export-dependencies-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::export_dependencies::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::export_dependencies::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/export-dependencies.ts:85`).
//!
//! Exit-code contract:
//!   - 0 on success; the `✓ Dependencies exported to <output>` message is
//!     written to stdout (the file write itself is performed by fspec_core).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to export dependencies:` (parity with
//!     the TS chalk-red error path at `src/commands/export-dependencies.ts:141-144`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::export_dependencies;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/export-dependencies.ts:127-132`): two positional arguments
/// `<format>` and `<output>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub format: String,
    pub output: String,
}

/// Entry point invoked from `main.rs` for the `export-dependencies` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    obj.insert("format".to_string(), json!(args.format));
    obj.insert("output".to_string(), json!(args.output));
    let args_json = Value::Object(obj).to_string();

    match export_dependencies::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // TS: output.log(chalk.green(`✓ Dependencies exported to ...`)).
            // Under a non-TTY pipe chalk disables colour, so the plain message
            // is byte-correct parity.
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to export dependencies: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
