//! `show-foundation-event-storm` shell-facing CLI bridge (RPC-306).
//!
//! Feature: spec/features/show-foundation-event-storm-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::show_foundation_event_storm::run`, then print
//! the `data` array verbatim as 2-space-indented JSON to stdout.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_foundation_event_storm;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub r#type: Option<String>,
    pub context: Option<String>,
}

/// Entry point for the `show-foundation-event-storm` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    if let Some(t) = args.r#type.as_deref() {
        payload.insert("type".to_string(), Value::String(t.to_string()));
    }
    if let Some(c) = args.context.as_deref() {
        payload.insert("context".to_string(), Value::String(c.to_string()));
    }
    let args_json = json!(payload).to_string();

    match show_foundation_event_storm::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Re-pretty-print the `data` field as the top-level stdout body.
            let envelope: Value = serde_json::from_str(&rendered)
                .context("parse fspec_core JSON envelope")?;
            let data = envelope.get("data").cloned().unwrap_or(Value::Array(vec![]));
            let body = serde_json::to_string_pretty(&data)
                .context("re-render data array")?;
            println!("{body}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
