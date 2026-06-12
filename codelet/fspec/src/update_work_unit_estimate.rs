//! `update-work-unit-estimate` shell-facing CLI bridge (RPC-318).
//!
//! Feature: spec/features/update-work-unit-estimate-cli-subcommand.feature
//!
//! Parses the `Mode::UpdateWorkUnitEstimate` clap variant (in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::update_work_unit_estimate::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Exit-code contract (parity with TS
//! `src/commands/update-work-unit-estimate.ts:146-158`):
//!   - 0 on success; prints `✓ Work unit <id> estimate set to <points>` to
//!     stdout (the `<points>` echoed is the raw CLI argument, matching TS
//!     which interpolates the string `estimate` not the parsed number).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; prints
//!     `✗ Failed to update estimate: <reason>` to stderr.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_work_unit_estimate;
use codelet_fspec_core::js_compat::parse_js_int;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/update-work-unit-estimate.ts:140-159`: two required
/// positionals `<id> <points>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw `<points>` argument as typed on the CLI. Parsed to an integer for
    /// the core call (TS `parseInt(estimate, 10)`); echoed verbatim in the
    /// success line.
    pub points: String,
}

/// Entry point invoked from `main.rs` for the `update-work-unit-estimate`
/// clap subcommand. Returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // TS: `parseInt(estimate, 10)`. `parse_js_int` reproduces JS `parseInt`
    // exactly: leading-digit values coerce to a number (`5abc` → 5,
    // `13.9` → 13) and unparseable values become JSON `null` (the `NaN`
    // sentinel the core renders as `Invalid estimate: NaN`). A strict integer
    // parse here would diverge from TS by rejecting `5abc`/`13.9` and by
    // printing a garbage sentinel value instead of `NaN`.
    let estimate = parse_js_int(&args.points);

    let args_json = json!({
        "workUnitId": args.work_unit_id,
        "estimate": estimate,
    })
    .to_string();

    match update_work_unit_estimate::run(&args_json, &project_root).await {
        Ok(_) => {
            println!(
                "✓ Work unit {} estimate set to {}",
                args.work_unit_id, args.points
            );
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to update estimate: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
