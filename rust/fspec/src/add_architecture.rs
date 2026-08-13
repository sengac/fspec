//! `add-architecture` shell-facing CLI bridge (RPC-167).
//!
//! Feature: spec/features/add-architecture-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::AddArchitecture` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_architecture::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_architecture::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_architecture::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-architecture.ts:25`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered `✓ <message>` line is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the unwrapped reason
//!     is written to stderr prefixed with `Error:` (parity with the TS
//!     `output.error('Error:', error.message)` path collapsed via
//!     [`crate::common::render_core_error`]).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_architecture;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/add-architecture.ts:208-217`.
#[derive(Debug)]
pub struct CliArgs {
    pub feature: String,
    pub text: String,
}

/// Entry point invoked from `main.rs` for the `add-architecture` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let body = json!({
        "feature": args.feature,
        "text": args.text,
    });
    let args_json = body.to_string();

    match add_architecture::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value =
                serde_json::from_str(&data_json).context("parse core response as JSON")?;
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                println!("✓ {msg}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
