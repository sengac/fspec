//! `list-prefixes` shell-facing CLI bridge (RPC-248).
//!
//! Feature: spec/features/list-prefixes-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ListPrefixes` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_prefixes::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-248):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_prefixes::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_prefixes::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/list-prefixes.ts:39`). The clap
//! subcommand carries NO flags — matching the flag-less TS Commander.js
//! registration at `src/commands/list-prefixes.ts:101-104`. No filter or
//! rendering logic is duplicated here.
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-248):
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_prefixes;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-prefixes` (`src/commands/list-prefixes.ts:101-104`).
///
/// The TS registration declares NO `.option(...)` calls, so this struct
/// currently has no public fields — the JSON shape handed to
/// `fspec_core::commands::list_prefixes::run` always serialises to `{}`.
/// The struct is kept (mirroring the `list_work_units::CliArgs` shape)
/// so future flag additions (e.g. a `--format json` parity surface)
/// land as field additions rather than an API break, and so the
/// bridge's `run` signature stays symmetric with `list_work_units::run`
/// for the cross-command parity expected by RPC-003 §7/§11.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `list-prefixes` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(_args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core::commands::
    // list_prefixes::run validates with serde. With no flags currently
    // exposed on `CliArgs`, the shape is the empty object — matching
    // both the TS Commander.js behaviour and `fspec_core`'s
    // `#[serde(default)]` arms. The marshalling lives here (rather
    // than a hard-coded `"{}"`) so adding a field to `CliArgs`
    // automatically threads through to `args_json`.
    let obj = serde_json::Map::new();
    let args_json = json!(obj).to_string();

    match list_prefixes::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure; print
            // as-is and avoid a duplicate \n that would shift the header.
            // The empty-result sentinel (rendered by fspec_core) has no
            // trailing newline, so we append one for shell-pipeline
            // friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required for parity with RPC-253 rule [14].
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
