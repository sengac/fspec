//! `query-dependency-stats` shell-facing CLI bridge (RPC-257).
//!
//! Feature: spec/features/query-dependency-stats-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::QueryDependencyStats` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::query_dependency_stats::run`] — the
//! SAME function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::query_dependency_stats::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_dependency_stats::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/query-dependency-stats.ts:70`).
//!
//! The clap subcommand carries a single `--format <format>` flag (default
//! `"text"`), matching the TS Commander.js registration at
//! `src/commands/query-dependency-stats.ts:144`. No aggregation, DFS, or
//! rendering logic is duplicated here — the bridge module deliberately
//! contains NONE of the canonical field names so a parity-regression test
//! locks the no-duplication contract.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered output (empty for text, pretty-JSON for
//!     `--format json`) is written to stdout. The text path mirrors the TS
//!     CLI which prints nothing when `format !== 'json'`.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS chalk-red
//!     `output.error('✗ Query failed:', error.message)` path; the leading
//!     `Error:` substring is asserted by the CLI red-phase tests).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_dependency_stats;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag set
/// for `query-dependency-stats` (`src/commands/query-dependency-stats.ts:144`).
///
/// Only `format` is exposed today — the TS registration also declares no
/// other action-side flags. (`--show-critical-path` appears in the help
/// fixture as a documented future option but the TS implementation has no
/// `.option('--show-critical-path', ...)` binding, so the Rust bridge
/// matches the TS runtime, not the help doc.)
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Output format selector: `"text"` (default — silent) or `"json"`.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `query-dependency-stats` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // We only include `format` when supplied so the dispatcher-facing schema
    // stays consistent across CLI and tool-call paths.
    let args_json = match args.format.as_deref() {
        Some(fmt) => json!({ "format": fmt }).to_string(),
        None => json!({}).to_string(),
    };

    match query_dependency_stats::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // TS-parity: text path produces an empty string here and the CLI
            // prints nothing to stdout (no trailing newline). JSON path
            // produces pretty-printed output that already has internal
            // newlines; we still terminate with one newline for
            // shell-pipeline friendliness — matching `output.log` which
            // appends `\n`. We only emit the trailing newline when there is
            // actual content (so the silent text path stays byte-empty).
            if !rendered.is_empty() {
                println!("{rendered}");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗ Query failed:', error.message)`
            // path: stderr, `Error:` prefix, no ANSI required for parity
            // (the CLI red-phase test asserts the `Error:` substring).
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
