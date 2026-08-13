//! `delete-work-unit` shell-facing CLI bridge (RPC-223).
//!
//! Feature: spec/features/delete-work-unit-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::DeleteWorkUnit` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::delete_work_unit::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::delete_work_unit::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::delete_work_unit::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/delete-work-unit.ts:25`).
//!
//! Exit-code contract:
//!   - 0 on success; the text rendered by the core (success line + any `⚠`
//!     warning lines) is written verbatim to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to delete work unit:`
//!     (parity with the TS `output.error('✗ Failed to delete work unit:',
//!     error.message)` path at `src/commands/delete-work-unit.ts:173-177`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_work_unit;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/delete-work-unit.ts:142-180`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub force: bool,
    pub skip_confirmation: bool,
    pub cascade_dependencies: bool,
}

/// Entry point invoked from `main.rs` for the `delete-work-unit` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core. The dispatcher
    // and CLI both feed the SAME serde shape.
    let body = json!({
        "workUnitId": args.work_unit_id,
        "force": args.force,
        "skipConfirmation": args.skip_confirmation,
        "cascadeDependencies": args.cascade_dependencies,
    });
    let args_json = body.to_string();

    match delete_work_unit::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to delete work unit: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
