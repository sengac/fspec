//! `query-orphans` shell-facing CLI bridge (RPC-262).
//!
//! Feature: spec/features/query-orphans-cli-subcommand.feature
//!
//! Two-front-doors pattern: both shell argv and LLM tool call paths
//! converge on fspec_core::commands::query_orphans::run.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_orphans;
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--output <format>`: `"text"` (default) or `"json"`.
    pub output: Option<String>,
    /// `--exclude-done`: filter out units in done status.
    pub exclude_done: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    if let Some(v) = args.output.as_ref() {
        obj.insert("output".into(), Value::String(v.clone()));
    }
    if args.exclude_done {
        obj.insert("excludeDone".into(), Value::Bool(true));
    }
    let args_json = json!(obj).to_string();

    match query_orphans::run(&args_json, &project_root).await {
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
