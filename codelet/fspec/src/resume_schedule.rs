//! `resume-schedule` shell-facing CLI bridge (RPC-292).
//!
//! Feature: spec/features/resume-schedule-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ResumeSchedule` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::resume_schedule::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::resume_schedule::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::resume_schedule::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/schedule/resume-schedule.ts:24`).
//!
//! Exit-code contract:
//!   - 0 on success; the canonical success line RETURNED by fspec_core (its
//!     `message` field — the single source of truth) is written verbatim to
//!     stdout. The bridge never re-authors that text (parity with the TS
//!     `output.log(...)` path at `resume-schedule.ts:84`).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to resume schedule:` (parity with the
//!     TS `output.error('✗ Failed to resume schedule:', err.message)` path at
//!     `resume-schedule.ts:86`). The dispatcher-only
//!     `"Invalid args for fspec command resume-schedule: "` envelope is stripped
//!     via [`crate::common::render_core_error`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::resume_schedule;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js positional set for
/// `resume-schedule` (`src/commands/schedule/resume-schedule.ts:78-83`). NO
/// `.option(...)` calls are declared, so the surface is a single required
/// schedule `name`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub name: String,
}

/// Entry point invoked from `main.rs` for the `resume-schedule` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal the JSON args shape fspec_core::commands::resume_schedule::run
    // validates with serde. The marshalling lives here (not a hard-coded
    // literal) so future flag additions thread through automatically.
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), Value::String(args.name.clone()));
    let args_json = json!(obj).to_string();

    match resume_schedule::run(&args_json, &project_root).await {
        Ok(data) => {
            // `data` is the JSON result fspec_core authored; its `message`
            // field is the single source-of-truth success line. We print
            // exactly what core RETURNS — the bridge never re-creates it.
            let parsed: Value =
                serde_json::from_str(&data).context("parse fspec_core resume-schedule result")?;
            if let Some(msg) = parsed.get("message").and_then(Value::as_str) {
                println!("{msg}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to resume schedule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
