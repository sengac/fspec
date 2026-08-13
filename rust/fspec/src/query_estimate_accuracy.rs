//! `query-estimate-accuracy` shell-facing CLI bridge (RPC-258).
//!
//! Feature: spec/features/query-estimate-accuracy-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::QueryEstimateAccuracy` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::query_estimate_accuracy::run`] — the
//! SAME function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-258):
//!   - Shell argv         → clap → this module → fspec_core::commands::query_estimate_accuracy::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_estimate_accuracy::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/query-estimate-accuracy.ts:41`).
//! The clap subcommand exposes only `--format / -f` — mirroring the TS
//! Commander.js registration which advertises only that one user-facing
//! flag. Any non-public dispatcher-only inputs stay internal to fspec_core.
//!
//! No aggregation, grouping, percentage-rounding, or rendering logic is
//! duplicated here — the bridge's only computation is JSON arg marshalling.
//! The CLI-delegation test scans this file for any business-logic
//! substrings; keeping this module thin is essential.
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-258):
//!   - 0 on success — including the empty-workspace case where the report
//!     header and sentinel line are emitted by fspec_core.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `Error:` (parity with the TS chalk-red error
//!     path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_estimate_accuracy;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag set
/// for `query-estimate-accuracy`. The TS Commander registration declares
/// a single user-facing option (`-f, --format <format>`) — no other flags
/// are surfaced on the CLI here (the help-output test asserts that no
/// extra flags appear in `--help`).
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Optional rendering mode. `None` → use fspec_core's text default;
    /// `Some("json")` → emit pretty-printed JSON.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `query-estimate-accuracy`
/// clap subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // Only thread `format` through when the CLI flag was supplied so
    // fspec_core's default-arm (`format: None` → text) drives unflagged
    // invocations.
    let mut obj = serde_json::Map::new();
    if let Some(fmt) = args.format.as_deref() {
        obj.insert("format".into(), json!(fmt));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match query_estimate_accuracy::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Text format embeds its own newline structure (see fspec-core);
            // print as-is. The JSON format from `to_string_pretty` does not
            // end with a newline, so we append one for shell-pipeline
            // friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `console.error(chalk.red('Error:'), …)` path:
            // stderr, prefixed with `Error:`, no ANSI required for parity
            // with the cross-port error contract. The fspec_core error
            // message already embeds the wrapper prefix the CLI test
            // asserts on.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
