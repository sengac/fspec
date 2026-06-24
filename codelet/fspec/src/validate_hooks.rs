//! `validate-hooks` shell-facing CLI bridge (RPC-322).
//!
//! Feature: spec/features/validate-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ValidateHooks` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::validate_hooks::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!
//! - Shell argv: clap -> this module -> `fspec_core::commands::validate_hooks::run`
//! - LLM tool call JSON: `fspec_core::dispatch::dispatch_command` -> the same run
//!
//! ## CLI rendering — print the core message, exit with the core exitCode
//!
//! The acceptance criteria (the feature file scenarios) require the shell
//! subcommand to PRINT the validation outcome and exit non-zero on failure
//! (the success banner with exit 0; the failure banner and per-script
//! diagnostics with exit 1; the load-failure line with exit 1).
//!
//! All of that text and the `exitCode` come from the single source-of-truth
//! envelope returned by [`codelet_fspec_core::commands::validate_hooks::run`]
//! (the SAME function the dispatcher uses). This bridge embeds NO validation
//! logic — it only marshals args and renders the envelope's `message` +
//! `exitCode`. It intentionally does NOT replicate the TS Commander `.action`
//! defect (which silently discarded the result); the feature file is the
//! authority and mandates the rendered output.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_hooks;
use serde_json::Value;

use crate::common::render_core_error;

/// Strongly-typed args. `validate-hooks` declares no CLI flags (parity with
/// the flag-less TS Commander.js registration), so the JSON shape is `{}`.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `validate-hooks` clap
/// subcommand. Prints the core's `message` to stdout and returns its
/// `exitCode`, per the feature file's acceptance criteria.
pub async fn run(_args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    match validate_hooks::run("{}", &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            if let Some(message) = parsed.get("message").and_then(Value::as_str) {
                println!("{message}");
            }
            let exit_code = parsed.get("exitCode").and_then(Value::as_u64).unwrap_or(0) as u8;
            Ok(exit_code)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
