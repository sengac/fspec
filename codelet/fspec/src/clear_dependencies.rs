//! `clear-dependencies` shell-facing CLI bridge (RPC-204).
//!
//! Feature: spec/features/clear-dependencies-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ClearDependencies` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::clear_dependencies::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::clear_dependencies::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::clear_dependencies::run
//!
//! Both call sites pass the canonical JSON args shape
//! `{workUnitId, confirm}` and a `project_root: &Path`. NO domain logic
//! is duplicated here — the --confirm guard lives in the core.
//!
//! Exit-code contract:
//!   - 0 on success; the TS `output.log` message
//!     `✓ All dependencies cleared from <workUnitId>` is rendered on stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to clear dependencies:`
//!     (parity with the TS chalk-red error path at
//!     `src/commands/clear-dependencies.ts:116-121`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::clear_dependencies;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js flag set at
/// `src/commands/clear-dependencies.ts:101-112`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub confirm: bool,
}

/// Entry point invoked from `main.rs` for the `clear-dependencies` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CLI args into the canonical JSON shape consumed by
    // fspec_core::commands::clear_dependencies::run.
    let args_json = json!({
        "workUnitId": args.work_unit_id,
        "confirm": args.confirm,
    })
    .to_string();

    let work_unit_id = args.work_unit_id.clone();

    match clear_dependencies::run(&args_json, &project_root).await {
        Ok(_rendered) => {
            println!("✓ All dependencies cleared from {work_unit_id}");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to clear dependencies:', error.message)`.
            // `render_core_error` strips the dispatcher-only
            // `"Invalid args for fspec command clear-dependencies: "`
            // envelope so the shell stderr is byte-identical to TS.
            eprintln!(
                "✗ Failed to clear dependencies: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
