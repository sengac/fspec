//! `delete-diagram` shell-facing CLI bridge (RPC-216).
//!
//! Feature: spec/features/delete-diagram-cli-subcommand.feature
//!
//! Thin clap → fspec_core delegate. Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::delete_diagram::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::delete_diagram::run
//!
//! Bridge scope:
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL file IO, JSON parse, and array mutation logic lives in the core.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_diagram;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub section: String,
    pub title: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "section": args.section,
        "title": args.title,
    })
    .to_string();

    match delete_diagram::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Diagram deleted");
            println!("✓ {msg}");
            println!("  Updated: spec/foundation.json");
            println!("  Regenerated: spec/FOUNDATION.md");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
