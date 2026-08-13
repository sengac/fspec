//! `cleanup-checkpoints` shell-facing CLI bridge (RPC-203).
//!
//! Feature: spec/features/cleanup-checkpoints-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::CleanupCheckpoints` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::cleanup_checkpoints::run`]. No list, sort,
//! delete, or rendering logic lives here; the bridge's only computation is
//! `--keep-last` parsing + JSON arg marshalling.
//!
//! `--keep-last` parse note: the option is captured as a raw `String` (not a
//! clap-typed integer) so a non-numeric value (`abc`) surfaces our domain
//! validation message with exit code 1, rather than clap's own exit-code-2
//! value-parse error. A numeric-but-non-positive value (`0`) is forwarded to
//! the core, which owns the `--keep-last must be a positive number` guard.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::cleanup_checkpoints;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/cleanup-checkpoints.ts:108-114`): one positional
/// `<work-unit-id>` and a `--keep-last <N>` option.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw `--keep-last` value (captured as a string so non-numeric input
    /// yields our domain message instead of a clap parse error). Required by
    /// clap, mirroring the TS `requiredOption('--keep-last <number>')`.
    pub keep_last: String,
}

/// Entry point invoked from `main.rs` for the `cleanup-checkpoints` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Parse `--keep-last`. A non-numeric value is rejected with the same
    // message the core uses for a non-positive value, so the CLI surface is
    // consistent regardless of which guard fires. The message is emitted
    // WITHOUT the `Invalid args for fspec command ...` prefix to match the TS
    // `cleanupCheckpointsCommand` catch (`Error: --keep-last must be a positive
    // number`).
    let keep_last: i64 = match args.keep_last.trim().parse::<i64>() {
        Ok(n) if n >= 1 => n,
        _ => {
            eprintln!("Error: --keep-last must be a positive number");
            return Ok(1);
        }
    };

    let args_json = json!({
        "workUnitId": args.work_unit_id,
        "keepLast": keep_last,
    })
    .to_string();

    match cleanup_checkpoints::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Parity with TS `cleanupCheckpoints` util + command catch
            // (`src/commands/cleanup-checkpoints.ts:73-77, 98-104`): the util's
            // `output.error('✗ Failed to cleanup checkpoints: ...')` fires
            // first, then the re-thrown error surfaces via the command's
            // `output.error('Error:', message)`. Both lines route to stderr.
            // The core wraps repository-open failures in `Message(...)` so the
            // raw codelet-git text flows through without a wrapping prefix.
            let msg = err.to_string();
            eprintln!("✗ Failed to cleanup checkpoints: {msg}");
            eprintln!("Error: {msg}");
            Ok(1)
        }
    }
}
