//! `remove-dependency` shell-facing CLI bridge (RPC-271).
//!
//! Feature: spec/features/remove-dependency-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveDependency` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_dependency::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_dependency::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_dependency::run
//!
//! ## Shorthand + at-least-one guards
//!
//! Two behaviours live ONLY in this bridge (matching where they live in
//! the TS Commander.js action handler at `src/commands/remove-dependency.ts:147-182`):
//!
//! 1. **Shorthand reconciliation**: the second positional argument
//!    `[dependsOnId]` is folded into `--depends-on`. If both are supplied
//!    with DIFFERENT values, exit 1 with the canonical conflict message.
//!    If both are supplied with the same value, succeed without error.
//! 2. **At-least-one guard**: after shorthand reconciliation, if NO
//!    relationship flag is set, exit 1 with the canonical at-least-one
//!    message. The dispatcher tolerates all-empty args (returns
//!    `success:true`) but the CLI surface explicitly rejects them.
//!
//! ## Exit-code contract
//!
//!   - 0 on success; the TS `output.log` message
//!     `✓ Dependency removed successfully` (singular — NOT pluralised
//!     by count, unlike add-dependencies) is rendered on stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to remove dependency:`
//!     (parity with TS chalk-red error path at lines 192-198).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_dependency;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js flag set at
/// `src/commands/remove-dependency.ts:133-145`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Optional second positional argument `[dependsOnId]` — shorthand
    /// for `--depends-on`. Reconciled against `depends_on` before
    /// marshalling.
    pub depends_on_positional: Option<String>,
    pub blocks: Option<String>,
    pub blocked_by: Option<String>,
    pub depends_on: Option<String>,
    pub relates_to: Option<String>,
}

/// Entry point invoked from `main.rs` for the `remove-dependency` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // ── Shorthand reconciliation ── (TS lines 159-170)
    let final_depends_on = match (
        args.depends_on_positional.as_deref(),
        args.depends_on.as_deref(),
    ) {
        (Some(pos), Some(flag)) if pos != flag => {
            eprintln!(
                "✗ Failed to remove dependency: Cannot specify dependency both as argument and --depends-on option"
            );
            return Ok(1);
        }
        (Some(pos), _) => Some(pos.to_string()),
        (None, Some(flag)) => Some(flag.to_string()),
        (None, None) => None,
    };

    // ── At-least-one guard ── (TS lines 173-182)
    if final_depends_on.is_none()
        && args.blocks.is_none()
        && args.blocked_by.is_none()
        && args.relates_to.is_none()
    {
        eprintln!(
            "✗ Failed to remove dependency: Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to"
        );
        return Ok(1);
    }

    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CLI args into the canonical JSON shape consumed by
    // fspec_core::commands::remove_dependency::run.
    let mut obj = serde_json::Map::new();
    obj.insert(
        "workUnitId".into(),
        Value::String(args.work_unit_id.clone()),
    );
    if let Some(v) = args.blocks.as_ref() {
        obj.insert("blocks".into(), Value::String(v.clone()));
    }
    if let Some(v) = args.blocked_by.as_ref() {
        obj.insert("blockedBy".into(), Value::String(v.clone()));
    }
    if let Some(v) = final_depends_on.as_ref() {
        obj.insert("dependsOn".into(), Value::String(v.clone()));
    }
    if let Some(v) = args.relates_to.as_ref() {
        obj.insert("relatesTo".into(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    match remove_dependency::run(&args_json, &project_root).await {
        Ok(_rendered) => {
            println!("✓ Dependency removed successfully");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to remove dependency:', error.message)`.
            // `render_core_error` strips the dispatcher-only
            // `"Invalid args for fspec command remove-dependency: "`
            // envelope so the shell stderr is byte-identical to TS.
            eprintln!("✗ Failed to remove dependency: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
