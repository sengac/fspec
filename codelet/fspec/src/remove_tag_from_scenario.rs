//! `remove-tag-from-scenario` shell-facing CLI bridge (RPC-282).
//!
//! Feature: spec/features/remove-tag-from-scenario-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_tag_from_scenario::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_tag_from_scenario::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/remove-tag-from-scenario.ts:26`).
//! No domain logic in this bridge — JSON marshalling + ✓-prefix printing
//! only.
//!
//! Exit-code contract (parity with TS at
//! `src/commands/remove-tag-from-scenario.ts:199-210`):
//!   - 0 on success; the canonical `message` field returned from core is
//!     prefixed with `✓ ` and written to stdout (idempotent paths also
//!     return success).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the unwrapped
//!     reason is written to stderr prefixed with `Error: ` (the TS
//!     wrapper emits `output.error('Error:', result.error)`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the clap subcommand registered in
/// `main.rs`. The variadic `tags` list is collected by clap as a
/// `Vec<String>` (parity with TS Commander.js `<tags...>` variadic).
#[derive(Debug, Default)]
pub struct CliArgs {
    pub file: String,
    pub scenario_name: String,
    pub tags: Vec<String>,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "file": args.file,
        "scenario": args.scenario_name,
        "tags": args.tags,
    });
    let args_json = body.to_string();

    match commands::remove_tag_from_scenario::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // Core returns JSON {success, valid, message}. Extract
            // `message` and emit `✓ <message>` — parity with TS
            // `output.log(`✓ ${result.message}`)`. Idempotent paths
            // also flow through here.
            let v: Value = serde_json::from_str(&data_json)
                .context("parse core JSON response")?;
            let message = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            println!("✓ {message}");
            Ok(0)
        }
        Err(err) => {
            // Parity with TS `output.error('Error:', result.error)`.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
