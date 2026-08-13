//! `query-example-mapping-stats` shell-facing CLI bridge (RPC-260).
//!
//! Feature: spec/features/query-example-mapping-stats-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core
//! AND LLM tool call JSON → dispatcher → fspec_core. Both paths call the
//! same [`codelet_fspec_core::commands::query_example_mapping_stats::run`].
//!
//! The TS Commander.js registration at
//! `src/commands/query-example-mapping-stats.ts:162-178` declares ONLY a
//! `--format <format>` flag — the richer dispatcher surface (workUnitId,
//! hasQuestions, questionsFor) is exposed to the LLM agent only. The bridge
//! mirrors the TS CLI surface exactly: format-only and silent on text path.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_example_mapping_stats;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Output format selector: `"text"` (default — silent) or `"json"`.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = match args.format.as_deref() {
        Some(fmt) => json!({ "format": fmt }).to_string(),
        None => json!({}).to_string(),
    };

    match query_example_mapping_stats::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if !rendered.is_empty() {
                println!("{rendered}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
