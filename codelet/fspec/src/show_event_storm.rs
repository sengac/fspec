//! `show-event-storm` shell-facing CLI bridge (RPC-303).
//!
//! Feature: spec/features/show-event-storm-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core
//! AND LLM tool call JSON → dispatcher → fspec_core. Both paths call the
//! same [`codelet_fspec_core::commands::show_event_storm::run`].
//!
//! The TS Commander.js registration at
//! `src/commands/show-event-storm.ts:107-116` declares one required
//! positional `<work-unit-id>` and no flags. The CLI prints the JSON array
//! returned by fspec_core verbatim to stdout, or an `Error:` line plus
//! exit 1 on any failure.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::FspecCoreError;
use codelet_fspec_core::commands::show_event_storm;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the work-unit identifier to query.
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "workUnitId": args.work_unit_id }).to_string();

    match show_event_storm::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Strip the InvalidArgs wrapper so the printed stderr matches
            // the bare TS Error.message string.
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("Error: {reason}");
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
