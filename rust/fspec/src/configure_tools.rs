//! `configure-tools` shell-facing CLI bridge (RPC-208).
//!
//! Feature: spec/features/configure-tools-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ConfigureTools` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::configure_tools::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::configure_tools::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::configure_tools::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering
//!
//! ALL config-merge, the reconfigure short-circuit, and disk I/O live in the
//! core. The bridge MUST NOT duplicate any of that logic — it prints whatever
//! confirmation/guidance `message` the core returns verbatim, so both the
//! "configuration saved" line and the reconfigure guidance flow through one
//! code path without the bridge embedding either literal.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::configure_tools;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub test_command: Option<String>,
    pub quality_commands: Option<Vec<String>>,
    pub reconfigure: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut body = serde_json::Map::new();
    if let Some(tc) = args.test_command {
        body.insert("testCommand".to_string(), Value::String(tc));
    }
    if let Some(qc) = args.quality_commands {
        body.insert(
            "qualityCommands".to_string(),
            Value::Array(qc.into_iter().map(Value::String).collect()),
        );
    }
    if args.reconfigure {
        body.insert("reconfigure".to_string(), Value::Bool(true));
    }
    let args_json = json!(body).to_string();

    match configure_tools::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            // TS-parity (configure-tools.ts:225-227): the Commander.js action
            // DISCARDS the configureTools() return value and only emits the
            // saved-confirmation line when NOT in --reconfigure mode. The
            // reconfigure guidance the core returns is therefore NEVER printed
            // by the CLI front door — it surfaces only via the LLM-facing
            // dispatcher. So suppress all stdout on the reconfigure path here.
            let reconfigure = parsed
                .get("reconfigure")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !reconfigure {
                let message = parsed
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                println!("{message}");
            }
            Ok(0)
        }
        Err(err) => {
            let reason = render_core_error(&err);
            eprintln!("Error: {reason}");
            Ok(1)
        }
    }
}
