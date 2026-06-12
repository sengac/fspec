//! `add-bounded-context` shell-facing CLI bridge (RPC-172).
//!
//! Feature: spec/features/add-bounded-context-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddBoundedContext` clap variant in
//! [`crate::main`]) and delegates to the SAME source-of-truth the
//! LLM-facing dispatcher uses — `add-bounded-context` routed through
//! [`codelet_fspec_core::dispatch_command`].
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → dispatch_command → core
//!   - LLM tool call JSON → dispatch_command → core
//!
//! This bridge performs ONLY JSON arg marshalling: it builds the args
//! object, dispatches, and renders the TS-parity stdout/stderr lines. It
//! contains NO item construction, status guard, or file-write logic — all
//! domain behaviour lives in `fspec-core`. Routing through the kebab
//! command string (not the snake module path) keeps the bridge free of the
//! core's internal identifiers.
//!
//! One subtlety: `dispatch_command` wraps validation failures in the
//! LLM-tool envelope `"Invalid args for fspec command <name>: <reason>"`.
//! The TS shell user never sees that framing — TS prints only `<reason>`.
//! The bridge therefore strips the envelope (see
//! [`crate::common::strip_dispatch_envelope`]) before rendering stderr, so
//! the shell surface is byte-parity with the TS binary's
//! `output.error('✗ Failed to add bounded context:', error.message)` line.
//!
//! Exit-code contract:
//!   - 0 on success; the TS success line is written to stdout.
//!   - 1 on any failure; the message is written to stderr prefixed with
//!     `✗ Failed to add bounded context:` (parity with the TS
//!     `output.error('✗ Failed to add bounded context:', error.message)`
//!     path at `src/commands/add-bounded-context.ts:102-119`).

use std::env;

use anyhow::{Context, Result};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{Map, Value};

use crate::common::strip_dispatch_envelope;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-bounded-context.ts:72-99`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub description: Option<String>,
    pub timestamp: Option<String>,
    /// Parent association supplied via `--bounded-context`.
    pub context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-bounded-context` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → the SAME serde shape the dispatcher feeds the
    // core. Optionals are only inserted when present (parity with the TS
    // command's conditional field assignment).
    let mut body = Map::new();
    body.insert("workUnitId".to_string(), Value::String(args.work_unit_id.clone()));
    body.insert("text".to_string(), Value::String(args.text));
    if let Some(d) = args.description {
        body.insert("description".to_string(), Value::String(d));
    }
    if let Some(t) = args.timestamp {
        body.insert(
            "timestamp".to_string(),
            codelet_fspec_core::js_compat::parse_js_int(&t),
        );
    }
    if let Some(c) = args.context {
        body.insert("boundedContext".to_string(), Value::String(c));
    }
    let args_json = Value::Object(body).to_string();

    let result = dispatch_command(DispatchRequest {
        command: "add-bounded-context".to_string(),
        args_json,
        project_root,
    });

    if result.success {
        let id = serde_json::from_str::<Value>(&result.data)
            .ok()
            .and_then(|v| v.get("boundedContextId").and_then(Value::as_u64))
            .unwrap_or(0);
        println!(
            "✓ Bounded context added to {} (id: {})",
            args.work_unit_id, id
        );
        Ok(0)
    } else {
        // `dispatch_command` wraps validation failures in the LLM-tool
        // envelope `"Invalid args for fspec command <name>: <reason>"`. The
        // TS shell user never sees that framing — TS prints only `<reason>`
        // via `output.error('✗ Failed to add bounded context:',
        // error.message)`. Strip the envelope prefix so stderr is
        // byte-parity with the TS binary.
        let raw = result.error.unwrap_or_default();
        let reason = strip_dispatch_envelope(&raw);
        eprintln!("✗ Failed to add bounded context: {reason}");
        Ok(1)
    }
}
