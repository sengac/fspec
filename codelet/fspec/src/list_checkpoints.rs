//! `list-checkpoints` shell-facing CLI bridge (RPC-242).
//!
//! Feature: spec/features/list-checkpoints-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ListCheckpoints` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_checkpoints::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-242):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_checkpoints::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_checkpoints::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/list-checkpoints.ts:69`). The clap
//! subcommand carries NO flags — matching the flag-less TS Commander.js
//! registration at `src/commands/list-checkpoints.ts:83-88`. No checkpoint-
//! listing, classification, or rendering logic is duplicated here.
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-242):
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_checkpoints;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// declaration for `list-checkpoints`
/// (`src/commands/list-checkpoints.ts:83-88`).
///
/// Only the positional `<work-unit-id>` is exposed; the TS registration
/// declares no `.option(...)` flags. We marshal the value into the JSON
/// shape consumed by `fspec_core::commands::list_checkpoints::run`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Positional `<work-unit-id>` from clap.
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `list-checkpoints` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core::commands::
    // list_checkpoints::run validates with serde. We deliberately do NOT
    // forward a `format` field here so the bridge inherits the TS text
    // default — matching the chalk-stripped output that
    // `node dist/index.js list-checkpoints AUTH-001` produces today.
    let payload = json!({ "workUnitId": args.work_unit_id });
    let args_json = payload.to_string();

    match list_checkpoints::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure; print
            // as-is and avoid a duplicate \n that would shift the header.
            // The empty-result sentinel (rendered by fspec_core) has no
            // trailing newline, so we append one for shell-pipeline
            // friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required for parity with RPC-253 rule [14].
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
