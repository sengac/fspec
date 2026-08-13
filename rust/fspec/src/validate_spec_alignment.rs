//! `validate-spec-alignment` shell-facing CLI bridge (RPC-323).
//!
//! Feature: spec/features/validate-spec-alignment-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::validate_spec_alignment::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::validate_spec_alignment::run
//!
//! Framing A (supervisor-approved): the clap surface exposes a REQUIRED
//! positional `<workUnitId>` (mirroring the real exported contract) plus an
//! accepted-but-no-op `--fix` flag. The `--help` text still advertises the
//! broken `[feature-files...]` shape (help doc is canon).
//!
//! This module performs NO scan logic: it only marshals the parsed clap
//! arguments into the JSON arg shape, calls the single fspec-core entry point,
//! and renders the structured `{valid, warnings?}` envelope. All globbing and
//! tag-scanning lives in fspec-core.
//!
//! Exit-code contract:
//!   - valid → stdout `✓ ...`, exit 0.
//!   - invalid → the returned warnings printed to stderr, exit 1.
//!   - error → stderr `Error: <msg>`, exit 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_spec_alignment;
use serde_json::{json, Value};

/// Strongly-typed args. Framing A: required positional `<workUnitId>` plus the
/// accepted-but-no-op `--fix` flag.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub fix: bool,
}

/// Entry point invoked from `main.rs` for the `validate-spec-alignment` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let payload: Value = json!({ "workUnitId": args.work_unit_id, "fix": args.fix });
    let args_json = payload.to_string();

    match validate_spec_alignment::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let value: Value =
                serde_json::from_str(&rendered).context("parse validate-spec-alignment payload")?;
            if value.get("valid").and_then(Value::as_bool).unwrap_or(false) {
                println!("✓ All specs are aligned with tests and implementation");
                Ok(0)
            } else {
                // Print each returned warning verbatim to stderr (the warning
                // text is computed by the core — the bridge does not author it).
                if let Some(warnings) = value.get("warnings").and_then(Value::as_array) {
                    for w in warnings {
                        if let Some(text) = w.as_str() {
                            eprintln!("  - {text}");
                        }
                    }
                }
                Ok(1)
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
