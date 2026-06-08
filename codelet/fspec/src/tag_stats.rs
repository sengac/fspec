//! `tag-stats` shell-facing CLI bridge (RPC-310).
//!
//! Feature: spec/features/tag-stats-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::TagStats` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::tag_stats::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::tag_stats::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::tag_stats::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/tag-stats.ts:40`). The clap subcommand carries NO
//! flags — matching the flag-less TS Commander.js registration at
//! `src/commands/tag-stats.ts:258-262`. No counting, projection, or
//! rendering logic is duplicated here.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message
//!     is written to stderr prefixed with `Error:` (parity with the
//!     TS chalk-red `output.error('Error:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::tag_stats;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// flag set for `tag-stats` (`src/commands/tag-stats.ts:258-262`).
///
/// The TS registration declares NO `.option(...)` calls, so this
/// struct currently has no public fields — the JSON shape handed
/// to `fspec_core::commands::tag_stats::run` always carries only
/// the bridge-internal `format` discriminator (set to `"text"` for
/// CLI invocations to request the plain-text rendering path).
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `tag-stats` clap
/// subcommand. Returns the process exit code so `main` can
/// propagate it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(_args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // The CLI front door always requests text rendering — the
    // dispatcher front door requests JSON. The shape of the args
    // object is otherwise empty: no `--category`, no `--format`
    // flag is exposed at the shell.
    let args_json = json!({ "format": "text" }).to_string();

    match tag_stats::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // render_text always terminates with `\n`. Print as-is
            // and avoid a duplicate `\n` that would shift the
            // header position. (The empty-features sentinel still
            // ends with `\n` because the trailing `output.log('')`
            // in TS always emits one final blank line.)
            debug_assert!(
                rendered.ends_with('\n'),
                "tag_stats render_text contract: must end with \\n"
            );
            print!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)`
            // path: stderr, prefixed, no ANSI required for parity.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
