//! `create-feature` shell-facing CLI bridge (RPC-212).
//!
//! Feature: spec/features/create-feature-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::create_feature::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::create_feature::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/create-feature.ts:35`).
//! No domain logic in this bridge — JSON marshalling + result-field printing
//! only.
//!
//! Exit-code contract (parity with TS at
//! `src/commands/create-feature.ts:132-169`):
//!   - 0 on success; prints `✓ Created <spec/features/file>` + edit hint,
//!     the coverage-file message, an optional file-naming reminder, and an
//!     optional prefill `<system-reminder>` (all on stdout).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the unwrapped reason
//!     is written to stderr prefixed with `Error: `.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::create_feature;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/create-feature.ts:172-177`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub name: String,
}

/// Entry point invoked from `main.rs` for the `create-feature` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let body = json!({ "name": args.name });
    let args_json = body.to_string();

    match create_feature::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value =
                serde_json::from_str(&data_json).context("parse core response as JSON")?;

            // ✓ Created spec/features/<file> — derive the trailing two path
            // segments (spec/features/<file>) from the absolute filePath.
            if let Some(file_path) = parsed.get("filePath").and_then(Value::as_str) {
                let short = short_path(file_path);
                println!("✓ Created {short}");
                println!("  Edit the file to add your scenarios");
            }

            // Coverage-file status message (always present on success).
            if let Some(msg) = parsed
                .get("coverageFile")
                .and_then(|c| c.get("message"))
                .and_then(Value::as_str)
            {
                println!("{msg}");
            }

            // File-naming anti-pattern reminder, if present.
            if let Some(reminder) = parsed.get("fileNamingReminder").and_then(Value::as_str) {
                println!("\n{reminder}");
            }

            // Prefill detection system-reminder, if present.
            if let Some(rem) = parsed
                .get("prefillDetection")
                .and_then(|p| p.get("systemReminder"))
                .and_then(Value::as_str)
            {
                println!("\n{rem}");
            }

            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Return the trailing `spec/features/<file>` slice of an absolute feature
/// path (parity with the TS `result.filePath.split('/').slice(-2).join('/')`).
fn short_path(file_path: &str) -> String {
    let parts: Vec<&str> = file_path.split('/').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join("/")
    } else {
        file_path.to_string()
    }
}
