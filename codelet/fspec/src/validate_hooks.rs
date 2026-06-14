//! `validate-hooks` shell-facing CLI bridge (RPC-322).
//!
//! Feature: spec/features/validate-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ValidateHooks` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::validate_hooks::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::validate_hooks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::validate_hooks::run
//!
//! ## CLI parity — silent, always exit 0 (TS `registerValidateHooksCommand`)
//! The TS Commander.js `.action` calls `validateHooks(options)` but DISCARDS
//! the returned `{exitCode, valid, errors}`: it never prints anything and
//! never calls `process.exit`. Consequently the TS CLI subcommand produces NO
//! output and ALWAYS exits 0 for every input (missing config, valid hooks,
//! missing scripts, empty hooks, malformed JSON). This bridge mirrors that
//! exactly. The structured envelope from
//! [`codelet_fspec_core::commands::validate_hooks::run`] is still consumed by
//! the LLM-facing dispatcher; only this shell front door is intentionally
//! silent. The core call is retained so its side effects/behaviour match, but
//! its rendered message is deliberately ignored here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::validate_hooks;

/// Strongly-typed args. `validate-hooks` declares no CLI flags (parity with
/// the flag-less TS Commander.js registration), so the JSON shape is `{}`.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `validate-hooks` clap
/// subcommand. Mirrors the TS CLI which prints nothing and always exits 0.
pub async fn run(_args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Invoke the core for behavioural parity, but discard its rendered output:
    // the TS Commander.js `.action` ignores the result entirely (no print, no
    // exit). Any core error is also swallowed to preserve "always exit 0".
    let _ = validate_hooks::run("{}", &project_root).await;

    Ok(0)
}
