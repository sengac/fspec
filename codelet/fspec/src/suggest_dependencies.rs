//! `suggest-dependencies` shell-facing CLI bridge (RPC-309).
//!
//! Feature: spec/features/suggest-dependencies-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::suggest_dependencies::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::suggest_dependencies::run
//!
//! This module performs NO domain computation: it only marshals the parsed
//! clap arguments into the JSON arg shape, calls the single fspec-core entry
//! point, and prints the returned body. All suggestion logic lives in
//! fspec-core.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered body (text summary or pretty JSON) is
//!     written to stdout via `println!`.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the TS-parity prefix
//!     `✗ Failed to suggest dependencies:` plus an `Error: <msg>` line are
//!     written to stderr.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::suggest_dependencies;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/suggest-dependencies.ts:206-217`): the single
/// `--output <format>` option defaulting to `text`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--output <format>`: `"text"` (default) or `"json"`.
    pub output: Option<String>,
}

/// Entry point invoked from `main.rs` for the `suggest-dependencies` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let output = args.output.as_deref().unwrap_or("text").to_string();
    let payload: Value = json!({ "output": output });
    let args_json = payload.to_string();

    match suggest_dependencies::run(&args_json, &project_root).await {
        Ok(rendered) => {
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            // The supervisor-approved CLI feature spec
            // (suggest-dependencies-cli-subcommand) requires stderr to contain
            // BOTH the TS-parity prefix `✗ Failed to suggest dependencies:` and
            // an `Error:` line carrying the underlying cause. Emit both lines
            // (the spec/tests are canon over the TS single-line form).
            eprintln!("✗ Failed to suggest dependencies:");
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
