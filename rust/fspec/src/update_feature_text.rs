//! `update-feature-text` shell-facing CLI bridge (BUG-507).
//!
//! Feature: spec/features/update-feature-text-edits-non-step-feature-text.feature
//!
//! Two front doors: shell argv → clap → this module, and LLM tool calls →
//! `dispatch_command`, both reach
//! `codelet_fspec_core::commands::update_feature_text::run`. This bridge only
//! marshals the arguments and routes the message or error.

use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_feature_text;
use serde_json::{json, Value};

use crate::common::{render_core_error, spec_project_root};

/// Arguments of `fspec update-feature-text <feature> --find <text> --replace <text>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature: String,
    pub find: String,
    pub replace: String,
}

/// Entry point for the clap subcommand; returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;
    let args_json = json!({
        "feature": args.feature,
        "find": args.find,
        "replace": args.replace,
    })
    .to_string();
    match update_feature_text::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let v: Value = serde_json::from_str(&data_json).context("parse core JSON response")?;
            println!("{}", v.get("message").and_then(Value::as_str).unwrap_or(""));
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
