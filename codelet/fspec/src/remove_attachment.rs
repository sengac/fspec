//! `remove-attachment` shell-facing CLI bridge (RPC-268).
//!
//! Feature: spec/features/remove-attachment-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveAttachment` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_attachment::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_attachment::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_attachment::run
//!
//! Bridge scope: JSON arg marshalling + stdout/stderr rendering only.
//! No splice, no file unlink, no atomic write.
//!
//! Exit-code contract:
//!   - 0 on success — the core's rendered multi-line output is written
//!     to stdout as-is. This includes BOTH the ✓ "and file deleted" path
//!     AND the ⚠ "already missing" path (TS exits 0 on both).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]. The message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', errorMessage)` line at
//!     `src/commands/remove-attachment.ts:112`).
//!   - 2 (clap's own usage error) when a required positional is omitted.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_attachment;
use serde_json::json;

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub file_name: String,
    pub keep_file: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut body = serde_json::Map::new();
    body.insert(
        "workUnitId".to_string(),
        serde_json::Value::String(args.work_unit_id.clone()),
    );
    body.insert(
        "fileName".to_string(),
        serde_json::Value::String(args.file_name.clone()),
    );
    if args.keep_file {
        body.insert("keepFile".to_string(), serde_json::Value::Bool(true));
    }
    let args_json = json!(body).to_string();

    match remove_attachment::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
