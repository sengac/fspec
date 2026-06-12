//! `add-policy` shell-facing CLI bridge (RPC-187).
//!
//! Feature: spec/features/add-policy-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddPolicy` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_policy::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_policy::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_policy::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default).
//!
//! Exit-code contract:
//!   - 0 on success; prints `✓ Policy added to <id> (id: <policyId>)` (parity
//!     with the TS `output.log` line at src/commands/add-policy.ts:106-110).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to add policy:` (parity with the TS
//!     `output.error('✗ Failed to add policy:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_policy;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-policy.ts:73-116`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub when: Option<String>,
    pub then: Option<String>,
    pub timestamp: Option<String>,
    pub bounded_context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-policy` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // add_policy::run. The dispatcher and CLI both feed the SAME serde shape.
    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".to_string(), json!(args.work_unit_id));
    obj.insert("text".to_string(), json!(args.text));
    if let Some(w) = args.when {
        obj.insert("when".to_string(), json!(w));
    }
    if let Some(t) = args.then {
        obj.insert("then".to_string(), json!(t));
    }
    if let Some(ts) = args.timestamp {
        obj.insert(
            "timestamp".to_string(),
            codelet_fspec_core::js_compat::parse_js_int(&ts),
        );
    }
    if let Some(bc) = args.bounded_context {
        obj.insert("boundedContext".to_string(), json!(bc));
    }
    let args_json = Value::Object(obj).to_string();

    match add_policy::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns a JSON {success, policyId} payload. The CLI
            // surface mirrors the TS `output.log('✓ Policy added to <id>
            // (id: <policyId>)')` line at src/commands/add-policy.ts:106-110.
            let policy_id = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| v.get("policyId").and_then(Value::as_u64))
                .unwrap_or(0);
            println!(
                "✓ Policy added to {} (id: {})",
                args.work_unit_id, policy_id
            );
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to add policy:', error.message)`.
            eprintln!("✗ Failed to add policy: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
