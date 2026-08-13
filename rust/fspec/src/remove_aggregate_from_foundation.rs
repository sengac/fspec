//! `remove-aggregate-from-foundation` shell-facing CLI bridge (RPC-266).
//!
//! Feature: spec/features/remove-aggregate-from-foundation-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the clap variant in [`crate::main`]) and delegates to the
//! single source-of-truth in
//! [`codelet_fspec_core::commands::remove_aggregate_from_foundation::run`]
//! — the SAME function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → same run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL validation, the domain-model lookup, the soft-delete mutation, and
//! disk I/O live in the core. The bridge MUST NOT embed any of that logic.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_aggregate_from_foundation;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub context_name: String,
    pub aggregate_name: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "contextName": args.context_name,
        "aggregateName": args.aggregate_name,
    })
    .to_string();

    match remove_aggregate_from_foundation::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Aggregate removed");
            println!("✓ {msg}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
