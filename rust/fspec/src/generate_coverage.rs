//! `generate-coverage` shell-facing CLI bridge (RPC-231).
//!
//! Feature: spec/features/generate-coverage-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::generate_coverage::run`, then print the rendered
//! report (already including the link-coverage guidance block) verbatim on
//! stdout. All scanning and rendering logic lives in fspec_core.

use crate::common::spec_project_root;
use std::path::PathBuf;

use anyhow::Result;
use codelet_fspec_core::commands::generate_coverage;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub dry_run: bool,
    /// REFAC-020: generate only this exact feature; the core returns a JSON plan.
    pub feature_name: Option<String>,
}

/// Entry point for the `generate-coverage` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert("dryRun".to_string(), Value::Bool(args.dry_run));
    if let Some(name) = &args.feature_name {
        payload.insert("featureName".to_string(), Value::String(name.clone()));
    }
    let args_json = json!(payload).to_string();

    match generate_coverage::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns the full report string; print it verbatim.
            println!("{rendered}");
            // A selected preview reports a blocked plan as `success: false`.
            let blocked = args.feature_name.is_some()
                && serde_json::from_str::<Value>(&rendered)
                    .is_ok_and(|plan| plan.get("success") == Some(&Value::Bool(false)));
            Ok(u8::from(blocked))
        }
        Err(err) => {
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ Error: {reason}");
                }
                _ => {
                    eprintln!("✗ Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
