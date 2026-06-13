//! `remove-capability` shell-facing CLI bridge (RPC-269).
//!
//! Feature: spec/features/remove-capability-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::RemoveCapability` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_capability::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_capability::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_capability::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL draft probing, matching, JSON mutation, and disk I/O live in the
//! core. The bridge MUST NOT duplicate any of that logic.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_capability;
use serde_json::Value;

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub name: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = serde_json::json!({ "name": args.name }).to_string();

    match remove_capability::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let file_name = parsed
                .get("fileName")
                .and_then(|v| v.as_str())
                .unwrap_or("foundation.json");
            println!(
                "✓ Removed capability \"{}\" from {file_name}",
                args.name
            );
            Ok(0)
        }
        Err(err) => {
            // TS `removeCapability` emits its errors via two `output.error`
            // lines; `register-remove-capability` swallows the rethrown
            // message (`process.exit(1)` only), so no bare message line is
            // printed. The core reason already embeds the indented detail
            // line after a newline (e.g. for the not-found / no-capabilities
            // paths). Re-attach the `✗ ` prefix to the first line.
            //
            // A SILENT_TYPE_ERROR sentinel marks an unguarded JS TypeError
            // path (malformed solution-space / capabilities data) that throws
            // BEFORE any `output.error` call — TS prints nothing and exits 1,
            // so we suppress all output here too.
            let reason = render_core_error(&err);
            if reason.starts_with(remove_capability::SILENT_TYPE_ERROR_PREFIX) {
                return Ok(1);
            }
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
