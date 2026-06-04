//! `list-tags` shell-facing CLI bridge (RPC-251).
//!
//! Feature: spec/features/list-tags-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::ListTags` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_tags::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-251):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_tags::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_tags::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/list-tags.ts:30`). The clap subcommand exposes
//! `--category` only — matching the TS Commander.js registration at
//! `src/commands/list-tags.ts:101-105`. No filter, sorting, or
//! rendering logic is duplicated here.
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-251):
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message
//!     is written to stderr prefixed with `Error:` (parity with the
//!     TS chalk-red `output.error('Error:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_tags;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-tags` (`src/commands/list-tags.ts:101-105`).
///
/// The TS registration declares exactly one `.option(...)` call —
/// `--category <category>` — so this struct carries a single
/// `Option<String>` field. Future flag additions land as field
/// additions only, preserving the bridge's `run` signature.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Exact-match category filter passed through to
    /// `fspec_core::commands::list_tags::run`. `None` ⇔ no filter.
    pub category: Option<String>,
}

/// Entry point invoked from `main.rs` for the `list-tags` clap
/// subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON object expected by
    // `fspec_core::commands::list_tags::run`. Only emit the
    // `category` key when set, so the JSON shape mirrors the TS
    // Commander.js `options` object (where omitted flags are
    // `undefined`).
    let mut obj = serde_json::Map::new();
    if let Some(category) = args.category {
        obj.insert("category".to_string(), json!(category));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match list_tags::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure;
            // print as-is. The TS implementation always emits at
            // least one trailing `\n` via `output.log('')`, so the
            // rendered string is already shell-pipeline friendly.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)`
            // path: stderr, prefixed, no ANSI required for parity
            // with RPC-253 rule [14]. The canonical error substrings
            // are carried in the Display impl of FspecCoreError
            // verbatim by fspec_core itself.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
