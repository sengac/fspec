//! `add-external-system` shell-facing CLI bridge (RPC-182).
//!
//! Feature: spec/features/add-external-system-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddExternalSystem` clap variant in
//! [`crate::main`]) and delegates to the SAME source-of-truth the
//! LLM-facing dispatcher uses — the `add-external-system` command routed
//! through [`codelet_fspec_core::dispatch_command`].
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
//! the shell surface is byte-parity with the TS binary.
//!
//! Exit-code contract:
//!   - 0 on success; the TS success line is written to stdout.
//!   - 1 on any failure; the message is written to stderr prefixed with
//!     `✗ Failed to add external system:` (parity with the TS
//!     `output.error('✗ Failed to add external system:', error.message)`
//!     path at `src/commands/add-external-system.ts:108-126`).

use std::env;

use anyhow::{Context, Result};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{Map, Value};

use crate::common::strip_dispatch_envelope;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-external-system.ts:77-106`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    /// Integration category supplied via `--type` (REST_API, MESSAGE_QUEUE,
    /// DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM).
    pub system_type: Option<String>,
    pub timestamp: Option<String>,
    /// Domain association supplied via `--bounded-context`.
    pub context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-external-system` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → the SAME serde shape the dispatcher feeds the
    // core. Optionals are only inserted when present (parity with the TS
    // command's conditional field assignment).
    let mut body = Map::new();
    body.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );
    body.insert("text".to_string(), Value::String(args.text));
    if let Some(t) = args.system_type {
        body.insert("type".to_string(), Value::String(t));
    }
    if let Some(ts) = args.timestamp {
        body.insert(
            "timestamp".to_string(),
            codelet_fspec_core::js_compat::parse_js_int(&ts),
        );
    }
    if let Some(c) = args.context {
        body.insert("boundedContext".to_string(), Value::String(c));
    }
    let args_json = Value::Object(body).to_string();

    let result = dispatch_command(DispatchRequest {
        command: "add-external-system".to_string(),
        args_json,
        project_root,
    });

    if result.success {
        let id = serde_json::from_str::<Value>(&result.data)
            .ok()
            .and_then(|v| v.get("externalSystemId").and_then(Value::as_u64))
            .unwrap_or(0);
        println!(
            "✓ External system added to {} (id: {})",
            args.work_unit_id, id
        );
        Ok(0)
    } else {
        // `dispatch_command` wraps validation failures in the LLM-tool
        // envelope `"Invalid args for fspec command <name>: <reason>"`. The
        // TS shell user never sees that framing — TS prints only `<reason>`
        // via `output.error('✗ Failed to add external system:',
        // error.message)`. Strip the envelope prefix so stderr is
        // byte-parity with the TS binary.
        let raw = result.error.unwrap_or_default();
        let reason = strip_dispatch_envelope(&raw);
        eprintln!("✗ Failed to add external system: {reason}");
        Ok(1)
    }
}
