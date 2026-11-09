//! `add-aggregate` shell-facing CLI bridge (RPC-165).
//!
//! Feature: spec/features/add-aggregate-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::AddAggregate` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_aggregate::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_aggregate::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → …::add_aggregate::run
//!
//! Console output and exit code (BUG-532):
//!   - success: `✓ Added aggregate "<text>" to <workUnitId> (ID: <id>)` on
//!     stdout, exit 0 (like add-domain-event);
//!   - failure: `✗ Failed to add aggregate: <error>` on stderr, exit 1.
//!
//! The TypeScript CLI printed nothing and exited 1 even on success (its
//! file-only logger had no `success` level); that bug is not kept.

use crate::common::spec_project_root;
use std::path::PathBuf;

use anyhow::Result;
use codelet_fspec_core::commands::add_aggregate;

use crate::common::render_core_error;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-aggregate.ts:152-189`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub responsibilities: Option<String>,
    pub timestamp: Option<String>,
    pub bounded_context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-aggregate` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;
    let work_unit_id = args.work_unit_id;
    let text = args.text;

    // Marshal clap args → JSON object expected by the core. Only present
    // optionals are included so serde's `Option` defaults stay `None`.
    let mut body = json!({
        "workUnitId": work_unit_id,
        "text": text,
    });
    if let Some(map) = body.as_object_mut() {
        if let Some(r) = args.responsibilities {
            map.insert("responsibilities".to_string(), Value::String(r));
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

    match add_aggregate::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns {success, aggregateId}.
            let aggregate_id = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| v.get("aggregateId").and_then(Value::as_u64));
            let Some(id) = aggregate_id else {
                eprintln!(
                    "✗ Failed to add aggregate: the result carries no aggregateId: {data_json}"
                );
                return Ok(1);
            };
            println!("✓ Added aggregate \"{text}\" to {work_unit_id} (ID: {id})");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add aggregate: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
