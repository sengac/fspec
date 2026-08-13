//! `set-user-story` shell-facing CLI bridge (RPC-298).
//!
//! Feature: spec/features/set-user-story-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::SetUserStory` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::set_user_story::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::set_user_story::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::set_user_story::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/set-user-story.ts:20`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered four-line success block (no ANSI) is
//!     written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', error.message)` path at
//!     `src/commands/set-user-story.ts:59-62`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::set_user_story;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/set-user-story.ts:65-80`.
#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub role: String,
    pub action: String,
    pub benefit: String,
}

/// Entry point invoked from `main.rs` for the `set-user-story` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // set_user_story::run. The dispatcher and CLI both feed the SAME serde
    // shape, so adding a field to `CliArgs` automatically threads through
    // to `args_json` without duplication.
    let body = json!({
        "workUnitId": args.work_unit_id,
        "role": args.role,
        "action": args.action,
        "benefit": args.benefit,
    });
    let args_json = body.to_string();

    match set_user_story::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns the canonical four-line success block. Print
            // it verbatim — the trailing newline is already in the rendered
            // string.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
