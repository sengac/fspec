//! `generate-scenarios` shell-facing CLI bridge (RPC-234).
//!
//! Feature: spec/features/generate-scenarios-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::generate_scenarios::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::generate_scenarios::run
//!
//! This module performs ZERO domain logic — no example-mapping analysis, no
//! duplicate detection, no rendering. Its only computation is marshalling the
//! clap flags into the JSON args shape that the single source-of-truth
//! [`codelet_fspec_core::commands::generate_scenarios::run`] consumes. (The
//! `cli_and_dispatcher_converge` contract test asserts this file embeds none
//! of the core's rendering literals.)
//!
//! The TS Commander surface (`src/commands/generate-scenarios.ts:630-672`)
//! exposes exactly: a positional `<workUnitId>`, `--feature <name>`, and
//! `--ignore-possible-duplicates`.
//!
//! Exit-code contract (parity with the TS `.action` wrapper,
//! `generate-scenarios.ts:648-670`): the TS action prints the creation lines +
//! consolidated reminder to STDOUT on success and `process.exit`es 0; on ANY
//! thrown error it prints `✗ Failed to generate scenarios: <message>` to
//! STDERR and exits 1. The Rust core returns the rendered success body as `Ok`
//! and every gate failure as an [`FspecCoreError`]; this bridge mirrors the
//! exit-code behaviour exactly.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_scenarios;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/generate-scenarios.ts:630-672`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Positional `<workUnitId>`.
    pub work_unit_id: String,
    /// `--feature <name>`.
    pub feature: Option<String>,
    /// `--ignore-possible-duplicates`.
    pub ignore_possible_duplicates: bool,
}

/// Entry point invoked from `main.rs` for the `generate-scenarios` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal the flags into the JSON args shape. Booleans/options are only
    // emitted when set so the core's `#[serde(default)]` arms cover the rest.
    let mut obj = serde_json::Map::new();
    obj.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );
    if let Some(feature) = &args.feature {
        obj.insert("feature".to_string(), Value::String(feature.clone()));
    }
    if args.ignore_possible_duplicates {
        obj.insert(
            "ignorePossibleDuplicates".to_string(),
            Value::Bool(true),
        );
    }
    let args_json = json!(obj).to_string();

    match generate_scenarios::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        // Every failure (soft gate or hard error) is printed to STDERR with the
        // TS failure prefix and exits 1. The core carries the rendered body in
        // the `Message` variant; other variants render through the shared
        // helper.
        Err(FspecCoreError::Message(body)) => {
            eprintln!("✗ Failed to generate scenarios: {body}");
            Ok(1)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to generate scenarios: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
