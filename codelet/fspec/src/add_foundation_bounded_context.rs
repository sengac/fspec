//! `add-foundation-bounded-context` shell-facing CLI bridge (RPC-183).
//!
//! Feature: spec/features/add-foundation-bounded-context-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the clap variant in [`crate::main`]) and delegates to the
//! single source-of-truth `run` in fspec_core — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core core `run`
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → core `run`
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL validation, JSON mutation, item construction, ID seeding, and disk
//! I/O live in the core. The bridge MUST NOT embed any of that logic.
//!
//! NOTE: the core module is imported under the alias `add_foundation_ctx`
//! (re-exported from `fspec_core::commands`) so this façade contains no
//! domain identifiers — see the source-grep guard in the CLI test.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_foundation_ctx;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub text: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "text": args.text }).to_string();

    match add_foundation_ctx::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Updated foundation Event Storm");
            println!("✓ {msg}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
