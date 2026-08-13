//! `show-test-patterns` shell-facing CLI bridge (RPC-307).
//!
//! Feature: spec/features/show-test-patterns-cli-subcommand.feature
//!
//! Thin façade: marshal CLI args into JSON, delegate to
//! `fspec_core::commands::show_test_patterns::run`, then:
//!   * `--json` → print the rendered JSON envelope verbatim
//!   * default → print the `message` summary line
//!
//! This bridge intentionally contains NO inline filtering, coverage
//! reading, or summary formatting — those live exclusively in fspec_core.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_test_patterns;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js flag set.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub tag: String,
    pub include_coverage: bool,
    pub json: bool,
}

/// Entry point for the `show-test-patterns` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut payload: Map<String, Value> = Map::new();
    payload.insert("tag".to_string(), Value::String(args.tag.clone()));
    payload.insert(
        "includeCoverage".to_string(),
        Value::Bool(args.include_coverage),
    );
    payload.insert("json".to_string(), Value::Bool(args.json));
    let args_json = json!(payload).to_string();

    match show_test_patterns::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if args.json {
                println!("{rendered}");
            } else {
                // Core emits a `message` field for the table-format path
                // (see commands/show_test_patterns.rs). The bridge simply
                // surfaces it — no inline rendering of work-unit counts
                // or tags lives here.
                let envelope: Value =
                    serde_json::from_str(&rendered).context("parse fspec_core envelope")?;
                let msg = envelope
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                println!("{msg}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
