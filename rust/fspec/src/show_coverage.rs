//! `show-coverage` shell-facing CLI bridge (RPC-300).
//!
//! Feature: spec/features/show-coverage-cli-subcommand.feature
//!
//! This bridge does NOT contain any coverage parsing, stats aggregation, or
//! markdown rendering — that all lives in
//! `codelet_fspec_core::commands::show_coverage::run` so both invocation
//! paths (LLM dispatcher and clap CLI) share a single implementation.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_coverage;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature_name: Option<String>,
    pub format: Option<String>,
    pub output: Option<String>,
}

/// Entry point for the `show-coverage` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    if let Some(name) = args.feature_name.as_deref() {
        payload.insert("featureName".to_string(), Value::String(name.to_string()));
    }
    if let Some(fmt) = args.format.as_deref() {
        payload.insert("format".to_string(), Value::String(fmt.to_string()));
    }
    if let Some(out) = args.output.as_deref() {
        payload.insert("output".to_string(), Value::String(out.to_string()));
    }

    let args_str = json!(payload).to_string();
    match show_coverage::run(&args_str, &project_root).await {
        Ok(rendered) => {
            if args.output.is_none() {
                // TS parity: `console.log(rendered)` always appends a
                // trailing newline to whatever string is passed in,
                // regardless of whether it already ends with one.
                println!("{rendered}");
            } else if let Some(p) = args.output.as_deref() {
                println!("\u{2713} Coverage report written to {p}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
