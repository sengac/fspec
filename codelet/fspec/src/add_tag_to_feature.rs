//! `add-tag-to-feature` shell-facing CLI bridge (RPC-193).
//!
//! Feature: spec/features/add-tag-to-feature-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddTagToFeature` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_tag_to_feature::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_tag_to_feature::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_tag_to_feature::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-tag-to-feature.ts:34`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the unwrapped
//!     reason is written to stderr prefixed with `Error:` (parity with
//!     the TS `output.error('Error:', error.message)` path collapsed
//!     via [`crate::common::render_core_error`]).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_tag_to_feature;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-tag-to-feature.ts:329-345`.
#[derive(Debug)]
pub struct CliArgs {
    pub file: String,
    pub tags: Vec<String>,
    pub validate_registry: bool,
}

/// Entry point invoked from `main.rs` for the `add-tag-to-feature` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core. Both the
    // dispatcher and the CLI feed the SAME serde shape, so any future
    // field added to `CliArgs` automatically threads through to the
    // core without duplication.
    let body = json!({
        "file": args.file,
        "tags": args.tags,
        "validateRegistry": args.validate_registry,
    });
    let args_json = body.to_string();

    match add_tag_to_feature::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns a JSON envelope: { success, valid, message,
            // systemReminder?, systemReminders? }. The CLI surface mirrors
            // the TS `output.log('✓ ' + result.message)` line and, when
            // present, the consolidated reminder block.
            let parsed: Value = serde_json::from_str(&data_json)
                .context("parse core response as JSON")?;
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                println!("✓ {msg}");
            }
            if let Some(rem) = parsed.get("systemReminder").and_then(|v| v.as_str()) {
                println!("\n{rem}");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('Error:', error.message)` at
            // src/commands/add-tag-to-feature.ts:311.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
