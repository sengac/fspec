//! `review` shell-facing CLI bridge (RPC-295).
//!
//! Feature: spec/features/review-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 as the
//! Commander.js equivalent. This module is the thin façade that parses argv
//! (the `Mode::Review` clap variant in [`crate::main`]) and delegates to the
//! single source-of-truth in
//! [`codelet_fspec_core::commands::review::run`] — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::review::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → review::run
//!
//! Per the supervisor ruling, `review` follows the delete-scenarios
//! SPECIAL-CASE: bare clap-generated `--help` (NO rich byte-parity fixture,
//! NO help CONFIG, NO help-intercept arm).
//!
//! No review logic is duplicated here — the bridge's only computation is JSON
//! arg marshalling and stdout printing. The delegation test
//! `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher` scans
//! this file for forbidden report substrings.
//!
//! Exit-code contract:
//!   - 0 on success.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `Error:` (parity with the TS chalk-red error
//!     path). The dispatcher wraps domain errors in `InvalidArgs { reason }`
//!     — strip that wrapper so the printed message matches the bare TS
//!     `Error.message` (e.g. `Work unit '<id>' does not exist`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::review;
use codelet_fspec_core::FspecCoreError;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface for
/// `review` (`src/commands/review.ts:569-575`): one required positional
/// `<work-unit-id>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the work-unit identifier to review.
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `review` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "workUnitId": args.work_unit_id }).to_string();

    match review::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Parity with the TS `.action`'s `output.log(result.output)`:
            // `console.log` UNCONDITIONALLY appends a trailing newline. The
            // core report already ends in `\n` (its final `lines.push('')`),
            // so the TS CLI emits a terminating blank line (`…\n\n`).
            // `println!` reproduces that exactly; a conditional
            // `ends_with('\n')` guard would drop the trailing blank line.
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("Error: {reason}");
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
