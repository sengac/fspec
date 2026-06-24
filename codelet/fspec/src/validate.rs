//! `validate` shell-facing CLI bridge (RPC-320).
//!
//! Feature: spec/features/validate-gherkin-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root from
//! CWD (parity with the TS `process.cwd()` default), marshals the optional
//! `[file]` positional and `-v/--verbose` flag into JSON, and delegates to the
//! single source-of-truth in
//! [`codelet_fspec_core::commands::validate::run`] — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! Exit-code contract (parity with `validateCommand` at
//! `src/commands/validate.ts:20-78`):
//!   - 0 → every validated file is valid; the display block goes to stdout.
//!   - 1 → one or more files have syntax errors; display block to stdout.
//!   - 2 → no feature files found OR an unexpected error; message to stderr.
//!
//! The core carries the 0/1 distinction in the JSON payload's `exitCode`
//! field; any `Err(FspecCoreError)` (no-files / unexpected) maps to exit 2.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate;
use serde::Deserialize;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/validate.ts:256-265`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Optional positional `[file]` to validate (validates all when absent).
    pub file: Option<String>,
    /// `-v/--verbose` flag.
    pub verbose: bool,
}

/// Structured `{success, output, exitCode, ...}` envelope returned by the core
/// command on the validated-files path.
#[derive(Debug, Deserialize)]
struct Outcome {
    #[serde(default)]
    output: String,
    #[serde(default, rename = "exitCode")]
    exit_code: u8,
}

/// Entry point invoked from `main.rs` for the `validate` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj: Map<String, Value> = Map::new();
    if let Some(file) = args.file.as_deref() {
        obj.insert("file".to_string(), Value::String(file.to_string()));
    }
    if args.verbose {
        obj.insert("verbose".to_string(), Value::Bool(true));
    }
    let args_json = json!(obj).to_string();

    match validate::run(&args_json, &project_root).await {
        Ok(payload) => {
            let outcome: Outcome =
                serde_json::from_str(&payload).context("parse validate JSON payload")?;
            // The display block is printed to stdout (parity with output.log).
            println!("{}", outcome.output);
            Ok(outcome.exit_code)
        }
        Err(err) => {
            // No feature files found OR an unexpected error → exit 2 with the
            // message on stderr (mirrors src/commands/validate.ts:28,75).
            eprintln!("{err}");
            Ok(2)
        }
    }
}
