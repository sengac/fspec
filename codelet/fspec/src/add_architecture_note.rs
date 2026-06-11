//! `add-architecture-note` shell-facing CLI bridge (RPC-168).
//!
//! Feature: spec/features/add-architecture-note-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_architecture_note::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_architecture_note::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-architecture-note.ts:24`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path at `src/commands/add-architecture-note.ts:102-107`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_architecture_note;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-architecture-note.ts:89-108`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub note: String,
}

/// Entry point invoked from `main.rs` for the `add-architecture-note` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by
    // fspec_core::commands::add_architecture_note::run.
    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".to_string(), json!(args.work_unit_id));
    obj.insert("note".to_string(), json!(args.note));
    let args_json = serde_json::Value::Object(obj).to_string();

    match add_architecture_note::run(&args_json, &project_root).await {
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
            // `"Invalid args for fspec command add-architecture-note: "`
            // envelope so the shell user sees the same payload as the TS
            // CLI does.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
