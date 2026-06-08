//! `list-schedules` shell-facing CLI bridge (RPC-250).
//!
//! Feature: spec/features/list-schedules-rust-port.feature
//!         spec/features/list-schedules-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ListSchedules` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_schedules::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-250):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_schedules::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_schedules::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default). The clap
//! subcommand exposes `--json` only — matching the TS Commander.js
//! registration at `src/commands/schedule/list-schedules.ts:95-104`,
//! which declares exactly `.option('--json', 'Output as JSON')`. The
//! TS `--json` boolean is translated to the dispatcher's
//! `format: "json"` (truthy) / `format: "text"` (falsy) protocol — the
//! marshalling lives in this bridge so the underlying fspec_core
//! `run` signature remains uniform with the other ported commands
//! (list_hooks, list_tags, list_prefixes, list_epics).
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-250):
//!   - 0 on success; the rendered text or JSON is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message
//!     is written to stderr prefixed with `Error:` (parity with the
//!     TS chalk-red `output.error('Error:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_schedules;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-schedules`
/// (`src/commands/schedule/list-schedules.ts:95-104`).
///
/// The TS registration declares exactly one `.option(...)` call —
/// `--json` (a boolean switch) — so this struct carries a single
/// `bool` field. Future flag additions land as field additions only,
/// preserving the bridge's `run` signature.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `true` when the user passed `--json`. Mapped to the dispatcher
    /// `format` key: `true` → `"json"`, `false` → `"text"`.
    pub json: bool,
}

/// Entry point invoked from `main.rs` for the `list-schedules` clap
/// subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON object expected by
    // `fspec_core::commands::list_schedules::run`. Translate the TS
    // boolean `--json` into the dispatcher's `format` key:
    //   --json  → {"format":"json"}
    //   (none)  → {} (fspec_core defaults to text)
    // The TS implementation at `list-schedules.ts:103` performs the
    // same `opts.json ? 'json' : 'table'` translation; we mirror that
    // shape here so the bridge stays the only place that knows about
    // the boolean surface.
    let mut obj = serde_json::Map::new();
    if args.json {
        obj.insert("format".to_string(), json!("json"));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match list_schedules::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if args.json {
                // TS `--json` emits the BARE schedules array
                // (`output.log(JSON.stringify(result.schedules, null, 2))`),
                // NOT the dispatcher envelope `{schedules, columns}`. Project
                // the array out of the dispatcher payload so the CLI surface
                // stays byte-compatible with TS for `--json` consumers.
                //
                // The dispatcher path keeps the envelope (its documented
                // contract); the projection lives here in the bridge so
                // `fspec_core` retains a single canonical response shape.
                let parsed: Value = serde_json::from_str(&rendered)
                    .context("parse list-schedules JSON payload")?;
                let schedules = parsed
                    .get("schedules")
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new()));
                let projected = serde_json::to_string_pretty(&schedules)
                    .context("re-serialize schedules array")?;
                println!("{projected}");
                return Ok(0);
            }
            // text format embeds its own trailing newline structure
            // (rule [7]); the JSON pretty-print path does NOT, so we
            // append one for shell-pipeline friendliness in that case
            // only. Both the text-empty sentinel and the populated
            // text path already terminate with `\n`.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)`
            // path: stderr, prefixed, no ANSI required for parity
            // with RPC-253 rule [14].
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
