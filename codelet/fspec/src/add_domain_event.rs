//! `add-domain-event` shell-facing CLI bridge (RPC-179).
//!
//! Feature: spec/features/add-domain-event-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddDomainEvent` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_domain_event::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_domain_event::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_domain_event::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-domain-event.ts:37`).
//!
//! This bridge performs NO domain logic — no item construction, dedup check,
//! status guard, or file write. Its only computation is JSON arg marshalling
//! plus rendering the TS-parity success/error lines. The success line lives
//! in the TS action callback (`src/commands/add-domain-event.ts:188-192`),
//! NOT inside `addDomainEvent()`, so it is correctly the bridge's
//! responsibility here.
//!
//! Exit-code contract:
//!   - 0 on success; the success line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to add domain event:` (parity with
//!     `src/commands/add-domain-event.ts:181-184`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_domain_event;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-domain-event.ts:161-178`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub text: String,
    pub timestamp: Option<String>,
    pub bounded_context: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-domain-event` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // add_domain_event::run. Optional fields are omitted when absent so the
    // serde `#[serde(default)]` arms see `None`.
    let mut body = serde_json::Map::new();
    body.insert("workUnitId".to_string(), json!(args.work_unit_id));
    body.insert("text".to_string(), json!(args.text));
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

    match add_domain_event::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns a JSON {success, eventId} payload. Mirror the
            // TS action-callback success line, which embeds the event id.
            let event_id = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| v.get("eventId").and_then(|e| e.as_u64()))
                .unwrap_or(0);
            println!(
                "✓ Added domain event \"{}\" to {} (ID: {event_id})",
                args.text, args.work_unit_id
            );
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add domain event: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
