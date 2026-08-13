//! `remove-init-files` shell-facing CLI bridge (RPC-276).
//!
//! Feature: spec/features/remove-init-files-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_init_files::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_init_files::run
//!
//! This module performs NO agent detection or file deletion: it only marshals
//! the parsed clap arguments into the JSON arg shape, calls the single
//! fspec-core entry point, and prints the returned `{filesRemoved}` summary.
//! All detection and deletion lives in fspec-core.
//!
//! The clap surface exposes `--keep-config` / `--no-keep-config` (boolean).
//! The headless Rust port does NOT render the interactive Ink prompt; an
//! unspecified keepConfig defaults to false (remove config) inside the core.
//!
//! Exit-code contract:
//!   - 0 on success; `✓ Successfully removed fspec init files` plus one
//!     `  - <path>` line per removed file are written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed `✗ Failed to remove init files:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_init_files;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/remove-init-files.ts:166-177`): the boolean keepConfig
/// surfaced as `--keep-config` / `--no-keep-config`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `Some(true)` → `--keep-config`; `Some(false)` → `--no-keep-config`;
    /// `None` → unspecified (core defaults to false / remove config).
    pub keep_config: Option<bool>,
}

/// Entry point invoked from `main.rs` for the `remove-init-files` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    if let Some(keep) = args.keep_config {
        payload.insert("keepConfig".to_string(), Value::Bool(keep));
    }
    let args_json = json!(payload).to_string();

    match remove_init_files::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let value: Value =
                serde_json::from_str(&rendered).context("parse remove-init-files payload")?;
            println!("✓ Successfully removed fspec init files");
            if let Some(files) = value.get("filesRemoved").and_then(Value::as_array) {
                for f in files {
                    if let Some(path) = f.as_str() {
                        println!("  - {path}");
                    }
                }
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to remove init files: {err}");
            Ok(1)
        }
    }
}
