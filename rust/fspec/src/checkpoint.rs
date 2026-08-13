//! `checkpoint` shell-facing CLI bridge (RPC-202).
//!
//! Feature: spec/features/checkpoint-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::Checkpoint` clap variant in [`crate::main`]) and delegates
//! to the single source-of-truth in
//! [`codelet_fspec_core::commands::checkpoint::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes. No capture, index-write, or
//! rendering logic lives here; the bridge's only computation is JSON arg
//! marshalling + exit-code selection.
//!
//! Exit-code contract: 0 when a checkpoint was created, 1 when the working
//! tree was clean (nothing to capture) or on any
//! [`codelet_fspec_core::FspecCoreError`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::checkpoint;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/checkpoint.ts:193-199`): two positional arguments,
/// `<work-unit-id>` and `<checkpoint-name>`, with no `.option(...)` flags.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub checkpoint_name: String,
}

/// Entry point invoked from `main.rs` for the `checkpoint` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Structured call: drives the exit code from the core's `success` flag
    // (a clean working tree returns `success:false`).
    let structured_args = json!({
        "workUnitId": args.work_unit_id,
        "checkpointName": args.checkpoint_name,
        "format": "json",
    })
    .to_string();

    let succeeded = match checkpoint::run(&structured_args, &project_root).await {
        Ok(rendered) => serde_json::from_str::<Value>(&rendered)
            .ok()
            .and_then(|v| v.get("success").and_then(Value::as_bool))
            .unwrap_or(false),
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    };

    // Text call: produces the human-facing output verbatim from the core.
    let text_args = json!({
        "workUnitId": args.work_unit_id,
        "checkpointName": args.checkpoint_name,
        "format": "text",
    })
    .to_string();

    match checkpoint::run(&text_args, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    }

    Ok(if succeeded { 0 } else { 1 })
}
