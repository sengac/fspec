//! `list-foundation-sections` shell-facing CLI bridge (RPC-246).
//!
//! Feature: spec/features/list-foundation-sections-rust-port.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ListFoundationSections` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_foundation_sections::run`] — the
//! SAME function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-246):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_foundation_sections::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_foundation_sections::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default). The clap
//! subcommand exposes a single `--format <FORMAT>` flag (`text`
//! default, `json`) — matching the TS Commander.js registration.
//! No rendering logic is duplicated here.
//!
//! Exit-code contract (RPC-253 rule [14], reused for RPC-246):
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_foundation_sections;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-foundation-sections`.
///
/// The TS registration declares ONE option — `--format <format>`. The
/// bridge marshals it into the JSON shape that
/// `fspec_core::commands::list_foundation_sections::run` validates with
/// serde.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Output format: `"text"` (default) or `"json"`.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `list-foundation-sections`
/// clap subcommand. Returns the process exit code so `main` can
/// propagate it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-
    // driven invocations behave identically. (The fspec_core impl
    // ignores project_root for this command — the section list is a
    // static constant — but we pass CWD anyway for dispatcher symmetry.)
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that
    // `fspec_core::commands::list_foundation_sections::run` validates
    // with serde. Only fields explicitly supplied via clap are
    // included so the serde `#[serde(default)]` arms behave correctly.
    let mut obj: Map<String, Value> = Map::new();
    if let Some(format) = args.format.as_deref() {
        obj.insert("format".to_string(), Value::String(format.to_string()));
    }
    let args_json = json!(obj).to_string();

    match list_foundation_sections::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own trailing newline structure;
            // print as-is and avoid a duplicate \n that would shift
            // the header. Append a final newline only when missing so
            // shell pipelines stay tidy.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required for parity with RPC-253
            // rule [14].
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
