//! `copy-virtual-hooks` shell-facing CLI bridge (RPC-209).
//!
//! Feature: spec/features/copy-virtual-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::CopyVirtualHooks` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::copy_virtual_hooks::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::copy_virtual_hooks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::copy_virtual_hooks::run
//!
//! ## --from / --to presence guard
//!
//! The TS Commander.js action handler enforces the canonical
//! `"--from option is required"` / `"--to option is required"` messages
//! BEFORE invoking the core function (see
//! `src/commands/copy-virtual-hooks.ts:103-110`). The core impl also
//! enforces them so dispatcher callers see the same canonical messages.
//! Either path produces the same stderr byte-for-byte.
//!
//! Exit-code contract:
//!   - 0 on success — the success line is delivered to stdout from the
//!     `message` field on the core impl's JSON result (the bridge is
//!     forbidden by the delegation test from embedding any of the
//!     canonical success/error verbatim literals).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to copy virtual hooks:`
//!     (parity with TS chalk-red error path at
//!     `src/commands/copy-virtual-hooks.ts:123-128`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::copy_virtual_hooks;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js flag set at
/// `src/commands/copy-virtual-hooks.ts:96-103`. All three are optional in
/// clap — the presence guards for `--from` / `--to` live inside the
/// fspec_core impl which surfaces the canonical messages.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub from: Option<String>,
    pub to: Option<String>,
    pub hook_name: Option<String>,
}

/// Entry point invoked from `main.rs` for the `copy-virtual-hooks` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Marshal CLI args into the JSON shape consumed by fspec_core.
    let mut obj = serde_json::Map::new();
    if let Some(v) = args.from.as_ref() {
        obj.insert("from".into(), Value::String(v.clone()));
    }
    if let Some(v) = args.to.as_ref() {
        obj.insert("to".into(), Value::String(v.clone()));
    }
    if let Some(v) = args.hook_name.as_ref() {
        obj.insert("hookName".into(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    match copy_virtual_hooks::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Extract the canonical message field from the core impl's
            // JSON response. The bridge does NOT format counts or ids.
            match serde_json::from_str::<Value>(&rendered) {
                Ok(v) => {
                    if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                        println!("{msg}");
                    }
                }
                Err(_) => {
                    print!("{rendered}");
                    if !rendered.ends_with('\n') {
                        println!();
                    }
                }
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to copy virtual hooks: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
