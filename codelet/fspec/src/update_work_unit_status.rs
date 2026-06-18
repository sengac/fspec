//! `update-work-unit-status` shell-facing CLI bridge (RPC-319).
//!
//! Feature: spec/features/update-work-unit-status-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::UpdateWorkUnitStatus` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::update_work_unit_status::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::update_work_unit_status::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::update_work_unit_status::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered confirmation text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to update work unit status:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_work_unit_status;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration for
/// `update-work-unit-status`: positional `<workUnitId> <status>` plus the
/// `--blocked-reason <reason>` and `--skip-temporal-validation` options.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub status: String,
    pub blocked_reason: Option<String>,
    pub reason: Option<String>,
    pub skip_temporal_validation: bool,
}

/// Entry point invoked from `main.rs` for the `update-work-unit-status` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut body = serde_json::Map::new();
    body.insert("workUnitId".to_string(), json!(args.work_unit_id));
    body.insert("status".to_string(), json!(args.status));
    if let Some(reason) = args.blocked_reason {
        body.insert("blockedReason".to_string(), json!(reason));
    }
    if let Some(reason) = args.reason {
        body.insert("reason".to_string(), json!(reason));
    }
    body.insert(
        "skipTemporalValidation".to_string(),
        json!(args.skip_temporal_validation),
    );
    let args_json = json!(body).to_string();

    match update_work_unit_status::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to update work unit status: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
