//! `audit` shell-facing CLI bridge (AUDIT-003).
//!
//! Feature: spec/features/fspec-audit-is-the-one-deterministic-drift-check.feature
//!
//! Two front doors: shell argv → clap → this module, and LLM tool calls →
//! `dispatch_command`, both reach `codelet_fspec_core::commands::audit::run`.
//! This bridge only marshals the arguments, prints the report and returns
//! its exit code: 0 no problems, 1 problems, 2 an error. AUDIT-004: the
//! session commands print their step text the same way (exit 1 when
//! `--continue` finds the step's problem still reported).

use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::audit;
use serde_json::{json, Value};

use crate::common::{render_core_error, spec_project_root};

/// Arguments of `fspec audit`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub area: Vec<String>,
    pub feature: Vec<String>,
    pub work_unit: Option<String>,
    pub json: bool,
    pub fix: bool,
    pub changed: bool,
    /// AUDIT-004: the audit session commands.
    pub start: bool,
    pub status: bool,
    pub r#continue: bool,
    pub reset: bool,
    pub complete: bool,
}

/// Entry point for the clap subcommand; returns the process exit code.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;
    let mut payload = json!({
        "area": args.area,
        "feature": args.feature,
        "json": args.json,
        "fix": args.fix,
        "changed": args.changed,
        "start": args.start,
        "status": args.status,
        "continue": args.r#continue,
        "reset": args.reset,
        "complete": args.complete,
    });
    if let Some(work_unit) = args.work_unit {
        payload["workUnitId"] = json!(work_unit);
    }
    match audit::run(&payload.to_string(), &project_root).await {
        Ok(envelope) => {
            let v: Value = serde_json::from_str(&envelope).context("parse audit JSON payload")?;
            println!("{}", v.get("output").and_then(Value::as_str).unwrap_or(""));
            let code = v.get("exitCode").and_then(Value::as_u64).unwrap_or(2);
            Ok(u8::try_from(code).unwrap_or(2))
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(2)
        }
    }
}
