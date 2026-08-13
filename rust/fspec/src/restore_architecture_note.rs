//! `restore-architecture-note` shell-facing CLI bridge (RPC-287).
//!
//! Feature: spec/features/restore-architecture-note-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::RestoreArchitectureNote` clap
//! variant in [`crate::main`]) and delegates to the single
//! source-of-truth in
//! [`codelet_fspec_core::commands::restore_architecture_note::run`] —
//! the SAME function the LLM-facing dispatcher invokes.
//!
//! NO logic in the bridge — JSON marshalling + a fixed success line.
//!
//! Exit-code contract:
//!   - 0 on success; the canonical success line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::restore_architecture_note;
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

    match restore_architecture_note::run(&args_json, &project_root).await {
        Ok(data) => {
            // TS wrapper prints a fixed success line, then surfaces the
            // optional `message` field (idempotent path) — see
            // `src/commands/restore-architecture-note.ts:96-100`.
            println!("✓ Architecture note restored successfully");
            let parsed: serde_json::Value =
                serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                println!("  {msg}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
