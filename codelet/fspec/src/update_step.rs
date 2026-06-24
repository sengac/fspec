//! `update-step` shell-facing CLI bridge (RPC-315).
//!
//! Feature: spec/features/update-step-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::update_step::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::update_step::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/update-step.ts:33`).
//! No domain logic in this bridge — JSON marshalling + ✓-prefix printing
//! and success/error stream routing only.
//!
//! Exit-code contract (parity with TS
//! `src/commands/update-step.ts:195-220`):
//!   - 0 on success; the canonical `message` field returned by core is
//!     prefixed with `✓ ` and written to stdout.
//!   - 1 on a soft failure (`{success:false,error}`): the `error` text is
//!     written to stderr prefixed with `Error:`.
//!   - 1 on any escalated [`codelet_fspec_core::FspecCoreError`]; the
//!     unwrapped reason is written to stderr prefixed with `Error:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_step;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/update-step.ts:223-235`. The two
/// optional flags are `Option<String>` so unset flags are elided from
/// the JSON payload (parity with Commander.js `undefined` options).
#[derive(Debug, Default)]
pub struct CliArgs {
    pub feature: String,
    pub scenario: String,
    pub current_step: String,
    pub text: Option<String>,
    pub keyword: Option<String>,
}

/// Entry point invoked from `main.rs` for the `update-step` clap
/// subcommand. Returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = Map::new();
    obj.insert("feature".to_string(), Value::String(args.feature.clone()));
    obj.insert("scenario".to_string(), Value::String(args.scenario.clone()));
    obj.insert(
        "currentStep".to_string(),
        Value::String(args.current_step.clone()),
    );
    if let Some(t) = &args.text {
        obj.insert("text".to_string(), Value::String(t.clone()));
    }
    if let Some(k) = &args.keyword {
        obj.insert("keyword".to_string(), Value::String(k.clone()));
    }
    let args_json = json!(obj).to_string();

    match update_step::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let v: Value = serde_json::from_str(&data_json).context("parse core JSON response")?;
            if v.get("success").and_then(Value::as_bool) == Some(false) {
                let err = v.get("error").and_then(Value::as_str).unwrap_or("");
                eprintln!("Error: {err}");
                return Ok(1);
            }
            let message = v.get("message").and_then(Value::as_str).unwrap_or("");
            println!("✓ {message}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
