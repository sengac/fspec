//! `remove-command-from-foundation` shell-facing CLI bridge (RPC-270).
//!
//! Feature: spec/features/remove-command-from-foundation-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveCommandFromFoundation` clap variant in
//! [`crate::main`]) and delegates to the SAME source-of-truth the
//! LLM-facing dispatcher uses — `remove-command-from-foundation` routed
//! through [`codelet_fspec_core::dispatch_command`].
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → dispatch_command → core
//!   - LLM tool call JSON → dispatch_command → core
//!
//! This bridge performs ONLY JSON arg marshalling: it builds the args
//! object, dispatches, and renders the TS-parity stdout/stderr lines. All
//! domain behaviour lives in `fspec-core`.
//!
//! Exit-code contract:
//!   - 0 on success; `✓ <message>` is written to stdout (parity with the
//!     TS `output.log('✓', result.message)` line).
//!   - 1 on any failure; `Error: <reason>` is written to stderr (parity
//!     with the TS `output.error('Error:', error.message)` path).

use std::env;

use anyhow::{Context, Result};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{Map, Value};

use crate::common::strip_dispatch_envelope;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/remove-command-from-foundation.ts:131-154`.
#[derive(Debug)]
pub struct CliArgs {
    pub context_name: String,
    pub command_name: String,
}

/// Entry point invoked from `main.rs` for the
/// `remove-command-from-foundation` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → the SAME serde shape the dispatcher feeds the core.
    let mut body = Map::new();
    body.insert("contextName".to_string(), Value::String(args.context_name));
    body.insert("commandName".to_string(), Value::String(args.command_name));
    let args_json = Value::Object(body).to_string();

    let result = dispatch_command(DispatchRequest {
        command: "remove-command-from-foundation".to_string(),
        args_json,
        project_root,
    });

    if result.success {
        let message = serde_json::from_str::<Value>(&result.data)
            .ok()
            .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        println!("✓ {message}");
        Ok(0)
    } else {
        let raw = result.error.unwrap_or_default();
        let reason = strip_dispatch_envelope(&raw);
        eprintln!("Error: {reason}");
        Ok(1)
    }
}
