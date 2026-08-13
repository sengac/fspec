//! `add-question` shell-facing CLI bridge (RPC-188).
//!
//! Feature: spec/features/add-question-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::AddQuestion` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_question::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_question::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_question::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! NO logic in the bridge — JSON marshalling + a fixed success line only
//! (TS parity at `src/commands/add-question.ts:92-99` discards the
//! questionCount / mentionedPeople payload and prints the constant
//! `'✓ Question added successfully'`).
//!
//! Exit-code contract:
//!   - 0 on success; the fixed success line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to add question:`,
//!     matching the TS error path at `src/commands/add-question.ts:97`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_question;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-question.ts:86-100`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub question: String,
}

/// Entry point invoked from `main.rs` for the `add-question` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "workUnitId": args.work_unit_id,
        "question": args.question,
    });
    let args_json = body.to_string();

    match add_question::run(&args_json, &project_root).await {
        Ok(_data) => {
            // TS wrapper discards the result object and prints a fixed
            // line. We mirror that — the JSON shape is reserved for the
            // dispatcher.
            println!("✓ Question added successfully");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add question: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
