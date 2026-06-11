//! `remove-hook` shell-facing CLI bridge (RPC-275).
//!
//! Feature: spec/features/remove-hook-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveHook` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_hook::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_hook::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_hook::run
//!
//! Exit-code contract (parity with TS `remove-hook.ts:49-51` action handler):
//!   - 0 on success; stdout is exactly zero bytes (TS action prints nothing).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; stderr starts with
//!     `Error:` followed by the rendered message. Missing file (ENOENT)
//!     and invalid JSON both propagate as core errors (see core module
//!     architecture note — DIVERGES from add-hook).
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - argv → JSON marshalling (event, name only)
//!   - error rendering via [`crate::common::render_core_error`]
//!
//! Load, retain-filter, write, and atomic-rename all live in the core.
//! The bridge MUST NOT embed `write_json_atomic` or `read_to_string` or
//! any HookFile shape.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_hook;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/remove-hook.ts:37-51`.
#[derive(Debug)]
pub struct CliArgs {
    pub event: String,
    pub name: String,
}

/// Entry point invoked from `main.rs` for the `remove-hook` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core.
    let body = json!({
        "event": args.event,
        "name": args.name,
    });
    let args_json = body.to_string();

    match remove_hook::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            // TS Commander action prints nothing on success — silent exit 0.
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
