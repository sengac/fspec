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
//! Console-output contract (parity with TS `logger`):
//!   The TS `.action()` callback uses `logger.success(...)` on success and
//!   `logger.error(...)` on failure. The fspec `logger` is a Winston logger
//!   whose ONLY transport is a file (`~/.fspec/fspec.log`) — it writes
//!   NOTHING to stdout/stderr. Moreover Winston has no `success` level, so
//!   `logger.success(...)` throws a `TypeError` that is swallowed by the
//!   surrounding `try/catch`, which then calls `logger.error(...)` (also
//!   file-only). Net observable behaviour of `node dist/index.js
//!   add-aggregate ...`:
//!     - stdout: EMPTY in every case
//!     - stderr: EMPTY in every case
//!     - exit code: ALWAYS 1 (success path falls through the caught
//!       `TypeError` into `process.exit(1)`; error path is an explicit
//!       `process.exit(1)`)
//!     - disk state: the aggregate IS persisted on the success path before
//!       the (caught) `logger.success` throw, so `eventStorm.items` is
//!       written exactly as `addAggregate()` mutated it.
//!   This bridge therefore emits NO console output and ALWAYS returns exit
//!   code 1, matching the TS binary byte-for-byte. The core `run()` still
//!   performs the real mutation + atomic write, so disk parity holds.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_aggregate;
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
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core. Only present
    // optionals are included so serde's `Option` defaults stay `None`.
    let mut body = json!({
        "workUnitId": args.work_unit_id.clone(),
        "text": args.text.clone(),
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

    // The core performs the real mutation + atomic write. Regardless of
    // success or failure, the TS binary writes nothing to the console and
    // always exits 1 (see the module-level Console-output contract): the
    // `logger.success(...)` call on the success path throws a swallowed
    // `TypeError` (Winston has no `success` level) and falls through to
    // `process.exit(1)`, while the error path is an explicit
    // `process.exit(1)`. Both `logger.*` transports are file-only. We
    // discard the result and emit no console output for byte-parity.
    let _ = add_aggregate::run(&args_json, &project_root).await;
    Ok(1)
}
