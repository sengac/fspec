//! `restore-question` shell-facing CLI bridge (RPC-290).
//!
//! Feature: spec/features/restore-question-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::RestoreQuestion` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::restore_question::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! NO logic in the bridge — JSON marshalling + a success line sourced
//! from the dispatcher result.
//!
//! Exit-code contract:
//!   - 0 on success; the canonical success line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to restore question:`,
//!     matching TS at `src/commands/restore-question.ts:107` —
//!     `output.error('✗ Failed to restore question:', error.message)`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::restore_question;
use serde_json::json;

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub index: u64,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "workUnitId": args.work_unit_id,
        "index": args.index,
    });
    let args_json = body.to_string();

    match restore_question::run(&args_json, &project_root).await {
        Ok(data) => {
            // Parse the dispatcher JSON to extract `restoredQuestion` and
            // the optional `message`, mirroring TS at
            // `src/commands/restore-question.ts:100-105` which prints
            // `✓ Restored question: "${result.restoredQuestion}"` and, if
            // present, an indented `  ${result.message}` line.
            let parsed: serde_json::Value =
                serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
            let text = parsed
                .get("restoredQuestion")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("✓ Restored question: \"{text}\"");
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                println!("  {msg}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to restore question: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
