//! `validate-config` shell-facing CLI bridge (CONFIG-009).
//!
//! Feature: spec/features/config-scope-validation.feature
//!
//! Rust-only extension command (DISC-003 pattern, like `foundation-status`).
//! Thin façade: the clap `Mode::ValidateConfig` variant parses argv and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::validate_config::run`] — the SAME
//! function the LLM-facing dispatcher invokes (two-front-doors, RPC-003
//! §7/§11).
//!
//! The core returns an envelope `{valid, exitCode, message, warnings[]}`;
//! this bridge prints `message` to stdout and exits with `exitCode`. No
//! validation logic lives here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_config;
use serde_json::Value;

/// Strongly-typed args. `validate-config` declares no CLI flags, so the
/// JSON args shape is `{}`.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `validate-config` clap
/// subcommand. Prints the core's `message` to stdout and returns its
/// `exitCode`.
pub async fn run(_args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    match validate_config::run("{}", &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            if let Some(message) = parsed.get("message").and_then(Value::as_str) {
                println!("{message}");
            }
            let exit_code = parsed.get("exitCode").and_then(Value::as_u64).unwrap_or(0) as u8;
            Ok(exit_code)
        }
        Err(err) => {
            eprintln!("Error: {}", crate::common::render_core_error(&err));
            Ok(1)
        }
    }
}
