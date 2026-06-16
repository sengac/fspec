//! `format` shell-facing CLI bridge (RPC-230).
//!
//! Feature: spec/features/format-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default), marshals the
//! optional `[file]` positional into JSON, and delegates to the single
//! source-of-truth in [`codelet_fspec_core::commands::format::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Rendering parity with `formatCommand` at `src/commands/format.ts:105-127`:
//!   - formattedCount === 0 → 'No feature files found to format', exit 0.
//!   - file argument         → '✓ Formatted <file>', exit 0.
//!   - otherwise (all files) → green '✓ Formatted N feature files', exit 0.
//!   - any core error (e.g. missing single file) → 'Error: <message>' to
//!     stderr, exit 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::format;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/format.ts:129-134` (one optional positional `[file]`, no
/// flags).
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Optional positional `[file]` to format (formats all when absent).
    pub file: Option<String>,
}

/// Entry point invoked from `main.rs` for the `format` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut obj: Map<String, Value> = Map::new();
    if let Some(file) = args.file.as_deref() {
        obj.insert("file".to_string(), Value::String(file.to_string()));
    }
    let args_json = json!(obj).to_string();

    match format::run(&args_json, &project_root).await {
        Ok(payload) => {
            let value: Value =
                serde_json::from_str(&payload).context("parse format JSON payload")?;
            let count = value
                .get("formattedCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);

            if count == 0 {
                println!("No feature files found to format");
            } else if let Some(file) = args.file.as_deref() {
                println!("✓ Formatted {file}");
            } else {
                println!("✓ Formatted {count} feature files");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
