//! `compare-implementations` shell-facing CLI bridge (RPC-207).
//!
//! Feature: spec/features/compare-implementations-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default), marshals the
//! `--tag` / `--show-coverage` / `--json` flags into JSON, and delegates to
//! the single source-of-truth in
//! [`codelet_fspec_core::commands::compare_implementations::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Rendering parity with `compareImplementations` action at
//! `src/commands/compare-implementations.ts:104-128`:
//!   - `--json` → pretty-printed (2-space) JSON envelope to stdout, exit 0.
//!   - otherwise → green `✓ Compared N work units tagged with <tag>`, exit 0.
//!   - any core error → `✗ Comparison failed: <message>` to stderr, exit 1.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::compare_implementations;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/compare-implementations.ts:89-104`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--tag <tag>` (required).
    pub tag: String,
    /// `--show-coverage` flag.
    pub show_coverage: bool,
    /// `--json` flag.
    pub json: bool,
}

/// Entry point invoked from `main.rs` for the `compare-implementations` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj: Map<String, Value> = Map::new();
    obj.insert("tag".to_string(), Value::String(args.tag.clone()));
    if args.show_coverage {
        obj.insert("showCoverage".to_string(), Value::Bool(true));
    }
    if args.json {
        obj.insert("json".to_string(), Value::Bool(true));
    }
    let args_json = json!(obj).to_string();

    match compare_implementations::run(&args_json, &project_root).await {
        Ok(payload) => {
            if args.json {
                // Re-serialise with 2-space indent (parity with
                // JSON.stringify(result, null, 2)).
                let value: Value =
                    serde_json::from_str(&payload).context("parse core JSON payload")?;
                let pretty =
                    serde_json::to_string_pretty(&value).context("pretty-print compare result")?;
                println!("{pretty}");
            } else {
                let value: Value =
                    serde_json::from_str(&payload).context("parse core JSON payload")?;
                let count = value
                    .get("workUnits")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                println!("✓ Compared {count} work units tagged with {}", args.tag);
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Comparison failed: {err}");
            Ok(1)
        }
    }
}
