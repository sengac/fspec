//! `repair-work-units` shell-facing CLI bridge (RPC-284).
//!
//! Feature: spec/features/repair-work-units-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RepairWorkUnits` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::repair_work_units::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::repair_work_units::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::repair_work_units::run
//!
//! Bridge scope: marshal `{dryRun?}` JSON and render the success/error
//! lines. The `--dry-run` flag is forwarded but has NO effect (the core
//! always writes — preserving the TS parity bug). The repair logic itself
//! (state-index rebuild, bidirectional-link repair, disk write) lives in
//! the core.
//!
//! Exit-code contract:
//!   - 0 on success; `✓ Repaired <n> issues` on stdout, where `<n>` is the
//!     `repaired` count returned by the core (TS
//!     `output.log('✓ Repaired ${result.repaired} issues')` at
//!     `src/commands/repair-work-units.ts:138`).
//!   - 1 on any error; `✗ Failed to repair work units: <message>` on stderr
//!     (parity with the TS catch arm at
//!     `src/commands/repair-work-units.ts:144-149`).
//!
//! The buggy TS `result.details` loop is intentionally omitted — `details`
//! is never set by the implementation, so it is always `undefined`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::repair_work_units;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/repair-work-units.ts:128-133`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `repair-work-units` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut body = Map::new();
    if args.dry_run {
        body.insert("dryRun".to_string(), Value::Bool(true));
    }
    let args_json = json!(body).to_string();

    match repair_work_units::run(&args_json, &project_root).await {
        Ok(data) => {
            // The core returns { success, repairs, repaired }; extract the
            // count for the canonical success line. A malformed payload
            // defaults to 0 (the line is informational only).
            let repaired = serde_json::from_str::<Value>(&data)
                .ok()
                .and_then(|v| v.get("repaired").and_then(Value::as_u64))
                .unwrap_or(0);
            println!("✓ Repaired {repaired} issues");
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to repair work units: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
