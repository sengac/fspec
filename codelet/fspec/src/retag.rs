//! `retag` shell-facing CLI bridge (RPC-293).
//!
//! Feature: spec/features/retag-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core run.
//! The bridge is JSON marshalling + stdout/stderr rendering only; all domain
//! logic (enumerate features, whole-token tag replace, Gherkin re-parse,
//! file-write) lives in [`codelet_fspec_core::commands::retag::run`] — the
//! SAME function the LLM-facing agent_loop dispatcher invokes.
//!
//! The core returns the inner envelope `{success, fileCount, occurrenceCount,
//! message?, files?, error?}`. This bridge owns ALL rendering decisions
//! (parity with the TS `retagCommand` at `src/commands/retag.ts:169-212`):
//!   * inner success=false → `Error: <error>` to stderr, exit 1
//!   * dry-run with files  → 'Dry run mode - no files modified' + would-rename list
//!   * real rename w/ files → `✓ <message>` + 'Modified files:' list
//!   * empty (no files)     → bare message line, exit 0

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::retag;
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub struct CliArgs {
    pub from: Option<String>,
    pub to: Option<String>,
    pub dry_run: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let from = args.from.unwrap_or_default();
    let to = args.to.unwrap_or_default();

    let args_json = json!({
        "from": from,
        "to": to,
        "dryRun": args.dry_run,
    })
    .to_string();

    let json_text = match retag::run(&args_json, &project_root).await {
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
    let file_count = value.get("fileCount").and_then(Value::as_u64).unwrap_or(0);
    let occurrence_count = value
        .get("occurrenceCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let files: Vec<&str> = value
        .get("files")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if args.dry_run && !files.is_empty() {
        // Parity with TS dry-run branch (lines 186-196).
        println!("Dry run mode - no files modified");
        println!(
            "\nWould rename {from} to {to} in {file_count} file(s) ({occurrence_count} occurrence(s)):\n"
        );
        for file in &files {
            println!("  - {file}");
        }
    } else if !files.is_empty() {
        // Parity with TS real-rename branch (lines 197-202).
        println!("✓ {message}");
        println!("\nModified files:");
        for file in &files {
            println!("  - {file}");
        }
    } else {
        // Parity with TS empty branch (lines 203-205).
        println!("{message}");
    }

    Ok(0)
}
