//! `list-work-units` shell-facing CLI bridge (RPC-253 follow-up).
//!
//! Feature: spec/features/list-work-units-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ListWorkUnits` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_work_units::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_work_units::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_work_units::run
//!
//! Both call sites pass a JSON-encoded `ListWorkUnitsArgs` shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from CWD
//! (parity with the TypeScript `process.cwd()` default) and serialises the
//! clap fields into the same JSON shape `fspec_core` already validates.
//! No filter or rendering logic is duplicated here.
//!
//! Exit-code contract (rules [13] / [14] on RPC-253):
//!   - 0 on success; the rendered text (no ANSI) or 2-space JSON is
//!     written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` (parity with the TS
//!     chalk-red error path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_work_units;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set
/// (`src/commands/list-work-units.ts:126-141`). The fields are decomposed
/// into a clap struct on `Mode::ListWorkUnits` and re-marshalled into JSON
/// here so the shared `fspec_core::commands::list_work_units::run` function
/// can parse them with serde the same way it does for the agent-loop
/// dispatcher.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub status: Option<String>,
    pub prefix: Option<String>,
    pub epic: Option<String>,
    /// Limited at the clap layer to `story` / `task` / `bug`. The CLI does
    /// not own that validation — it is enforced by fspec_core's serde
    /// `WorkUnitType` enum.
    pub r#type: Option<String>,
    /// `"text"` (default) or `"json"`. Passed through verbatim — fspec_core
    /// owns the rendering switch.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `list-work-units` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically. A future `--workspace` flag is out of
    // scope per rule [15] on RPC-253 but the design accommodates it: the
    // value would simply override the CWD result before this point.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core::commands::
    // list_work_units::run validates with serde. We omit unset fields so
    // serde's `#[serde(default)]` arms fire instead of receiving `null`.
    let mut obj = serde_json::Map::new();
    if let Some(v) = args.status.as_ref() {
        obj.insert("status".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.prefix.as_ref() {
        obj.insert("prefix".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.epic.as_ref() {
        obj.insert("epic".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.r#type.as_ref() {
        obj.insert("type".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = args.format.as_ref() {
        obj.insert("format".to_string(), Value::String(v.clone()));
    }
    let args_json = json!(obj).to_string();

    // Delegate to the single source of truth. fspec_core handles ensure-files
    // creation, filter chain, text vs JSON rendering, and ParseJson errors.
    match list_work_units::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text / json both go to stdout. The text path embeds its own
            // trailing newline structure; print as-is and avoid a duplicate
            // \n that would shift the "Work Units (N)" header.
            print!("{rendered}");
            // For the JSON path serde_json::to_string_pretty omits a
            // trailing newline — append one so shell pipelines (e.g.
            // `| jq`) and human readers see a properly-terminated line.
            // Detection: if the rendered output does not already end in
            // a newline, append one.
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required for parity with rule [14].
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
