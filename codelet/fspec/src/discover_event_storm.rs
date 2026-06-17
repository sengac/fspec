//! `discover-event-storm` shell-facing CLI bridge (RPC-225).
//!
//! Feature: spec/features/discover-event-storm-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core
//! AND LLM tool call JSON → dispatcher → fspec_core. Both paths call the
//! same [`codelet_fspec_core::commands::discover_event_storm::run`].
//!
//! The TS Commander.js registration at
//! `src/commands/discover-event-storm.ts:88-98` declares one required
//! positional `<work-unit-id>` and no flags. On success the core returns the
//! green confirmation line plus the `<system-reminder>` guidance block, which
//! this bridge prints verbatim to stdout. On any failure it prints an
//! `Error:` line to stderr and exits 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::FspecCoreError;
use codelet_fspec_core::commands::discover_event_storm;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the work-unit identifier to start discovery on.
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "workUnitId": args.work_unit_id }).to_string();

    match discover_event_storm::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // TS uses `output.error('✗ ...')` for every failure path, which
            // prints a bare `✗`-prefixed line to stderr (no `Error:` prefix).
            // Mirror that exactly (src/commands/discover-event-storm.ts:34-63).
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ {reason}");
                }
                _ => {
                    eprintln!("✗ {err}");
                }
            }
            Ok(1)
        }
    }
}
