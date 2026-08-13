//! `remove-schedule` shell-facing CLI bridge (RPC-280).
//!
//! Feature: spec/features/remove-schedule-rust-port.feature
//!          spec/features/remove-schedule-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses argv
//! (the `Mode::RemoveSchedule` clap variant in [`crate::main`]) and delegates
//! to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_schedule::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_schedule::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_schedule::run
//!
//! ALL schedule-mutation and file-writing logic lives in `fspec_core`. This
//! module performs JSON arg marshalling + delegation ONLY (enforced by the
//! cli_remove_schedule.rs thin-bridge guard).
//!
//! The clap subcommand takes a single positional `<name>` argument and no flags
//! (parity with the TS Commander.js registration at
//! `src/commands/schedule/remove-schedule.ts:40-44`).
//!
//! Exit-code contract: 0 on success (TS-parity confirmation printed to stdout);
//! 1 on any [`codelet_fspec_core::FspecCoreError`] (message to stderr prefixed
//! with `Error:`, parity with the TS chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_schedule;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js positional argument
/// for `remove-schedule`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// The positional `<name>` (schedule slug to remove).
    pub name: String,
}

/// Entry point invoked from `main.rs` for the `remove-schedule` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON object expected by fspec_core.
    let args_json = json!({ "name": args.name }).to_string();

    match remove_schedule::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Confirm the structured success payload before printing the
            // TS-parity acknowledgement (no domain logic in the bridge).
            let _parsed: Value =
                serde_json::from_str(&rendered).context("parse remove-schedule result payload")?;
            println!("✓ Schedule '{}' removed successfully", args.name);
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to remove schedule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
