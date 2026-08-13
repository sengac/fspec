//! `validate-work-units` shell-facing CLI bridge (RPC-325).
//!
//! Feature: spec/features/validate-work-units-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ValidateWorkUnits` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::validate_work_units::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::validate_work_units::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::validate_work_units::run
//!
//! ## Rendering (parity with TS `registerValidateWorkUnitsCommand`)
//! The core returns `{valid, checks, errors?}`. This bridge owns the
//! presentation only:
//!   - valid → `✓ All work units are valid` to stdout, exit 0
//!   - invalid → `✗ Found N validation errors` + one `  - <err>` line per
//!     error to STDERR, exit 1
//!
//! No validation logic lives here — the bridge never inspects work-unit
//! shape, only the envelope's `valid`/`errors`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_work_units;
use serde_json::{json, Value};

/// Strongly-typed args. The TS reference declares no functional flags; the
/// documented-only `--fix` lives in `--help` text but is rejected at runtime
/// as an unknown option (handled by clap in `main.rs`), so this struct is
/// empty and the JSON shape passed to the core is `{}`.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `validate-work-units` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let _ = args;
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({}).to_string();

    let json_text = match validate_work_units::run(&args_json, &project_root).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("✗ Failed to validate work units: {err}");
            return Ok(1);
        }
    };

    let value: Value = serde_json::from_str(&json_text).context("parse core response as JSON")?;

    let valid = value.get("valid").and_then(Value::as_bool).unwrap_or(false);
    if valid {
        println!("✓ All work units are valid");
        return Ok(0);
    }

    let errors: Vec<&str> = value
        .get("errors")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    eprintln!("✗ Found {} validation errors", errors.len());
    for e in &errors {
        eprintln!("  - {e}");
    }
    Ok(1)
}
