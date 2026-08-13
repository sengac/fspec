//! `restore-checkpoint` shell-facing CLI bridge (RPC-288).
//!
//! Feature: spec/features/restore-checkpoint-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::RestoreCheckpoint` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::restore_checkpoint::run`]. No dirty-check,
//! conflict-detection, restore, or rendering logic lives here; the bridge's
//! only computation is JSON arg marshalling + exit-code selection.
//!
//! Exit-code contract (parity with `src/commands/restore-checkpoint.ts:160-190`):
//!   - dirty tree with no user choice → print the re-run hint, exit 1;
//!   - successful restore → exit 0;
//!   - any other failure (e.g. missing checkpoint) → exit 1.
//!
//! The bridge makes two delegate calls: a `format:"json"` call to read the
//! structured `success` / `requiresUserChoice` flags that drive the exit code,
//! and a `format:"text"` call to emit the human-facing output verbatim. The
//! underlying restore is idempotent, so the double call is safe.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::restore_checkpoint;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/restore-checkpoint.ts:193-199`): two positional arguments,
/// `<work-unit-id>` and `<checkpoint-name>`, with no `.option(...)` flags.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub checkpoint_name: String,
}

/// Entry point invoked from `main.rs` for the `restore-checkpoint` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Structured call: read the flags that drive the exit code.
    let structured_args = json!({
        "workUnitId": args.work_unit_id,
        "checkpointName": args.checkpoint_name,
        "format": "json",
    })
    .to_string();

    let (succeeded, requires_user_choice) =
        match restore_checkpoint::run(&structured_args, &project_root).await {
            Ok(rendered) => {
                let v: Value = serde_json::from_str(&rendered).unwrap_or(Value::Null);
                (
                    v.get("success").and_then(Value::as_bool).unwrap_or(false),
                    v.get("requiresUserChoice")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )
            }
            Err(err) => {
                eprintln!("Error: {err}");
                return Ok(1);
            }
        };

    // Text call: emit the human-facing output verbatim from the core.
    let text_args = json!({
        "workUnitId": args.work_unit_id,
        "checkpointName": args.checkpoint_name,
        "format": "text",
    })
    .to_string();

    match restore_checkpoint::run(&text_args, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    }

    if requires_user_choice {
        println!("\nRe-run with user choice to proceed with restoration");
        Ok(1)
    } else if succeeded {
        Ok(0)
    } else {
        Ok(1)
    }
}
