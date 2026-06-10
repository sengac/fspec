//! `show-acceptance-criteria` shell-facing CLI bridge (RPC-299).
//!
//! Feature: spec/features/show-acceptance-criteria-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::show_acceptance_criteria::run`, then:
//!   * on success: print the envelope `message`; if no `--output` flag,
//!     also print the rendered body.
//!   * on structured failure (`success=false` inside envelope): print the
//!     envelope `error` to stderr and exit 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_acceptance_criteria;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub tags: Vec<String>,
    pub format: Option<String>,
    pub output: Option<String>,
}

/// Entry point for the `show-acceptance-criteria` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    if !args.tags.is_empty() {
        payload.insert(
            "tags".to_string(),
            Value::Array(args.tags.iter().map(|t| Value::String(t.clone())).collect()),
        );
    }
    if let Some(fmt) = args.format.as_deref() {
        payload.insert("format".to_string(), Value::String(fmt.to_string()));
    }
    if let Some(out) = args.output.as_deref() {
        payload.insert("output".to_string(), Value::String(out.to_string()));
    }
    let args_json = json!(payload).to_string();

    match show_acceptance_criteria::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let envelope: Value = serde_json::from_str(&rendered)
                .context("parse fspec_core envelope")?;
            let ok = envelope
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = envelope
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                eprintln!("Error: {err}");
                return Ok(1);
            }
            let msg = envelope
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("{msg}");
            if args.output.is_none() {
                let body = envelope
                    .get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("{body}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
