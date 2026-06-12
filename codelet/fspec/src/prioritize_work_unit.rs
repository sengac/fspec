//! `prioritize-work-unit` shell-facing CLI bridge (RPC-255).
//!
//! Feature: spec/features/prioritize-work-unit-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::PrioritizeWorkUnit` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::prioritize_work_unit::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::prioritize_work_unit::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::prioritize_work_unit::run
//!
//! Bridge scope: parse the `--position` string into the `'top' | 'bottom' |
//! number` polymorphic value (mirroring the TS `parseInt`-based coercion at
//! `src/commands/prioritize-work-unit.ts:147-154`) and marshal JSON. All
//! domain logic (existence, done guard, cross-column guard, data-integrity
//! check, reordering, disk write) lives in the core.
//!
//! Exit-code contract:
//!   - 0 on success; `✓ Work unit <id> prioritized successfully` on stdout.
//!   - 1 on any error; `✗ Failed to prioritize work unit: <message>` on
//!     stderr (parity with the TS `output.error('✗ Failed to prioritize
//!     work unit:', err.message)` path at
//!     `src/commands/prioritize-work-unit.ts:165-168`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::prioritize_work_unit;
use codelet_fspec_core::js_compat::parse_js_int;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/prioritize-work-unit.ts:133-140`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub position: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Entry point invoked from `main.rs` for the `prioritize-work-unit` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut body = Map::new();
    body.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );

    // Position coercion (TS parity: 'top'/'bottom' literal, else parseInt).
    // The numeric branch uses `parse_js_int` to reproduce JS
    // `parseInt(value, 10)` EXACTLY: leading-digit prefixes coerce (`12abc`
    // → 12, `2x` → 2, `3.7` → 3, `0x10` → 0) and a non-numeric value becomes
    // JSON `null` (matching `JSON.stringify(NaN)`), which the core treats as
    // index 0. A strict `i64` parse would reject every trailing-text form and
    // silently collapse it to "top", diverging from the TS CLI.
    if let Some(pos) = &args.position {
        let value = match pos.as_str() {
            "top" => Value::String("top".to_string()),
            "bottom" => Value::String("bottom".to_string()),
            other => parse_js_int(other),
        };
        body.insert("position".to_string(), value);
    }
    if let Some(before) = &args.before {
        body.insert("before".to_string(), Value::String(before.clone()));
    }
    if let Some(after) = &args.after {
        body.insert("after".to_string(), Value::String(after.clone()));
    }

    let args_json = json!(body).to_string();

    match prioritize_work_unit::run(&args_json, &project_root).await {
        Ok(_data) => {
            println!("✓ Work unit {} prioritized successfully", args.work_unit_id);
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to prioritize work unit: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
