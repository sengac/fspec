//! `add-persona` shell-facing CLI bridge (RPC-186).
//!
//! Feature: spec/features/add-persona-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddPersona` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_persona::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_persona::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_persona::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL draft probing, placeholder detection, JSON mutation, and disk I/O
//! live in the core. The bridge MUST NOT duplicate any of that logic.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_persona;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub name: String,
    pub description: String,
    pub goals: Vec<String>,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "name": args.name,
        "description": args.description,
        "goals": args.goals,
    })
    .to_string();

    match add_persona::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let file_name = parsed
                .get("fileName")
                .and_then(|v| v.as_str())
                .unwrap_or("foundation.json");
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let description = parsed
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let removed_count = parsed
                .get("removedPlaceholders")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let goals_joined = parsed
                .get("goals")
                .and_then(|v| v.as_array())
                .map(|gs| {
                    gs.iter()
                        .filter_map(|g| g.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            if removed_count > 0 {
                println!("Removed {removed_count} placeholder persona(s)");
            }
            println!("✓ Added persona to {file_name}");
            println!("  Name: {name}");
            println!("  Description: {description}");
            println!("  Goals: {goals_joined}");
            // DISC-003 rule 4/14: print the progress trailer.
            crate::common::print_next_steps(&parsed);
            Ok(0)
        }
        Err(err) => {
            // TS `addPersona` only fails on a missing foundation file. It
            // emits two `output.error` lines:
            //   ✗ foundation.json not found
            //     Run: fspec discover-foundation to create foundation.json
            // then the register-add-persona action prints the rethrown
            // `error.message` ("foundation.json not found") via
            // `output.error(err.message)`. Reproduce all three lines.
            let reason = render_core_error(&err);
            if reason == "foundation.json not found" {
                eprintln!("✗ foundation.json not found");
                eprintln!("  Run: fspec discover-foundation to create foundation.json");
                eprintln!("foundation.json not found");
            } else {
                eprintln!("{reason}");
            }
            Ok(1)
        }
    }
}
