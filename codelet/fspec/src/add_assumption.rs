//! `add-assumption` shell-facing CLI bridge (RPC-169).
//!
//! Feature: spec/features/add-assumption-cli-subcommand.feature
//!
//! Two-front-doors: marshals positional args to JSON {workUnitId, assumption}
//! and delegates to codelet_fspec_core::commands::add_assumption::run.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_assumption;
use serde_json::json;

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub assumption: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;
    let body = json!({
        "workUnitId": args.work_unit_id,
        "assumption": args.assumption,
    });
    let args_json = body.to_string();

    match add_assumption::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            println!("\u{2713} Assumption added successfully");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to add assumption:', error.message)`
            // at src/commands/add-assumption.ts:76.
            eprintln!("✗ Failed to add assumption: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
