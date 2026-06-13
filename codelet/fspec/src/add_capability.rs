//! `add-capability` shell-facing CLI bridge (RPC-173).
//!
//! Feature: spec/features/add-capability-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddCapability` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_capability::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_capability::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_capability::run
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
use codelet_fspec_core::commands::add_capability;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub name: String,
    pub description: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "name": args.name,
        "description": args.description,
    })
    .to_string();

    match add_capability::run(&args_json, &project_root).await {
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
                .get("removedCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if removed_count > 0 {
                println!("Removed {removed_count} placeholder capability(ies)");
            }
            println!("✓ Added capability to {file_name}");
            println!("  Name: {name}");
            println!("  Description: {description}");
            Ok(0)
        }
        Err(err) => {
            // TS `addCapability` emits its missing-file error via two
            // `output.error` lines, then `register-add-capability` prints the
            // rethrown `err.message` as a third stderr line:
            //   ✗ foundation.json not found
            //     Run: fspec discover-foundation to create foundation.json
            //   foundation.json not found
            // Any other thrown error (e.g. malformed foundation) is printed by
            // the register catch handler as the bare `err.message`.
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
