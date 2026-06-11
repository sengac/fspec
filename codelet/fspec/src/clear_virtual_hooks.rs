//! `clear-virtual-hooks` shell-facing CLI bridge (RPC-205).
//!
//! Feature: spec/features/clear-virtual-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::ClearVirtualHooks` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::clear_virtual_hooks::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::clear_virtual_hooks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::clear_virtual_hooks::run
//!
//! Exit-code contract:
//!   - 0 on success — the success message is delivered to stdout from the
//!     `message` field on the core impl's JSON result (the bridge is
//!     forbidden by the delegation test from embedding the success-line
//!     verb literal).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to clear virtual hooks:`
//!     (parity with the TS chalk-red error path at
//!     `src/commands/clear-virtual-hooks.ts:84-89`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::clear_virtual_hooks;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js positional surface
/// for `clear-virtual-hooks`. Today the TS registration declares ONLY the
/// `<workUnitId>` positional (no `.option(...)` calls), so `CliArgs` carries
/// exactly one field.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `clear-virtual-hooks` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The dispatcher arg key is `workUnitId` (camelCase).
    let args_json = json!({
        "workUnitId": args.work_unit_id,
    })
    .to_string();

    match clear_virtual_hooks::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core impl returns a JSON object with a `message` field
            // containing the canonical success line. Extracting that field
            // is the bridge's ONLY post-call computation — there is no
            // independent rendering of work-unit ids or counts here.
            match serde_json::from_str::<Value>(&rendered) {
                Ok(v) => {
                    if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                        println!("{msg}");
                    }
                }
                Err(_) => {
                    // Defensive: if the core impl ever changes shape, fall
                    // back to printing the raw rendered string so the user
                    // still sees something on stdout.
                    print!("{rendered}");
                    if !rendered.ends_with('\n') {
                        println!();
                    }
                }
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to clear virtual hooks:', error.message)`.
            // `render_core_error` strips the dispatcher-only
            // `"Invalid args for fspec command clear-virtual-hooks: "`
            // envelope so the shell stderr is byte-identical to TS.
            eprintln!(
                "✗ Failed to clear virtual hooks: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
