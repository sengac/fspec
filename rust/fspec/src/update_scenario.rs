//! `update-scenario` shell-facing CLI bridge (RPC-314).
//!
//! Feature: spec/features/update-scenario-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::update_scenario::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::update_scenario::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/update-scenario.ts:25`).
//! No domain logic in this bridge — JSON marshalling + ✓-prefix printing
//! and success/error stream routing only.
//!
//! Exit-code contract (parity with TS
//! `src/commands/update-scenario.ts:183-196`):
//!   - 0 on success; the canonical `message` field returned by core is
//!     prefixed with `✓ ` and written to stdout.
//!   - 1 on a soft failure (`{success:false,error}`): the `error` text is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', result.error); process.exit(1)` branch).
//!   - 1 on any escalated [`codelet_fspec_core::FspecCoreError`]; the
//!     unwrapped reason is written to stderr prefixed with `Error:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_scenario;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/update-scenario.ts:199-206`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub file: String,
    pub old_name: String,
    pub new_name: String,
}

/// Entry point invoked from `main.rs` for the `update-scenario` clap
/// subcommand. Returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "feature": args.file,
        "oldName": args.old_name,
        "newName": args.new_name,
    });
    let args_json = body.to_string();

    match update_scenario::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let v: Value = serde_json::from_str(&data_json).context("parse core JSON response")?;
            if v.get("success").and_then(Value::as_bool) == Some(false) {
                // Soft failure — parity with TS
                // `output.error('Error:', result.error); process.exit(1)`.
                let err = v.get("error").and_then(Value::as_str).unwrap_or("");
                eprintln!("Error: {err}");
                return Ok(1);
            }
            let message = v.get("message").and_then(Value::as_str).unwrap_or("");
            println!("✓ {message}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
