//! `delete-scenarios` shell-facing CLI bridge (RPC-220).
//!
//! Feature: spec/features/delete-scenarios-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default), marshals the
//! repeatable `--tag` flags + `--dry-run` into JSON, and delegates to the
//! single source-of-truth in
//! [`codelet_fspec_core::commands::delete_scenarios::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Rendering parity with `deleteScenariosByTagCommand` at
//! `src/commands/delete-scenarios-by-tag.ts:275-335`:
//!   - no `--tag` at all → `Error: At least one --tag is required`, exit 1.
//!   - inner success=false → `Error: <error>` to stderr, exit 1.
//!   - dry-run with scenarios → 'Dry run mode - no files modified' +
//!     'Would delete N scenario(s) from M file(s):' + per-file grouped list.
//!   - otherwise → `✓ <message>` (the message begins 'Deleted N scenario(s)
//!     from M file(s)…'), exit 0.

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_scenarios;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/delete-scenarios-by-tag.ts:337-349`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Repeatable `--tag` flags (AND logic).
    pub tags: Vec<String>,
    /// `--dry-run` preview flag.
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `delete-scenarios` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Parity with TS: with NO --tag at all, reject before doing any work.
    if args.tags.is_empty() {
        eprintln!("Error: At least one --tag is required");
        return Ok(1);
    }

    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "tags": args.tags,
        "dryRun": args.dry_run,
    })
    .to_string();

    let json_text = match delete_scenarios::run(&args_json, &project_root).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    };

    let value: Value =
        serde_json::from_str(&json_text).context("parse core response as JSON")?;

    let success = value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !success {
        let err = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        eprintln!("Error: {err}");
        return Ok(1);
    }

    let deleted_count = value
        .get("deletedCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let file_count = value.get("fileCount").and_then(Value::as_u64).unwrap_or(0);
    let message = value.get("message").and_then(Value::as_str).unwrap_or("");

    let scenarios = value.get("scenarios").and_then(Value::as_array);

    if args.dry_run && scenarios.is_some() {
        // Parity with TS dry-run branch (lines 301-325).
        println!("Dry run mode - no files modified");
        println!(
            "\nWould delete {deleted_count} scenario(s) from {file_count} file(s):\n"
        );

        // Group scenarios by file, preserving first-seen file order.
        let mut order: Vec<String> = Vec::new();
        let mut by_file: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for s in scenarios.unwrap_or(&Vec::new()) {
            let file = s.get("file").and_then(Value::as_str).unwrap_or("").to_string();
            let name = s.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let tags: Vec<String> = s
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if !by_file.contains_key(&file) {
                order.push(file.clone());
            }
            by_file
                .entry(file)
                .or_default()
                .push((name, tags.join(" ")));
        }

        for file in &order {
            println!("\n{file}:");
            if let Some(entries) = by_file.get(file) {
                for (name, tags) in entries {
                    println!("  - {name} ({tags})");
                }
            }
        }
    } else {
        // Parity with TS else branch (line 327): `✓ ${result.message}`.
        println!("✓ {message}");
    }

    Ok(0)
}
