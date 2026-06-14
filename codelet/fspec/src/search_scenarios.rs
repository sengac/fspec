//! `search-scenarios` shell-facing CLI bridge (RPC-297).
//!
//! Feature: spec/features/search-scenarios-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::search_scenarios::run`, then:
//!   * `--json` → print the rendered JSON envelope verbatim
//!   * default → print the `message` summary line emitted by the core
//!
//! This bridge contains NO inline matching, gherkin parsing, or summary
//! formatting — all of that lives exclusively in fspec_core.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::search_scenarios;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub query: String,
    pub regex: bool,
    pub json: bool,
}

/// Entry point for the `search-scenarios` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert("query".to_string(), Value::String(args.query.clone()));
    payload.insert("regex".to_string(), Value::Bool(args.regex));
    payload.insert("json".to_string(), Value::Bool(args.json));
    let args_json = json!(payload).to_string();

    match search_scenarios::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if args.json {
                println!("{rendered}");
            } else {
                // The core emits a `message` field on the default (table)
                // path; the bridge surfaces it without re-deriving any counts.
                let envelope: Value =
                    serde_json::from_str(&rendered).context("parse fspec_core envelope")?;
                let msg = envelope
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                println!("{msg}");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗ Search failed:', error.message)`
            // path. The dispatcher wraps domain errors (e.g. invalid regex)
            // in `FspecCoreError::InvalidArgs { reason }` — strip that wrapper
            // so the printed message matches the bare TS Error.message.
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ Search failed: {reason}");
                }
                _ => {
                    eprintln!("✗ Search failed: {err}");
                }
            }
            Ok(1)
        }
    }
}
