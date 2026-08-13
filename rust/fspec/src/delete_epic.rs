//! `delete-epic` shell-facing CLI bridge (RPC-217).
//!
//! Feature: spec/features/delete-epic-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::DeleteEpic` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::delete_epic::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253):
//!   - Shell argv         → clap → this module → fspec_core::commands::delete_epic::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::delete_epic::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/delete-epic.ts:41`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to delete epic:` (parity
//!     with the TS chalk-red error path at
//!     `src/commands/delete-epic.ts:106`, where `output.error('✗ Failed
//!     to delete epic:', error.message)` prints the prefix and the
//!     already-wrapped `Failed to delete epic: <inner>` message side by
//!     side, producing the doubled string the TS implementation has
//!     emitted since the file landed).
//!
//! `--force` is accepted on the clap surface for parity with the TS
//! Commander.js registration at `src/commands/delete-epic.ts:97-99`, but
//! the TS implementation never reads its value — neither do we, so the
//! flag is intentionally swallowed without forwarding into the dispatcher
//! args. (Forwarding it would be harmless but distorts the parity story.)

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::delete_epic;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/delete-epic.ts:92-108`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub epic_id: String,
    /// Accepted but ignored — parity-only field.
    pub force: bool,
}

/// Entry point invoked from `main.rs` for the `delete-epic` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. TS uses `process.cwd()` default.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Touch `args.force` to silence dead-code lints without changing
    // behaviour — TS parity dictates the flag is accepted but not
    // forwarded into the dispatcher's args payload.
    let _ = args.force;

    // Marshal the single required field. The dispatcher feeds the SAME
    // serde shape, so any future field additions land in both surfaces
    // without duplication.
    let mut obj = serde_json::Map::new();
    obj.insert("epicId".to_string(), json!(args.epic_id));
    let args_json = serde_json::Value::Object(obj).to_string();

    match delete_epic::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // TS: `output.error('✗ Failed to delete epic:', error.message)`
            // — error.message already contains the outer wrap
            // `"Failed to delete epic: <inner>"`. Mirroring that double
            // wrap preserves byte-parity with `node dist/index.js`.
            eprintln!("✗ Failed to delete epic: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
