//! `show-deleted` shell-facing CLI bridge (RPC-301).
//!
//! Feature: spec/features/show-deleted-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::ShowDeleted` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::show_deleted::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-301):
//!   - Shell argv         → clap → this module → fspec_core::commands::show_deleted::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::show_deleted::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/show-deleted.ts:28`). The clap
//! subcommand has a single REQUIRED positional `<WORK_UNIT_ID>` and NO flags
//! — matching the TS Commander.js registration at
//! `src/commands/show-deleted.ts:72-77`.
//!
//! No collection / filter / rendering logic is duplicated here — the bridge's
//! only computation is JSON arg marshalling. The
//! `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher` test
//! scans this file for forbidden substrings (the JSON field names, the empty
//! sentinel string, the rendered header prefix, and the per-category keys)
//! to enforce that invariant — see that test for the exact list.
//!
//! Exit-code contract:
//!   - 0 on success — including the empty-deleted-items sentinel.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `✗ Failed to show deleted items:` (parity with
//!     the TS chalk-red error path via `output.error('Failed to show deleted
//!     items:', ...)`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::FspecCoreError;
use codelet_fspec_core::commands::show_deleted;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js positional surface
/// for `show-deleted`. The TS registration declares ONLY the `<workUnitId>`
/// positional and no `.option(...)` calls, so `CliArgs` carries exactly one
/// field. The struct shape mirrors `list_attachments::CliArgs` for cross-
/// command parity.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `show-deleted` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim via
/// `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The marshalling lives here (rather than a hard-coded string literal)
    // so adding a field to `CliArgs` automatically threads through.
    let args_json = json!({
        "workUnitId": args.work_unit_id,
    })
    .to_string();

    match show_deleted::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure; print
            // as-is and avoid a duplicate \n that would shift the header.
            // The empty-result sentinel has no trailing newline, so we
            // append one for shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Failed to show deleted items:', error.message)`
            // path (`src/commands/show-deleted.ts:78` calls `output.error(...)` which
            // prefixes the line with the red ✗ marker). The dispatcher wraps domain
            // errors in `FspecCoreError::InvalidArgs { reason }` — strip that wrapper
            // so the printed message matches the bare TS Error.message.
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ Failed to show deleted items: {reason}");
                }
                _ => {
                    eprintln!("✗ Failed to show deleted items: {err}");
                }
            }
            Ok(1)
        }
    }
}
