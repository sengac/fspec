//! `list-features` shell-facing CLI bridge (RPC-245).
//!
//! Feature: spec/features/list-features-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin façade
//! that parses argv (the `Mode::ListFeatures` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_features::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-245):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_features::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_features::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/list-features.ts:29`). The clap subcommand exposes a
//! single `--tag <TAG>` flag — matching the TS Commander.js registration
//! at `src/commands/list-features.ts:156`. No filter / glob / parse /
//! render logic is duplicated here.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 2 when the underlying error message contains the substring
//!     `"Directory not found"` (parity with TS `process.exit(2)` at
//!     `src/commands/list-features.ts:145`).
//!   - 1 on any other [`codelet_fspec_core::FspecCoreError`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_features;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-features` (`src/commands/list-features.ts:156`).
///
/// The TS registration declares ONE option — `--tag <tag>`. The bridge
/// marshals it into the JSON shape that
/// `fspec_core::commands::list_features::run` validates with serde.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Optional exact-string tag filter (e.g. `"@critical"`).
    pub tag: Option<String>,
}

/// Entry point invoked from `main.rs` for the `list-features` clap
/// subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-
    // driven invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that
    // `fspec_core::commands::list_features::run` validates with serde.
    // Only fields explicitly supplied via clap are included so the
    // serde `#[serde(default)]` arms behave correctly.
    let mut obj: Map<String, Value> = Map::new();
    if let Some(tag) = args.tag.as_deref() {
        obj.insert("tag".to_string(), Value::String(tag.to_string()));
    }
    let args_json = json!(obj).to_string();

    match list_features::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure;
            // print as-is and avoid a duplicate \n that would shift
            // the header. The sentinel (rendered by fspec_core) has no
            // trailing newline, so we append one for shell-pipeline
            // friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            let msg = err.to_string();
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required. Detect the ENOENT branch by
            // substring (the orchestrator will rewrite this to match
            // on the dedicated `DirectoryNotFound` variant during the
            // wiring batch).
            eprintln!("Error: {msg}");
            if msg.contains("Directory not found") {
                Ok(2)
            } else {
                Ok(1)
            }
        }
    }
}
