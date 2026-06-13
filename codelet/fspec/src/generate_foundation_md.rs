//! `generate-foundation-md` shell-facing CLI bridge (RPC-233).
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core run.
//! The bridge is JSON marshalling + stdout/stderr rendering only; all domain
//! logic (read foundation.json, render markdown, write FOUNDATION.md) lives
//! in [`codelet_fspec_core::commands::generate_foundation_md::run`].
//!
//! TS reference output (piped, non-TTY):
//!   * success → `✓ Generated <output> from spec/foundation.json` (exit 0)
//!   * error   → `Error: <message>` (exit 1)

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_foundation_md;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub output: Option<String>,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut payload = json!({});
    if let Some(output) = args.output {
        payload["output"] = Value::String(output);
    }
    let args_json = payload.to_string();

    match generate_foundation_md::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Generated spec/FOUNDATION.md from spec/foundation.json");
            println!("✓ {msg}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
