//! `delete-features` shell-facing CLI bridge (RPC-218; the registered
//! command name is `delete-features`).
//!
//! Feature: spec/features/delete-features-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::DeleteFeatures` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::delete_features::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::delete_features::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::delete_features::run
//!
//! The core returns the inner envelope `{success, deletedCount, message?,
//! files?, error?}`. This bridge owns ALL rendering decisions (parity with
//! the TS `deleteFeaturesByTagCommand` at
//! `src/commands/delete-features-by-tag.ts:127-176`):
//!   - inner success=false → `Error: <error>` to stderr, exit 1
//!   - dry-run with files  → 'Dry run mode - no files modified' + would-delete list
//!   - real delete w/ files → `✓ <message>` + 'Deleted files:' list
//!   - empty (no files / no match) → bare message line, exit 0

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_features;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/delete-features-by-tag.ts:178-190`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Repeatable `--tag` flags (AND logic).
    pub tags: Vec<String>,
    /// `--dry-run` preview flag.
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `delete-features` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "tags": args.tags,
        "dryRun": args.dry_run,
    })
    .to_string();

    let json_text = match delete_features::run(&args_json, &project_root).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    };

    let value: Value = serde_json::from_str(&json_text).context("parse core response as JSON")?;

    let success = value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // ---- Recoverable failure (inner envelope) ----
    if !success {
        let err = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        eprintln!("Error: {err}");
        return Ok(1);
    }

    let message = value.get("message").and_then(Value::as_str).unwrap_or("");
    let files: Vec<&str> = value
        .get("files")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if args.dry_run && !files.is_empty() {
        // Parity with TS dry-run branch (lines 152-160).
        let count = files.len();
        println!("Dry run mode - no files modified");
        println!("\nWould delete {count} feature file(s):\n");
        for file in &files {
            println!("  - {file}");
        }
    } else if !files.is_empty() {
        // Parity with TS real-delete branch (lines 161-166).
        println!("✓ {message}");
        println!("\nDeleted files:");
        for file in &files {
            println!("  - {file}");
        }
    } else {
        // Parity with TS empty branch (lines 167-169).
        println!("{message}");
    }

    Ok(0)
}
