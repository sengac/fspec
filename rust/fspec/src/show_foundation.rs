//! `show-foundation` shell-facing CLI bridge (RPC-305).
//!
//! Feature: spec/features/show-foundation-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core
//! AND LLM tool call JSON → dispatcher → fspec_core. Both paths call the
//! same [`codelet_fspec_core::commands::show_foundation::run`].
//!
//! The TS Commander.js registration at
//! `src/commands/show-foundation.ts:250-269` declares one optional positional
//! `[section]` plus `--section`, `--format`, `--output`, `--draft`,
//! `--list-sections`, and `--line-numbers` flags. The last two are PARSED
//! BUT IGNORED by the TS implementation (no-op parity), so we accept them
//! at the clap surface and drop them before delegation.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_foundation;
use codelet_fspec_core::FspecCoreError;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub section: Option<String>,
    pub format: Option<String>,
    pub output: Option<String>,
    pub draft: bool,
    /// Accepted for parity but ignored at the bridge layer.
    #[allow(dead_code)]
    pub list_sections: bool,
    /// Accepted for parity but ignored at the bridge layer.
    #[allow(dead_code)]
    pub line_numbers: bool,
}

/// Entry point invoked from `main.rs` for the clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    if let Some(s) = args.section.as_deref() {
        obj.insert("section".into(), json!(s));
    }
    if let Some(f) = args.format.as_deref() {
        obj.insert("format".into(), json!(f));
    }
    if let Some(o) = args.output.as_deref() {
        obj.insert("output".into(), json!(o));
    }
    if args.draft {
        obj.insert("draft".into(), json!(true));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match show_foundation::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if let Some(out) = args.output.as_deref() {
                println!("\u{2713} Output written to {out}");
            } else {
                // Parity with TS `output.log(result.output)` which always
                // appends a trailing newline regardless of whether the
                // rendered content already ends with one.
                println!("{rendered}");
            }
            Ok(0)
        }
        Err(err) => {
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("Error: {reason}");
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            Ok(1)
        }
    }
}
