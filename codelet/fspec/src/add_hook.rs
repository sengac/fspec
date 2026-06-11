//! `add-hook` shell-facing CLI bridge (RPC-184).
//!
//! Feature: spec/features/add-hook-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddHook` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_hook::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_hook::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_hook::run
//!
//! Exit-code contract (parity with TS `add-hook.ts:82-95` action handler):
//!   - 0 on success; stdout is exactly zero bytes (TS action prints nothing).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; stderr starts with
//!     `Error:` followed by the rendered message.
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - argv → JSON marshalling (`workUnitId` not applicable; only event,
//!     name, command, blocking, timeout fields)
//!   - error rendering via [`crate::common::render_core_error`]
//!
//! Load, mutate, write, atomic-rename, and timeout-Option omission all live
//! in the core. The bridge MUST NOT embed `write_json_atomic` or
//! `read_to_string` or any HookFile shape.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_hook;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/add-hook.ts:56-95`.
#[derive(Debug)]
pub struct CliArgs {
    pub event: String,
    pub name: String,
    pub command: String,
    pub blocking: bool,
    pub timeout: Option<u64>,
}

/// Entry point invoked from `main.rs` for the `add-hook` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core. `timeout`
    // is omitted entirely when `None` (parity with `JSON.stringify(undefined)`).
    let mut body = Map::new();
    body.insert("event".to_string(), Value::String(args.event));
    body.insert("name".to_string(), Value::String(args.name));
    body.insert("command".to_string(), Value::String(args.command));
    body.insert("blocking".to_string(), Value::Bool(args.blocking));
    if let Some(t) = args.timeout {
        body.insert("timeout".to_string(), Value::Number(t.into()));
    }
    let args_json = json!(body).to_string();

    match add_hook::run(&args_json, &project_root).await {
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
