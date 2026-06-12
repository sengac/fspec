//! `add-command` shell-facing CLI bridge (RPC-174).
//!
//! Feature: spec/features/add-command-cli-subcommand.feature
//!
//! Thin clap façade for the `Mode::AddCommand` variant in [`crate::main`];
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_command::run`] — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_command::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → …::add_command::run
//!
//! Exit-code contract:
//!   - 0 on success; renders the TS-parity success line to stdout
//!     (`✓ Added command "<text>" to <id> (ID: <n>)`). The success line lives
//!     in the Commander.js `.action()` callback in the TS source, NOT in
//!     `addCommand()` (which returns `{success, commandId}`).
//!   - 1 on any error; written to stderr with the TS `output.error('✗ Failed
//!     to add command:', ...)` prefix.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_command;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-command.ts:149-185`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub actor: Option<String>,
    pub timestamp: Option<String>,
    pub bounded_context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-command` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core. Only present
    // optionals are included so serde's `Option` defaults stay `None`.
    let mut body = json!({
        "workUnitId": args.work_unit_id.clone(),
        "text": args.text.clone(),
    });
    if let Some(map) = body.as_object_mut() {
        if let Some(a) = args.actor {
            map.insert("actor".to_string(), Value::String(a));
        }
        if let Some(t) = args.timestamp {
            map.insert(
                "timestamp".to_string(),
                codelet_fspec_core::js_compat::parse_js_int(&t),
            );
        }
        if let Some(bc) = args.bounded_context {
            map.insert("boundedContext".to_string(), Value::String(bc));
        }
    }
    let args_json = body.to_string();

    match add_command::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let id = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| v.get("commandId").and_then(Value::as_u64))
                .unwrap_or(0);
            println!(
                "✓ Added command \"{}\" to {} (ID: {})",
                args.text, args.work_unit_id, id
            );
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add command: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
