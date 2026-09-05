//! `remove-foundation-bounded-context` shell-facing CLI bridge (RPC-274).
//!
//! Feature: spec/features/remove-foundation-bounded-context-cli-subcommand.feature
//!
//! Thin clap → fspec_core delegate. Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core core `run`
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → core `run`
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL validation, lookup, soft-removal, cascade, and disk I/O live in the
//! core. The bridge performs JSON arg marshalling only.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_foundation_bounded_context;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub context_name: String,
    pub cascade: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "contextName": args.context_name,
        "cascade": args.cascade,
    })
    .to_string();

    match remove_foundation_bounded_context::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Updated foundation Event Storm");
            println!("✓ {msg}");
            // DISC-003 rule 4/14: print the event-storm trailer.
            crate::common::print_next_steps(&parsed);
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
