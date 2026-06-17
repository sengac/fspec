//! `workflow-automation` shell-facing CLI bridge (RPC-326).
//!
//! Feature: spec/features/workflow-automation-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::workflow_automation::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::workflow_automation::run
//!
//! Unlike `auto-advance`, the TS Commander shell binds correctly: it passes
//! the positional `<action>` + `<work-unit-id>` and the `--event` /
//! `--from-state` flags straight into `workflowAutomation(...)`
//! (`src/commands/workflow-automation.ts:199-218`). This Rust bridge mirrors
//! that: it marshals those four values into the JSON args shape and delegates
//! to the single source-of-truth core function. It performs ZERO domain logic.
//!
//! Exit-code contract (parity with the TS Commander action, which passes
//! `workflowAutomation` directly to `.action(...)` with NO try/catch — so a
//! thrown error becomes a Node *uncaught exception* crash dump whose payload
//! line reads `Error: <message>`):
//!   - 0 on success; the shell prints nothing.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the bare reason is
//!     written to stderr prefixed with `Error:` — mirroring the `Error:
//!     <message>` line of the TS Node crash dump (the surrounding
//!     environment-specific stack trace is not reproducible and is not part of
//!     the behavioural contract).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::workflow_automation;
use codelet_fspec_core::error::FspecCoreError;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/workflow-automation.ts:199-218`: required positionals
/// `<action> <work-unit-id>` plus optional `--event` / `--from-state` flags.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub action: String,
    pub work_unit_id: String,
    pub event: Option<String>,
    pub from_state: Option<String>,
}

/// Entry point invoked from `main.rs` for the `workflow-automation` clap
/// subcommand. Returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Pure JSON marshalling: forward action + id + the optional flags exactly
    // as the TS Commander shell does. Only flags that were supplied are
    // included in the payload.
    let mut body = json!({
        "action": args.action,
        "workUnitId": args.work_unit_id,
    });
    if let Some(event) = &args.event {
        body["event"] = json!(event);
    }
    if let Some(from_state) = &args.from_state {
        body["fromState"] = json!(from_state);
    }
    let args_json = body.to_string();

    match workflow_automation::run(&args_json, &project_root).await {
        Ok(_data_json) => Ok(0),
        Err(err) => {
            // Parity with the TS uncaught-crash payload line: a `JSON.parse`
            // failure throws a `SyntaxError` (uncaught → crash dump line
            // `SyntaxError: <msg>`), whereas every other thrown `Error`
            // surfaces as `Error: <msg>`. RPC-334: the core now marks parse
            // failures with the dedicated `JsonSyntax` variant (body = serde
            // caret snippet, no fabricated `Unexpected token in JSON:` prefix),
            // so we route on the variant rather than sniffing the message text.
            // The `SyntaxError:` stream prefix is the parity-relevant part we
            // keep; the body is a deliberate, documented serde-vs-V8 divergence.
            match &err {
                FspecCoreError::JsonSyntax(reason) => eprintln!("SyntaxError: {reason}"),
                _ => eprintln!("Error: {}", render_core_error(&err)),
            }
            Ok(1)
        }
    }
}
