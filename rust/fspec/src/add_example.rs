//! `add-example` shell-facing CLI bridge (RPC-181).
//!
//! Feature: spec/features/add-example-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::add_example::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_example::run
//!
//! Both call sites pass a JSON-encoded args shape and `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/add-example.ts:25`). The clap
//! subcommand exposes two positional arguments mirroring the TS
//! Commander.js registration at `src/commands/add-example.ts:98-115`. No
//! domain logic in this bridge — JSON marshalling only.
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to add example:` (parity
//!     with the TS `output.error('✗ Failed to add example:', error.message)`
//!     path at `src/commands/add-example.ts:112`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_example;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub example: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON. We always set both keys (no `None` arms) because
    // the clap variant declares them as required positional arguments.
    let mut obj = serde_json::Map::new();
    obj.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );
    obj.insert("example".to_string(), Value::String(args.example.clone()));
    let args_json = json!(obj).to_string();

    match add_example::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to add example:', error.message)`
            // at src/commands/add-example.ts:112. `render_core_error` strips
            // the dispatcher envelope so we emit only the raw <message>.
            eprintln!("✗ Failed to add example: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
