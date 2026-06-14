//! `generate-tags-md` shell-facing CLI bridge (RPC-236).
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core run.
//! The bridge is JSON marshalling + stdout/stderr rendering only; all domain
//! logic (read tags.json, schema-validate, render markdown, write TAGS.md)
//! lives in [`codelet_fspec_core::commands::generate_tags_md::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! TS reference output (piped, non-TTY):
//!   * success → `✓ Generated <output> from spec/tags.json` (exit 0)
//!   * error   → `Error: <message>` (exit 1)

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_tags_md;
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

    match generate_tags_md::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed.get("message").and_then(Value::as_str).unwrap_or("");
            println!("✓ {msg}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
