//! `import-example-map` shell-facing CLI bridge (RPC-238).
//!
//! Feature: spec/features/import-example-map-cli-subcommand.feature
//!
//! Two-front-doors pattern (inverse of export-example-map / RPC-228):
//!   - Shell argv         → clap → this module → fspec_core::commands::import_example_map::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::import_example_map::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/import-example-map.ts:33`).
//!
//! Exit-code contract:
//!   - 0 on success; the `✓ Imported …` message is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to import example map:` (parity with
//!     the TS chalk-red error path at `src/commands/import-example-map.ts:129-134`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::import_example_map;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/import-example-map.ts:111-115`): two positional arguments
/// `<workUnitId>` and `<file>`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub file: String,
}

/// Entry point invoked from `main.rs` for the `import-example-map` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".to_string(), json!(args.work_unit_id));
    obj.insert("file".to_string(), json!(args.file));
    let args_json = Value::Object(obj).to_string();

    match import_example_map::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // TS: output.log(chalk.green(`✓ Imported …`)).
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to import example map: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
