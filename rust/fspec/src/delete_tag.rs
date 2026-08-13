//! `delete-tag` shell-facing CLI bridge (RPC-222).
//!
//! Feature: spec/features/delete-tag-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::DeleteTag` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::delete_tag::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::delete_tag::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::delete_tag::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! NO logic in the bridge — JSON marshalling only.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', ...)` path at
//!     `src/commands/delete-tag.ts:177` / `194`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_tag;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// argument set for `delete-tag` (`src/commands/delete-tag.ts:199-206`).
#[derive(Debug)]
pub struct CliArgs {
    pub tag: String,
    pub force: bool,
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `delete-tag` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default (`src/commands/delete-tag.ts:30`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal positional + flag args into the JSON shape that
    // fspec_core::commands::delete_tag::run validates with serde.
    let body = json!({
        "tag": args.tag,
        "force": args.force,
        "dryRun": args.dry_run,
    });
    let args_json = body.to_string();

    match delete_tag::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('Error:', error.message)`.
            // `render_core_error` strips the dispatcher-only
            // `"Invalid args for fspec command delete-tag: "` envelope
            // so the shell stderr is byte-identical to TS.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
