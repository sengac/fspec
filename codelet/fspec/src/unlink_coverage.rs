//! `unlink-coverage` shell-facing CLI bridge (RPC-311).
//!
//! Feature: spec/features/unlink-coverage-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::unlink_coverage::run`, then surface the `message`
//! field on stdout. All mutation, stats recomputation, and atomic write-back
//! logic lives exclusively in fspec_core — the bridge performs only JSON arg
//! marshalling.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::unlink_coverage;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature_name: String,
    pub scenario: String,
    pub test_file: Option<String>,
    pub impl_file: Option<String>,
    pub all: bool,
}

/// Entry point for the `unlink-coverage` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert(
        "featureName".to_string(),
        Value::String(args.feature_name.clone()),
    );
    payload.insert("scenario".to_string(), Value::String(args.scenario.clone()));
    if let Some(tf) = &args.test_file {
        payload.insert("testFile".to_string(), Value::String(tf.clone()));
    }
    if let Some(imf) = &args.impl_file {
        payload.insert("implFile".to_string(), Value::String(imf.clone()));
    }
    payload.insert("all".to_string(), Value::Bool(args.all));
    let args_json = json!(payload).to_string();

    match unlink_coverage::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns `{ success, message }`; the bridge surfaces the
            // message on stdout (parity with the TS `output.log(result.message)`).
            let envelope: Value =
                serde_json::from_str(&rendered).context("parse fspec_core envelope")?;
            let msg = envelope
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            println!("{msg}");
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)` path. The
            // dispatcher wraps domain errors in `FspecCoreError::InvalidArgs
            // { reason }` — strip that wrapper so the printed message matches
            // the bare TS Error.message.
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
