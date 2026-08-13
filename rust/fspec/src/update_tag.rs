//! `update-tag` shell-facing CLI bridge (RPC-316).
//!
//! Feature: spec/features/update-tag-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::UpdateTag` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::update_tag::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::update_tag::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::update_tag::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! NO logic in the bridge — JSON marshalling only.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', ...)` path at
//!     `src/commands/update-tag.ts:151` / `160`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_tag;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// argument set for `update-tag` (`src/commands/update-tag.ts:165-172`).
#[derive(Debug)]
pub struct CliArgs {
    pub tag: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// Entry point invoked from `main.rs` for the `update-tag` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default (`src/commands/update-tag.ts:28`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal positional + named args into the JSON shape that
    // fspec_core::commands::update_tag::run validates with serde.
    // Omit category / description keys entirely when None so the
    // dispatcher gate ("at least one update") fires correctly.
    let mut body = serde_json::Map::new();
    body.insert("tag".to_string(), json!(args.tag));
    if let Some(c) = args.category {
        body.insert("category".to_string(), json!(c));
    }
    if let Some(d) = args.description {
        body.insert("description".to_string(), json!(d));
    }
    let args_json = serde_json::Value::Object(body).to_string();

    match update_tag::run(&args_json, &project_root).await {
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
            // `"Invalid args for fspec command update-tag: "` envelope
            // so the shell stderr is byte-identical to TS.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
