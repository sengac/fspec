//! `remove-persona` shell-facing CLI bridge (RPC-277).
//!
//! Feature: spec/features/remove-persona-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemovePersona` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_persona::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_persona::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_persona::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL draft probing, name matching, JSON mutation, and disk I/O live in the
//! core. The bridge MUST NOT duplicate any of that logic.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_persona;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub name: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "name": args.name,
    })
    .to_string();

    match remove_persona::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let file_name = parsed
                .get("fileName")
                .and_then(|v| v.as_str())
                .unwrap_or("foundation.json");
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");

            println!("✓ Removed persona \"{name}\" from {file_name}");
            // DISC-003 rule 4/14: print the progress trailer.
            crate::common::print_next_steps(&parsed);
            Ok(0)
        }
        Err(err) => {
            // TS `removePersona` emits its errors via two `output.error`
            // lines. The first line is prefixed with `✗ `; the second is the
            // indented detail line (already embedded in the core reason after
            // the first newline). For the missing-file path TS emits:
            //   ✗ foundation.json not found
            //     Run: fspec discover-foundation to create foundation.json
            // `register-remove-persona` swallows the rethrown message
            // (`process.exit(1)` only), so no bare message line is printed.
            let reason = render_core_error(&err);
            if reason == "foundation.json not found" {
                eprintln!("✗ foundation.json not found");
                eprintln!("  Run: fspec discover-foundation to create foundation.json");
            } else {
                eprintln!("✗ {reason}");
            }
            Ok(1)
        }
    }
}
