//! `update-work-unit` shell-facing CLI bridge (RPC-317).
//!
//! Feature: spec/features/update-work-unit-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module parses the `Mode::UpdateWorkUnit`
//! clap variant (in [`crate::main`]) and delegates to the single
//! source-of-truth in
//! [`codelet_fspec_core::commands::update_work_unit::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Exit-code contract (parity with TS `src/commands/update-work-unit.ts:205-219`):
//!   - 0 on success; prints `✓ Work unit <id> updated successfully` to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; prints
//!     `✗ Failed to update work unit: <reason>` to stderr.
//!
//! Note: the core `run` returns `{ "success": true }` JSON; the human-readable
//! success line is composed here (TS builds it in the CLI action, not the
//! shared `updateWorkUnit` function).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_work_unit;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/update-work-unit.ts:186-204`. The `--type` flag is
/// deliberately omitted (parity: TS does not expose it on the CLI surface).
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub epic: Option<String>,
    pub parent: Option<String>,
}

/// Entry point invoked from `main.rs` for the `update-work-unit` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".to_string(), json!(args.work_unit_id));
    if let Some(t) = &args.title {
        obj.insert("title".to_string(), json!(t));
    }
    if let Some(d) = &args.description {
        obj.insert("description".to_string(), json!(d));
    }
    if let Some(e) = &args.epic {
        obj.insert("epic".to_string(), json!(e));
    }
    if let Some(p) = &args.parent {
        obj.insert("parent".to_string(), json!(p));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match update_work_unit::run(&args_json, &project_root).await {
        Ok(_) => {
            println!("✓ Work unit {} updated successfully", args.work_unit_id);
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to update work unit: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
