//! `register-tag` shell-facing CLI bridge (RPC-265).
//!
//! Feature: spec/features/register-tag-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::RegisterTag` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::register_tag::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::register_tag::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::register_tag::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the
//! TypeScript `process.cwd()` default at
//! `src/commands/register-tag.ts:29`). NO logic in the bridge — JSON
//! marshalling only.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path at `src/commands/register-tag.ts:163-166`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::register_tag;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// argument set for `register-tag` (`src/commands/register-tag.ts:171-175`).
#[derive(Debug)]
pub struct CliArgs {
    /// Tag name (e.g., `"@my-tag"`).
    pub tag: String,
    /// Category display name (e.g., `"Technical Tags"`).
    pub category: String,
    /// Free-text tag description.
    pub description: String,
}

/// Entry point invoked from `main.rs` for the `register-tag` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal positional args into the JSON shape that
    // fspec_core::commands::register_tag::run validates with serde.
    let args_json = json!({
        "tag": args.tag,
        "category": args.category,
        "description": args.description,
    })
    .to_string();

    match register_tag::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Rendered text already includes the leading "✓ <message>\n"
            // and the trailing "  Updated: spec/tags.json\n
            // Regenerated: spec/TAGS.md\n" lines. Print as-is.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('Error:', error.message)`.
            // `render_core_error` strips the dispatcher-only
            // `"Invalid args for fspec command register-tag: "` envelope
            // so the shell stderr is byte-identical to TS.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
