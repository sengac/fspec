//! `remove-tag-from-feature` shell-facing CLI bridge (RPC-281).
//!
//! Feature: spec/features/remove-tag-from-feature-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveTagFromFeature` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_tag_from_feature::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_tag_from_feature::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_tag_from_feature::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/remove-tag-from-feature.ts:25`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the unwrapped
//!     reason is written to stderr prefixed with `Error:` (parity with the
//!     TS `output.error('Error:', error.message)` path collapsed via
//!     [`crate::common::render_core_error`]).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_tag_from_feature;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/remove-tag-from-feature.ts:140-148`.
#[derive(Debug)]
pub struct CliArgs {
    pub file: String,
    pub tags: Vec<String>,
}

/// Entry point invoked from `main.rs` for the `remove-tag-from-feature` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "file": args.file,
        "tags": args.tags,
    });
    let args_json = body.to_string();

    match remove_tag_from_feature::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // Core returns a JSON envelope: { success, valid, message }.
            // The CLI mirrors the TS `output.log('✓ ' + result.message)`
            // line — no reminder branch (this command emits none).
            let parsed: Value =
                serde_json::from_str(&data_json).context("parse core response as JSON")?;
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                println!("✓ {msg}");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('Error:', error.message)` at
            // src/commands/remove-tag-from-feature.ts:128.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
