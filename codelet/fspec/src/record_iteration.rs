//! `record-iteration` shell-facing CLI bridge (RPC-264).
//!
//! Feature: spec/features/record-iteration-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::record_iteration::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::record_iteration::run
//!
//! ## Framing A — the broken TS shell, mirrored verbatim
//!
//! The TypeScript Commander action (`src/commands/record-iteration.ts:65-77`)
//! wires `name`/`start`/`end` from argv and calls
//! `recordIteration({ name, start, end })` — it NEVER passes `workUnitId`.
//! The function then reads `data.workUnits[undefined]`, which is always
//! missing, so it throws `Work unit undefined not found`, the catch wraps it
//! as `Failed to record iteration: Work unit undefined not found`, and the
//! shell exits 1. This Rust bridge reproduces that broken behaviour: we send
//! NO `workUnitId` in the dispatcher args so the core's Framing A path
//! deterministically surfaces the same error and exit code.
//!
//! Exit-code contract:
//!   - 0 never reached on the happy path (the shell is broken by design).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to record iteration:` mirroring the
//!     TS `output.error('✗ Failed to record iteration:', error.message)` path.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::record_iteration;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js registration
/// at `src/commands/record-iteration.ts:58-77`. `name` / `start` / `end` are
/// parsed off argv exactly as the TS shell does — and, exactly as the TS
/// shell does, NONE of them are forwarded as `workUnitId` (Framing A).
#[derive(Debug)]
pub struct CliArgs {
    pub name: String,
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Entry point invoked from `main.rs` for the `record-iteration` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Framing A: deliberately DO NOT thread `name`/`start`/`end` into a
    // `workUnitId`. The dispatcher args carry no workUnitId, so the core
    // falls through to the canonical `Work unit undefined not found` error —
    // byte-parity with the broken TS Commander shell.
    let _ = (&args.name, &args.start, &args.end);
    let body = json!({});
    let args_json = body.to_string();

    match record_iteration::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            // Unreachable on the broken-shell path, but kept for symmetry with
            // the TS `output.log('✓ Iteration recorded successfully')` line.
            println!("✓ Iteration recorded successfully");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to record iteration: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
