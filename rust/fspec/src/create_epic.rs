//! `create-epic` shell-facing CLI bridge (RPC-211).
//!
//! Feature: spec/features/create-epic-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::CreateEpic` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::create_epic::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253):
//!   - Shell argv         → clap → this module → fspec_core::commands::create_epic::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::create_epic::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/create-epic.ts:27`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path at `src/commands/create-epic.ts:107-109`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::create_epic;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/create-epic.ts:115-127`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub epic_id: String,
    pub title: String,
    pub description: Option<String>,
}

/// Entry point invoked from `main.rs` for the `create-epic` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. TS uses `process.cwd()` as the default.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // create_epic::run. The dispatcher and CLI both feed the SAME serde
    // shape, so adding a field to `CliArgs` automatically threads through
    // to `args_json` without duplication.
    let mut obj = serde_json::Map::new();
    obj.insert("epicId".to_string(), json!(args.epic_id));
    obj.insert("title".to_string(), json!(args.title));
    if let Some(desc) = args.description {
        obj.insert("description".to_string(), json!(desc));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match create_epic::run(&args_json, &project_root).await {
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
            // `"Invalid args for fspec command create-epic: "` envelope.
            // The inner `<reason>` itself carries the outer-catch wrap
            // for paths that the TS source wraps (duplicate-check, I/O
            // failure) but NOT for the id-format validation TS throws
            // before its try block — so the unwrapped reason is the
            // TS-parity payload either way.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
