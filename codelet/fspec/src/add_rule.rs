//! `add-rule` shell-facing CLI bridge (RPC-189).
//!
//! Feature: spec/features/add-rule-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddRule` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_rule::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_rule::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_rule::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-rule.ts:21`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('✗ Failed to add rule:', error.message)` path
//!     collapsed via [`crate::common::render_core_error`]).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_rule;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-rule.ts:76-92`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub rule: String,
}

/// Entry point invoked from `main.rs` for the `add-rule` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // add_rule::run. The dispatcher and CLI both feed the SAME serde shape,
    // so adding a field to `CliArgs` automatically threads through to
    // `args_json` without duplication.
    let body = json!({
        "workUnitId": args.work_unit_id,
        "rule": args.rule,
    });
    let args_json = body.to_string();

    match add_rule::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            // The core returns a JSON {success, ruleCount} payload — the
            // CLI surface mirrors the TS `output.log('✓ Rule added
            // successfully')` line instead of dumping JSON to stdout.
            println!("✓ Rule added successfully");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to add rule:', error.message)`
            // at src/commands/add-rule.ts:89.
            eprintln!("✗ Failed to add rule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
