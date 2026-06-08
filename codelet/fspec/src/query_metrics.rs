//! `query-metrics` shell-facing CLI bridge (RPC-261).
//!
//! Feature: spec/features/query-metrics-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::QueryMetrics` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::query_metrics::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::query_metrics::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_metrics::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/query-metrics.ts:39`). The clap subcommand carries
//! `--work-unit-id`, `--type` and `--format` — matching the TS
//! Commander.js registration at `src/commands/query-metrics.ts:182-198`.
//!
//! No filter / aggregation / rendering logic is duplicated here — that
//! would split the dispatcher and CLI answer shapes.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout
//!     followed by a single trailing newline (parity with TS
//!     `output.log(...)` which always appends `\n`).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Query failed:` (parity with
//!     the TS `output.error('✗ Query failed:', error.message)` path
//!     at `src/commands/query-metrics.ts:250`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::query_metrics;
use serde_json::Value;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `query-metrics` (`src/commands/query-metrics.ts:182-198`).
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--work-unit-id <id>` — query metrics for a single unit when set.
    pub work_unit_id: Option<String>,
    /// `--type <type>` — filter aggregate metrics to `story`, `task` or
    /// `bug`.
    pub r#type: Option<String>,
    /// `--format <format>` — `text` (default) or `json`. The TS default
    /// is `text` (`src/commands/query-metrics.ts:187`).
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `query-metrics` clap
/// subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve cwd")?;

    // Build the args JSON payload using a serde_json::Map so the keys
    // appear in a stable order and only-when-set fields are omitted
    // (parity with the TS object-literal: undefined fields are simply
    // missing on the wire).
    let mut payload = serde_json::Map::new();
    if let Some(id) = &args.work_unit_id {
        payload.insert("workUnitId".to_string(), Value::String(id.clone()));
    }
    if let Some(t) = &args.r#type {
        payload.insert("type".to_string(), Value::String(t.clone()));
    }
    let fmt = args.format.clone().unwrap_or_else(|| "text".to_string());
    payload.insert("format".to_string(), Value::String(fmt));

    let args_json = Value::Object(payload).to_string();

    match query_metrics::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The rendered text already ends with a `\n` (the text renderer
            // appends one per line), and JSON via `to_string_pretty` does
            // NOT end with a newline. Mirror the `show_deleted::run` bridge
            // pattern: print as-is, then add a single trailing newline only
            // when one isn't already present, so the byte count matches the
            // TS surface exactly.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗ Query failed:', error.message)`
            // path: stderr, prefixed, no ANSI required for parity. Strip the
            // FspecCoreError::InvalidArgs wrapper prefix so the printed
            // message matches the bare TS Error.message.
            let msg = strip_invalid_args_wrapper(&err);
            eprintln!("✗ Query failed: {msg}");
            Ok(1)
        }
    }
}

/// Strip the `Invalid args for fspec command <name>:` prefix from a
/// [`FspecCoreError::InvalidArgs`] Display so the user-facing message
/// matches the TS `Error.message`. Other variants pass through verbatim.
fn strip_invalid_args_wrapper(err: &codelet_fspec_core::FspecCoreError) -> String {
    match err {
        codelet_fspec_core::FspecCoreError::InvalidArgs { reason, .. } => reason.clone(),
        _ => err.to_string(),
    }
}
