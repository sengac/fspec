//! `bootstrap` shell-facing CLI bridge (RPC-200).
//!
//! Feature: spec/features/bootstrap-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::Bootstrap` clap variant in [`crate::main`]) and delegates
//! to the single source-of-truth in
//! [`codelet_fspec_core::commands::bootstrap::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::bootstrap::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::bootstrap::run
//!
//! This bridge embeds NO documentation-building or transform logic — its only
//! computation is JSON arg marshalling and stdout/stderr printing. All
//! document assembly, config replacement, and the Event Storm reminder live
//! in fspec_core.
//!
//! Exit-code contract (parity with bootstrap.ts:263-273):
//!   - 0 on success; the rendered documentation is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr as `Error running bootstrap: <message>`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::bootstrap;

use crate::common::render_core_error;

/// `bootstrap` takes no positional arguments and no flags, mirroring the
/// flag-less TS Commander.js registration (`src/commands/bootstrap.ts:258-262`).
/// The struct is retained for signature symmetry with the other bridges; the
/// JSON shape handed to fspec_core always serialises to `{}`.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `bootstrap` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(_args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // No flags → the empty JSON object (parity with the TS no-args command and
    // fspec_core's `_args_json` being ignored).
    let args_json = "{}".to_string();

    match bootstrap::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // fspec_core returns the documentation without a trailing newline;
            // println! appends the single newline that the TS `output.log`
            // (console.log) path emits, preserving byte parity.
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error running bootstrap: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
