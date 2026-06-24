//! `validate-tags` shell-facing CLI bridge (RPC-324).
//!
//! Feature: spec/features/validate-tags-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ValidateTags` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::validate_tags`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::validate_tags::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::validate_tags::run
//!
//! ## NO inline validation or rendering logic
//! Validation lives in `validate_tags::run`; the failures-only/--verbose/
//! --summary rendering lives in `validate_tags::render_cli_output`. This
//! bridge only marshals argv → JSON, calls those two functions, prints the
//! rendered text, and maps `invalidCount > 0` to exit code 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_tags;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/validate-tags.ts:99-121`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Optional single feature file to validate (validates all if absent).
    pub file: Option<String>,
    /// `--verbose` — print one passing line per valid file.
    pub verbose: bool,
    /// `--summary` — print only the summary count lines (overrides verbose).
    pub summary: bool,
}

/// Entry point invoked from `main.rs` for the `validate-tags` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    if let Some(f) = &args.file {
        obj.insert("file".to_string(), json!(f));
    }
    let args_json = json!(obj).to_string();

    let json_text = match validate_tags::run(&args_json, &project_root).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(2);
        }
    };

    let value: Value = serde_json::from_str(&json_text).context("parse core response as JSON")?;

    let rendered = validate_tags::render_cli_output(&value, args.verbose, args.summary);
    if !rendered.is_empty() {
        println!("{rendered}");
    }

    let invalid_count = value
        .get("invalidCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if invalid_count > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}
