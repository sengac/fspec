//! `query-estimation-guide` shell-facing CLI bridge (RPC-259).
//!
//! Feature: spec/features/query-estimation-guide-cli-subcommand.feature
//!
//! Two-front-doors pattern: both shell argv and LLM tool call paths
//! converge on fspec_core::commands::query_estimation_guide::run.
//!
//! TS-parity silent-text contract: when `--format` is not `"json"`,
//! fspec_core returns the empty string and the bridge prints nothing.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_estimation_guide;
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional argument; consumed but not used by the core
    /// computation (TS parity).
    pub work_unit_id: String,
    /// `--format <format>`: `"text"` (default — silent) or `"json"`.
    pub format: Option<String>,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    obj.insert(
        "workUnitId".into(),
        Value::String(args.work_unit_id.clone()),
    );
    if let Some(v) = args.format.as_ref() {
        obj.insert("format".into(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    match query_estimation_guide::run(&args_json, &project_root).await {
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
