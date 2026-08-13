//! `add-dependency` shell-facing CLI bridge (RPC-177).
//!
//! Feature: spec/features/add-dependency-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddDependency` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_dependency::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_dependency::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_dependency::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - shorthand resolution (`dependsOnId` positional → `--depends-on`)
//!   - shorthand/--depends-on conflict pre-check (TS add-dependency.ts:287)
//!   - "at least one relationship" precheck (TS add-dependency.ts:297)
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! Edge-add, status guard, cycle detection, and disk I/O all live in the
//! core. The bridge MUST NOT embed `detect_cycle`, `ensure_work_units_file`,
//! `write_json_atomic`, or any canonical error substring.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_dependency;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/add-dependency.ts:256-281`. All optional except
/// `work_unit_id`; the bridge enforces "at least one relationship"
/// upstream of the core call.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub depends_on_positional: Option<String>,
    pub blocks: Option<String>,
    pub blocked_by: Option<String>,
    pub depends_on: Option<String>,
    pub relates_to: Option<String>,
}

/// Entry point invoked from `main.rs` for the `add-dependency` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Shorthand resolution: positional `dependsOnId` is a sugar for
    // `--depends-on`. Mirrors TS add-dependency.ts:285.
    let final_depends_on: Option<String> = match (&args.depends_on_positional, &args.depends_on) {
        (Some(p), Some(d)) if p != d => {
            eprintln!(
                "✗ Failed to add dependency: Cannot specify dependency both as argument and --depends-on option"
            );
            return Ok(1);
        }
        (Some(p), _) => Some(p.clone()),
        (None, Some(d)) => Some(d.clone()),
        (None, None) => None,
    };

    if final_depends_on.is_none()
        && args.blocks.is_none()
        && args.blocked_by.is_none()
        && args.relates_to.is_none()
    {
        eprintln!(
            "✗ Failed to add dependency: Must specify at least one relationship: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to"
        );
        return Ok(1);
    }

    // Marshal clap args → JSON object expected by the core.
    let mut body = Map::new();
    body.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );
    if let Some(v) = args.blocks {
        body.insert("blocks".to_string(), Value::String(v));
    }
    if let Some(v) = args.blocked_by {
        body.insert("blockedBy".to_string(), Value::String(v));
    }
    if let Some(v) = final_depends_on {
        body.insert("dependsOn".to_string(), Value::String(v));
    }
    if let Some(v) = args.relates_to {
        body.insert("relatesTo".to_string(), Value::String(v));
    }
    let args_json = json!(body).to_string();

    match add_dependency::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            println!("✓ Dependency added successfully");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to add dependency: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
