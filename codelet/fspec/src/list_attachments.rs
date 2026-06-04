//! `list-attachments` shell-facing CLI bridge (RPC-241).
//!
//! Feature: spec/features/list-attachments-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::ListAttachments` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_attachments::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-241):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_attachments::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_attachments::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/list-attachments.ts:17`). The clap
//! subcommand has a single REQUIRED positional `<WORK_UNIT_ID>` and no flags
//! — matching the TS Commander.js registration at
//! `src/commands/list-attachments.ts:62-66`.
//!
//! No lookup / iteration / stat / rendering logic is duplicated here — the
//! bridge's only computation is JSON arg marshalling (rule [14] on RPC-241,
//! enforced by the `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher`
//! test which scans this file for forbidden substrings).
//!
//! Exit-code contract (rule [15] on RPC-241):
//!   - 0 on success — including the empty-attachments sentinel and the
//!     ✗ missing-file case (TS exits 0 on both).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `Error:` (parity with TS chalk-red error path).
//!   - 2 (clap's own usage error) when the required positional is omitted —
//!     clap validates before this module is reached, so we never see that
//!     case here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_attachments;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js positional surface
/// for `list-attachments`. Today the TS registration declares ONLY the
/// `<workUnitId>` positional (no `.option(...)` calls), so `CliArgs` carries
/// exactly one field — kept as a `pub struct` rather than a tuple/newtype so
/// future flag additions (e.g. a `--format json` parity surface) land as
/// field additions only, mirroring the `list_work_units::CliArgs` shape.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `list-attachments` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The marshalling lives here (rather than a hard-coded string literal)
    // so adding a field to `CliArgs` automatically threads through to
    // `args_json`.
    let args_json = json!({
        "workUnitId": args.work_unit_id,
    })
    .to_string();

    match list_attachments::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure; print
            // as-is and avoid a duplicate \n that would shift the layout.
            // The empty-result sentinel has no trailing newline, so we
            // append one for shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required for parity with rule [15] on RPC-241.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
