//! `add-virtual-hook` shell-facing CLI bridge (RPC-195).
//!
//! Feature: spec/features/add-virtual-hook-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddVirtualHook` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_virtual_hook::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         -> clap -> this module -> fspec_core::commands::add_virtual_hook::run
//!   - LLM tool call JSON -> fspec_core::dispatch::dispatch_command -> fspec_core::commands::add_virtual_hook::run
//!
//! Bridge scope: argv -> JSON marshalling + stdout/stderr rendering only.
//! Hook-name derivation, script generation, work-unit lookup, and disk I/O
//! all live in the core. The bridge MUST NOT embed any of those concerns —
//! see the `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher`
//! test which scans this file for forbidden substrings.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_virtual_hook;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/add-virtual-hook.ts:95-110`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub event: String,
    pub command: String,
    pub blocking: bool,
    pub git_context: bool,
}

#[derive(serde::Deserialize)]
struct CoreResultEnvelope {
    #[serde(rename = "hookCount")]
    hook_count: usize,
}

/// Entry point invoked from `main.rs` for the `add-virtual-hook` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let work_unit_id = args.work_unit_id.clone();

    let mut body = Map::new();
    body.insert(
        "workUnitId".to_string(),
        Value::String(work_unit_id.clone()),
    );
    body.insert("event".to_string(), Value::String(args.event));
    body.insert("command".to_string(), Value::String(args.command));
    body.insert("blocking".to_string(), Value::Bool(args.blocking));
    if args.git_context {
        body.insert("gitContext".to_string(), Value::Bool(true));
    }
    let args_json = json!(body).to_string();

    match add_virtual_hook::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let envelope: CoreResultEnvelope =
                serde_json::from_str(&data_json).context("parse core result envelope")?;
            println!("✓ Virtual hook added to {work_unit_id}");
            println!("  Total virtual hooks: {}", envelope.hook_count);
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add virtual hook: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
