//! `add-schedule` shell-facing CLI bridge (RPC-191).
//!
//! Feature: spec/features/add-schedule-rust-port.feature
//!          spec/features/add-schedule-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses argv
//! (the `Mode::AddSchedule` clap variant in [`crate::main`]) and delegates to
//! the single source-of-truth in
//! [`codelet_fspec_core::commands::add_schedule::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_schedule::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_schedule::run
//!
//! ALL validation, schedule-construction, and file-writing logic lives in
//! `fspec_core`. This module performs JSON arg marshalling + delegation ONLY
//! (enforced by the cli_add_schedule.rs thin-bridge guard).
//!
//! The clap subcommand mirrors the TS Commander.js flag set at
//! `src/commands/schedule/add-schedule.ts:141-152`: `-n/--name`, `-c/--cron`,
//! `-z/--timezone`, `-t/--type`, `-r/--role`, `-p/--prompt`, `--command`, and
//! `-o/--overlap` (default `skip`). The TS `--type` flag maps to the
//! dispatcher's `jobType` key.
//!
//! Exit-code contract: 0 on success (TS-parity confirmation printed to stdout);
//! 1 on any [`codelet_fspec_core::FspecCoreError`] (message to stderr prefixed
//! with `Error:`, parity with the TS chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_schedule;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js flag set for
/// `add-schedule`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub name: String,
    pub cron: String,
    pub timezone: String,
    /// TS `--type` (maps to the dispatcher `jobType` key).
    pub job_type: String,
    pub role: Option<String>,
    pub prompt: Option<String>,
    pub command: Option<String>,
    /// TS `-o/--overlap`, default `skip`.
    pub overlap: String,
}

/// Entry point invoked from `main.rs` for the `add-schedule` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON object expected by fspec_core. The TS `--type`
    // surface becomes the `jobType` key; `-o/--overlap` becomes `overlapPolicy`.
    let mut obj = Map::new();
    obj.insert("name".to_string(), json!(args.name));
    obj.insert("cron".to_string(), json!(args.cron));
    obj.insert("timezone".to_string(), json!(args.timezone));
    obj.insert("jobType".to_string(), json!(args.job_type));
    obj.insert("overlapPolicy".to_string(), json!(args.overlap));
    if let Some(role) = &args.role {
        obj.insert("role".to_string(), json!(role));
    }
    if let Some(prompt) = &args.prompt {
        obj.insert("prompt".to_string(), json!(prompt));
    }
    if let Some(c) = &args.command {
        obj.insert("command".to_string(), json!(c));
    }
    let args_json = Value::Object(obj).to_string();

    match add_schedule::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Parse the structured result to surface the TS-parity confirmation
            // lines (the bridge does no domain logic — it merely formats the
            // success acknowledgement from the value fspec_core returned).
            let parsed: Value =
                serde_json::from_str(&rendered).context("parse add-schedule result payload")?;
            let entry = parsed.get("schedule").cloned().unwrap_or(Value::Null);
            let job_type = entry
                .get("jobType")
                .and_then(Value::as_str)
                .unwrap_or(&args.job_type);
            let cron = entry.get("cron").and_then(Value::as_str).unwrap_or(&args.cron);
            let tz = entry
                .get("timezone")
                .and_then(Value::as_str)
                .unwrap_or(&args.timezone);
            println!("✓ Schedule '{}' added successfully", args.name);
            println!("  Type: {job_type}");
            println!("  Cron: {cron}");
            println!("  Timezone: {tz}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add schedule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
