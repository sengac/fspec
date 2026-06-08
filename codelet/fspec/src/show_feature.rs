//! `show-feature` shell-facing CLI bridge (RPC-304).
//!
//! Feature: spec/features/show-feature-cli-subcommand.feature

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_feature;
use serde::Deserialize;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature: String,
    pub format: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StructuredOutcome {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

/// Entry point for the `show-feature` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut base: Map<String, Value> = Map::new();
    base.insert("feature".to_string(), Value::String(args.feature.clone()));
    if let Some(fmt) = args.format.as_deref() {
        base.insert("format".to_string(), Value::String(fmt.to_string()));
    }
    if let Some(out) = args.output.as_deref() {
        base.insert("output".to_string(), Value::String(out.to_string()));
    }

    // Probe with JSON format so we can inspect success/error.
    let mut json_args = base.clone();
    json_args.insert("format".to_string(), Value::String("json".to_string()));
    json_args.remove("output");
    let json_args_str = json!(json_args).to_string();
    let json_payload = show_feature::run(&json_args_str, &project_root).await?;

    let outcome: StructuredOutcome = serde_json::from_str(&json_payload)
        .context("parse show-feature JSON payload")?;

    if !outcome.success {
        let msg = outcome
            .error
            .unwrap_or_else(|| "unknown show-feature error".to_string());
        eprintln!("Error: {msg}");
        return Ok(1);
    }

    // Success path: render with the user-requested format.
    let final_args_str = json!(base).to_string();
    match show_feature::run(&final_args_str, &project_root).await {
        Ok(rendered) => {
            if args.output.is_none() {
                print!("{rendered}");
                if !rendered.ends_with('\n') {
                    println!();
                }
            } else if let Some(p) = args.output.as_deref() {
                println!("\u{2713} Feature content written to {p}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
