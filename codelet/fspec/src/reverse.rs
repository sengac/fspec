//! `reverse` shell-facing CLI bridge (RPC-294).
//!
//! Feature: spec/features/reverse-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::reverse::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::reverse::run
//!
//! This module performs ZERO domain logic — no gap analysis, no strategy
//! suggestion, no session rendering. Its only computation is marshalling the
//! six clap flags into the JSON args shape that the single source-of-truth
//! [`codelet_fspec_core::commands::reverse::run`] validates. (The
//! `cli_and_dispatcher_converge` contract test asserts this file embeds none
//! of the rendering literals.)
//!
//! The TS Commander surface (`src/commands/reverse.ts:669-687`) exposes exactly
//! six flags — `--strategy <A|B|C|D>`, `--continue`, `--status`, `--reset`,
//! `--complete`, `--dry-run`. It does NOT expose `implementationContext`
//! (the Strategy-D persona path), so neither does this bridge — that field is
//! reachable only through the structured dispatcher path (parity confirmed).
//!
//! Exit-code contract (parity with the TS `reverseCommand` wrapper,
//! `reverse.ts:629-664`):
//!   - The TS wrapper prints `systemReminder`, `message`, `guidance`, and
//!     `suggestions` to STDOUT, then `process.exit(result.exitCode || 0)`.
//!     Soft failures (no active session, existing session detected, cannot
//!     complete) carry `exitCode: 1` but still print their content to STDOUT.
//!     The Rust core surfaces these as [`FspecCoreError::Message`], so this
//!     bridge prints the message body to STDOUT and exits 1.
//!   - Genuine errors (invalid args, I/O) surface as other
//!     [`FspecCoreError`] variants; this bridge prints `Error: <reason>` to
//!     STDERR and exits 1 (parity with the TS catch → `output.error`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::reverse;
use codelet_fspec_core::FspecCoreError;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/reverse.ts:669-687`. `implementationContext` is intentionally
/// absent — it is dispatcher-JSON-only.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--strategy <A|B|C|D>`.
    pub strategy: Option<String>,
    /// `--continue`.
    pub r#continue: bool,
    /// `--status`.
    pub status: bool,
    /// `--reset`.
    pub reset: bool,
    /// `--complete`.
    pub complete: bool,
    /// `--dry-run`.
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `reverse` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with the TS `process.cwd()`
    // default at `reverse.ts:29`). The core resolves the session-file path
    // from a boundary-marker walk of this root.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal the six flags into the JSON args shape. Booleans are only
    // emitted when set so an unset flag relies on the core's `#[serde(default)]`
    // arms (`false` / `None`). `strategy` is emitted only when present.
    let mut obj = serde_json::Map::new();
    if let Some(strategy) = &args.strategy {
        obj.insert("strategy".to_string(), Value::String(strategy.clone()));
    }
    if args.r#continue {
        obj.insert("continue".to_string(), Value::Bool(true));
    }
    if args.status {
        obj.insert("status".to_string(), Value::Bool(true));
    }
    if args.reset {
        obj.insert("reset".to_string(), Value::Bool(true));
    }
    if args.complete {
        obj.insert("complete".to_string(), Value::Bool(true));
    }
    if args.dry_run {
        obj.insert("dryRun".to_string(), Value::Bool(true));
    }
    let args_json = json!(obj).to_string();

    match reverse::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // An EMPTY body (e.g. `--status` on an active session, where the
            // TS CLI wrapper logs none of the structured status fields) must
            // print NOTHING — not even a trailing newline (TS emits 0 bytes).
            if !rendered.is_empty() {
                print!("{rendered}");
                if !rendered.ends_with('\n') {
                    println!();
                }
            }
            Ok(0)
        }
        // Soft failures (exitCode 1 in TS, but content on STDOUT): the core
        // carries the fully-rendered body (message + suggestions) in the
        // `Message` variant. Print it to STDOUT and exit 1.
        Err(FspecCoreError::Message(body)) => {
            print!("{body}");
            if !body.ends_with('\n') {
                println!();
            }
            Ok(1)
        }
        // Genuine errors: stderr, prefixed, exit 1 (parity with the TS catch).
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
