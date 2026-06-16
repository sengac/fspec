//! `link-coverage` shell-facing CLI bridge (RPC-240).
//!
//! Feature: spec/features/link-coverage-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::link_coverage::run`, then surface the result message
//! (and any yellow warnings) on stdout. All mutation, validation, and rendering
//! logic lives exclusively in fspec_core — the bridge performs only JSON arg
//! marshalling and envelope decoding.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::link_coverage;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature_name: String,
    pub scenario: String,
    pub test_file: Option<String>,
    pub test_lines: Option<String>,
    pub impl_file: Option<String>,
    pub impl_lines: Option<String>,
    pub skip_validation: bool,
    pub skip_step_validation: bool,
}

/// Entry point for the `link-coverage` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert(
        "featureName".to_string(),
        Value::String(args.feature_name.clone()),
    );
    payload.insert("scenario".to_string(), Value::String(args.scenario.clone()));
    if let Some(v) = &args.test_file {
        payload.insert("testFile".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &args.test_lines {
        payload.insert("testLines".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &args.impl_file {
        payload.insert("implFile".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &args.impl_lines {
        payload.insert("implLines".to_string(), Value::String(v.clone()));
    }
    payload.insert("skipValidation".to_string(), Value::Bool(args.skip_validation));
    payload.insert(
        "skipStepValidation".to_string(),
        Value::Bool(args.skip_step_validation),
    );
    let args_json = json!(payload).to_string();

    match link_coverage::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns `{ success, message, warnings? }`; surface the
            // message (and warnings) on stdout (parity with the TS output).
            let envelope: Value =
                serde_json::from_str(&rendered).context("parse fspec_core envelope")?;
            if let Some(m) = envelope.get("message").and_then(|m| m.as_str()) {
                println!("{m}");
            }
            if let Some(w) = envelope.get("warnings").and_then(|w| w.as_str()) {
                println!("\n{w}");
            }
            Ok(0)
        }
        Err(err) => {
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("Error: {reason}");
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
