//! `compact-work-unit` shell-facing CLI bridge (RPC-206).
//!
//! Feature: spec/features/compact-work-unit-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::CompactWorkUnit` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::compact_work_unit::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::compact_work_unit::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::compact_work_unit::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/compact-work-unit.ts:63`).
//!
//! Exit-code contract:
//!   - 0 on success; the text rendered by the core (either the no-op sentinel
//!     or the removed-items summary) is written verbatim to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to compact work unit:`
//!     (parity with the TS `output.error('✗ Failed to compact work unit:',
//!     error.message)` path at `src/commands/compact-work-unit.ts:196-201`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::compact_work_unit;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/compact-work-unit.ts:155-203`.
///
/// The TS CLI registers ONLY `<workUnitId>` — there is no `--force` option on
/// the Commander surface (the action calls `compactWorkUnit({ workUnitId })`
/// and never forwards a force flag). The core `force` capability is reachable
/// only via the LLM dispatcher front door, so this bridge always passes
/// `force: false`, matching the TS CLI exactly (a non-`done` unit can never be
/// compacted from the shell, and `--force` is rejected as an unknown option by
/// clap, parity with Commander's `error: unknown option '--force'`).
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `compact-work-unit` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "workUnitId": args.work_unit_id,
        "force": false,
    });
    let args_json = body.to_string();

    match compact_work_unit::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to compact work unit: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
