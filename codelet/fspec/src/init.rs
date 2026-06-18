//! `init` shell-facing CLI bridge (RPC-239).
//!
//! Feature: spec/features/init-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::Init` clap variant in [`crate::main`]) and delegates to
//! the single source-of-truth in
//! [`codelet_fspec_core::commands::init::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::init::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::init::run
//!
//! This bridge embeds NO scaffolding, agent-registry or template logic — its
//! only computation is JSON arg marshalling and stdout/stderr printing. All
//! file writes and per-agent transforms live in fspec_core.
//!
//! Exit-code contract:
//!   - 0 on success; the success summary is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Init failed:`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::init;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Args mirrored from the TS Commander.js registration for `init`: the
/// repeatable `--agent <agent>` option collected into a list.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub agents: Vec<String>,
}

/// Entry point invoked from `main.rs` for the `init` clap subcommand. Returns
/// the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "agent": args.agents }).to_string();

    match init::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let data: Value = serde_json::from_str(&rendered)
                .context("parse init result JSON from fspec_core")?;

            // User cancelled an agent switch (no files installed).
            if data["cancelled"].as_bool() == Some(true) {
                println!("Init cancelled");
                return Ok(0);
            }

            let agent_names = args.agents.join(", ");
            println!("✓ Installed fspec for {agent_names}");

            if let Some(files) = data["filesInstalled"].as_array() {
                for f in files.iter().filter_map(Value::as_str) {
                    println!("  - {f}");
                }
            }

            let activation = data["activationMessage"]
                .as_str()
                .unwrap_or("Run /fspec in your AI agent to activate");
            println!("\nNext steps:\n{activation}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Init failed: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
