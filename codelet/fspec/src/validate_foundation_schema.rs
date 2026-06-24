//! `validate-foundation-schema` shell-facing CLI bridge (RPC-321).
//!
//! Feature: spec/features/validate-foundation-schema-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default) and delegates to the
//! single source-of-truth in
//! [`codelet_fspec_core::commands::validate_foundation_schema::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! The subcommand exposes NO flags, mirroring the flag-less TS Commander.js
//! registration at `src/commands/validate-foundation-schema.ts:138-144`.
//!
//! Exit-code contract (parity with `validateFoundationSchemaCommand` at
//! `src/commands/validate-foundation-schema.ts:119-136`):
//!   - success → print `result.output` to stdout, exit 0.
//!   - failure → write `Error:` + the joined error messages to stderr, exit 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_foundation_schema;
use serde::Deserialize;

/// `validate-foundation-schema` accepts no flags; the struct exists only to
/// keep the `forward!` macro call shape consistent with the other bridges.
#[derive(Debug, Default)]
pub struct CliArgs;

/// Structured `{success, output?, error?}` envelope returned by the core
/// command.
#[derive(Debug, Deserialize)]
struct Outcome {
    success: bool,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Entry point invoked from `main.rs` for the `validate-foundation-schema`
/// clap subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(_args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let payload = validate_foundation_schema::run("{}", &project_root).await?;
    let outcome: Outcome =
        serde_json::from_str(&payload).context("parse validate-foundation-schema JSON payload")?;

    if outcome.success {
        if let Some(out) = outcome.output.as_deref() {
            println!("{out}");
        }
        Ok(0)
    } else {
        let msg = outcome
            .error
            .unwrap_or_else(|| "unknown validation error".to_string());
        eprintln!("Error: {msg}");
        Ok(1)
    }
}
