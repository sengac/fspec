//! `fspec --sync-version <version>` shell-facing bridge (BUG-285).
//!
//! Feature: spec/features/automatic-version-check-and-update-for-slash-command-files.feature
//!
//! The /fspec slash command runs this hidden top-level flag (no subcommand)
//! as its first step. This bridge only marshals the JSON args and prints; the
//! version comparison, agent detection and file reinstall live in
//! [`codelet_fspec_core::commands::sync_version::run`].
//!
//! Exit-code contract (parity with `src/commands/sync-version.ts`):
//!   - the `exitCode` fspec_core returns (0 continue, 1 stop the workflow),
//!     after printing its non-empty `output` to stdout;
//!   - 0 when the check itself fails (warning on stderr) so a broken check
//!     never blocks the agent.

use crate::common::spec_project_root;
use std::process::ExitCode;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::sync_version;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Entry point invoked from `main` when `--sync-version` is present.
pub async fn exit_code(embedded_version: &str) -> ExitCode {
    match run(embedded_version).await {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("⚠️  Version check failed: {err:#}");
            ExitCode::SUCCESS
        }
    }
}

async fn run(embedded_version: &str) -> Result<u8> {
    let project_root = spec_project_root()?;
    let args_json = json!({ "embeddedVersion": embedded_version }).to_string();

    match sync_version::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let data: Value = serde_json::from_str(&rendered)
                .context("parse sync-version result JSON from fspec_core")?;
            if let Some(output) = data["output"].as_str().filter(|o| !o.is_empty()) {
                println!("{output}");
            }
            Ok(data["exitCode"]
                .as_u64()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1))
        }
        Err(err) => {
            eprintln!("⚠️  Version check failed: {}", render_core_error(&err));
            Ok(0)
        }
    }
}
