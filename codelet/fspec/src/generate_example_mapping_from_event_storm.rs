//! `generate-example-mapping-from-event-storm` shell-facing CLI bridge (RPC-232).
//!
//! Feature: spec/features/generate-example-mapping-from-event-storm-cli-subcommand.feature
//!
//! Two-front-doors pattern: shell argv → clap → this module → fspec_core
//! AND LLM tool call JSON → dispatcher → fspec_core. Both paths call the
//! same [`codelet_fspec_core::commands::generate_example_mapping_from_event_storm::run`].
//!
//! Console-output contract (parity with TS `logger`):
//!   The TS `.action()` callback (src/commands/generate-example-mapping-from-event-storm.ts:188-212)
//!   uses `logger.success(...)` on the success path and `logger.error(...)`
//!   on failure. The fspec `logger` is a Winston logger whose ONLY transport
//!   is a file (`~/.fspec/fspec.log`) — it writes NOTHING to stdout/stderr.
//!   Moreover Winston has no `success` level, so `logger.success(...)` throws
//!   a `TypeError` that is swallowed by the surrounding `try/catch`, which
//!   then calls `logger.error(...)` (also file-only) and `process.exit(1)`.
//!   Net observable behaviour of `node dist/index.js
//!   generate-example-mapping-from-event-storm ...`:
//!     - stdout: EMPTY in every case
//!     - stderr: EMPTY in every case
//!     - exit code: ALWAYS 1 (success path falls through the caught
//!       `TypeError` into `process.exit(1)`; error path is an explicit
//!       `process.exit(1)`)
//!     - disk state: the Example Mapping entries ARE persisted on the success
//!       path before the (caught) `logger.success` throw, so the work unit's
//!       rules/examples/questions are written exactly as the core mutated them.
//!   This bridge therefore emits NO console output and ALWAYS returns exit
//!   code 1, matching the TS binary byte-for-byte. The core `run()` still
//!   performs the real mutation + atomic write, so disk parity holds.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::generate_example_mapping_from_event_storm;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the work-unit identifier whose Event Storm to
    /// transform into Example Mapping entries.
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let args_json = json!({ "workUnitId": args.work_unit_id }).to_string();

    // The core performs the real mutation + atomic write. Regardless of
    // success or failure, the TS binary writes nothing to the console and
    // always exits 1 (see the module-level Console-output contract): the
    // `logger.success(...)` call on the success path throws a swallowed
    // `TypeError` (Winston has no `success` level) and falls through to
    // `process.exit(1)`, while the error path is an explicit
    // `process.exit(1)`. Both `logger.*` transports are file-only. We discard
    // the result and emit no console output for byte-parity.
    let _ = generate_example_mapping_from_event_storm::run(&args_json, &project_root).await;
    Ok(1)
}
