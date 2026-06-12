//! `add-hotspot` shell-facing CLI bridge (RPC-185).
//!
//! Feature: spec/features/add-hotspot-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddHotspot` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_hotspot::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_hotspot::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_hotspot::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default; the TS hotspot path threads cwd through the
//! shared Event Storm helper).
//!
//! This bridge performs NO domain logic — no item construction, status guard,
//! or file write. Its only computation is JSON arg marshalling plus rendering
//! the TS-parity success/error lines. The success line lives in the TS action
//! callback (`src/commands/add-hotspot.ts:103-107`), NOT inside `addHotspot()`,
//! so it is correctly the bridge's responsibility here.
//!
//! Exit-code contract:
//!   - 0 on success; the success line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to add hotspot:` (parity with
//!     `src/commands/add-hotspot.ts:99`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_hotspot;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-hotspot.ts:69-90`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub concern: Option<String>,
    pub timestamp: Option<String>,
    pub bounded_context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-hotspot` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // add_hotspot::run. Optional fields are omitted when absent.
    let mut body = serde_json::Map::new();
    body.insert("workUnitId".to_string(), json!(args.work_unit_id));
    body.insert("text".to_string(), json!(args.text));
    if let Some(c) = &args.concern {
        body.insert("concern".to_string(), json!(c));
    }
    if let Some(ts) = args.timestamp {
        body.insert(
            "timestamp".to_string(),
            codelet_fspec_core::js_compat::parse_js_int(&ts),
        );
    }
    if let Some(bc) = &args.bounded_context {
        body.insert("boundedContext".to_string(), json!(bc));
    }
    let args_json = Value::Object(body).to_string();

    match add_hotspot::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns a JSON {success, hotspotId} payload. Mirror the
            // TS action-callback success line, which embeds the hotspot id.
            let hotspot_id = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| v.get("hotspotId").and_then(|e| e.as_u64()))
                .unwrap_or(0);
            println!(
                "✓ Hotspot added to {} (id: {hotspot_id})",
                args.work_unit_id
            );
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add hotspot: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
