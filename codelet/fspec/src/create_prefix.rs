//! `create-prefix` shell-facing CLI bridge (RPC-213).
//!
//! Feature: spec/features/create-prefix-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::CreatePrefix` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::create_prefix::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::create_prefix::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::create_prefix::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TypeScript
//! `process.cwd()` default at `src/commands/create-prefix.ts:26`).
//!
//! Exit-code contract:
//!   - 0 on success; the canonical line `✓ Prefix <X> created successfully`
//!     is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to create prefix:`
//!     (parity with the TS `output.error('✗ Failed to create prefix:',
//!     err.message)` path at `src/commands/create-prefix.ts:83`). The
//!     dispatcher-only `"Invalid args for fspec command create-prefix: "`
//!     envelope is stripped via [`crate::common::render_core_error`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::create_prefix;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// positional set for `create-prefix`
/// (`src/commands/create-prefix.ts:66-86`). NO `.option(...)` calls are
/// declared in TS, so the surface is just two required strings.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub prefix: String,
    pub description: String,
}

/// Entry point invoked from `main.rs` for the `create-prefix` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core::commands::
    // create_prefix::run validates with serde. The marshalling lives here
    // (rather than a hard-coded literal) so adding a field to `CliArgs`
    // automatically threads through to `args_json`.
    let mut obj = serde_json::Map::new();
    obj.insert("prefix".to_string(), Value::String(args.prefix.clone()));
    obj.insert(
        "description".to_string(),
        Value::String(args.description.clone()),
    );
    let args_json = json!(obj).to_string();

    match create_prefix::run(&args_json, &project_root).await {
        Ok(_data) => {
            // Mirror the TS success line at src/commands/create-prefix.ts:81
            // (`output.log('✓ Prefix ${prefix} created successfully')`).
            // The dispatcher JSON body is intentionally ignored here —
            // the CLI surface promises a single-line success message,
            // not a structured payload.
            println!("✓ Prefix {} created successfully", args.prefix);
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to create prefix:', err.message)`.
            // The core error reason already carries the outer
            // `"Failed to create prefix: "` wrap for paths the TS
            // outer try/catch produces (lines 49-53 of the TS source),
            // so this prefix combines with that wrap to reproduce the
            // doubled string the TS implementation has emitted since
            // the file landed — exactly matching `node dist/index.js`.
            eprintln!("✗ Failed to create prefix: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
