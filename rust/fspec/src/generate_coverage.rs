//! `generate-coverage` shell-facing CLI bridge (RPC-231).
//!
//! Feature: spec/features/generate-coverage-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::generate_coverage::run`, then print the rendered
//! report (already including the link-coverage guidance block) verbatim on
//! stdout. All scanning and rendering logic lives in fspec_core.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_coverage;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub dry_run: bool,
}

/// Entry point for the `generate-coverage` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert("dryRun".to_string(), Value::Bool(args.dry_run));
    let args_json = json!(payload).to_string();

    match generate_coverage::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns the full report string; print it verbatim.
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ Error: {reason}");
                }
                _ => {
                    eprintln!("✗ Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
