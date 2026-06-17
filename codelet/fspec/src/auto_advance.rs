//! `auto-advance` shell-facing CLI bridge (RPC-198).
//!
//! Feature: spec/features/auto-advance-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::auto_advance::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::auto_advance::run
//!
//! ## Framing A — the broken TS shell, mirrored verbatim
//!
//! The TypeScript Commander action (`src/commands/auto-advance.ts:116-134`)
//! wires only the `--dry-run` flag and calls `autoAdvance({ dryRun })` — it
//! NEVER passes a work-unit id, a `from` state, or an `event`. The function
//! then reads an undefined work-unit key, which is always missing, so it
//! throws `Work unit undefined not found`, the catch re-wraps it, and the
//! shell exits 1. This Rust bridge reproduces that broken behaviour: it
//! marshals an EMPTY args object (ignoring `--dry-run`) so the core's Framing
//! A path deterministically surfaces the same error and exit code.
//!
//! Exit-code contract:
//!   - 0 never reached on the happy path (the shell is broken by design).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `✗ Failed to auto-advance:` mirroring the TS
//!     `output.error('✗ Failed to auto-advance:', error.message)` path.
//!
//! This module performs ZERO domain logic — it is pure JSON arg marshalling
//! before delegating to the single source-of-truth core function.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::auto_advance;
use serde_json::json;

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/auto-advance.ts:116-134`: only the `--dry-run` flag is
/// parsed — and, exactly as the TS shell does, it is NOT forwarded to the core
/// (Framing A).
#[derive(Debug, Default)]
pub struct CliArgs {
    pub dry_run: bool,
}

/// Entry point invoked from `main.rs` for the `auto-advance` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Framing A: deliberately DO NOT thread `dry_run` into the dispatcher
    // args. The marshalled payload carries no work-unit id, so the core falls
    // through to the canonical `Work unit undefined not found` error —
    // byte-parity with the broken TS Commander shell.
    let _ = args.dry_run;
    let args_json = json!({}).to_string();

    match auto_advance::run(&args_json, &project_root).await {
        Ok(_data_json) => {
            // Unreachable on the broken-shell path, kept for symmetry.
            println!("✓ Advanced work units");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to auto-advance: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
