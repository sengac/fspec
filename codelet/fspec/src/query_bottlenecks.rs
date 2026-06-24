//! `query-bottlenecks` shell-facing CLI bridge (RPC-256).
//!
//! Feature: spec/features/query-bottlenecks-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::query_bottlenecks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → same function
//!
//! Bridge stays thin — no filter/DFS/rendering logic. All bottleneck
//! detection and text formatting lives in fspec_core. A two-front-doors
//! parity test enforces this by string-grepping forbidden tokens.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_bottlenecks;
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--output <format>`: `"text"` (default) or `"json"`.
    pub output: Option<String>,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    if let Some(v) = args.output.as_ref() {
        obj.insert("output".into(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    match query_bottlenecks::run(&args_json, &project_root).await {
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
