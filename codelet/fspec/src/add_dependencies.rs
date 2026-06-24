//! `add-dependencies` shell-facing CLI bridge (RPC-176).
//!
//! Feature: spec/features/add-dependencies-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AddDependencies` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::add_dependencies::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::add_dependencies::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::add_dependencies::run
//!
//! Both call sites pass the canonical JSON args shape
//! `{workUnitId, dependencies: {blocks, blockedBy, dependsOn, relatesTo}}`
//! and a `project_root: &Path`. No domain logic is duplicated here.
//!
//! Exit-code contract:
//!   - 0 on success; the TS `output.log` message
//!     `✓ Added <n> dependencies successfully` is rendered on stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to add dependencies:`
//!     (parity with the TS chalk-red error path at
//!     `src/commands/add-dependencies.ts:117-122`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::add_dependencies;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js flag set at
/// `src/commands/add-dependencies.ts:86-95`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub blocks: Option<Vec<String>>,
    pub blocked_by: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub relates_to: Option<Vec<String>>,
}

/// Entry point invoked from `main.rs` for the `add-dependencies` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args into the canonical JSON shape consumed by
    // fspec_core::commands::add_dependencies::run.
    let mut deps = serde_json::Map::new();
    if let Some(v) = args.blocks.as_ref() {
        deps.insert(
            "blocks".to_string(),
            Value::Array(v.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    if let Some(v) = args.blocked_by.as_ref() {
        deps.insert(
            "blockedBy".to_string(),
            Value::Array(v.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    if let Some(v) = args.depends_on.as_ref() {
        deps.insert(
            "dependsOn".to_string(),
            Value::Array(v.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    if let Some(v) = args.relates_to.as_ref() {
        deps.insert(
            "relatesTo".to_string(),
            Value::Array(v.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    let args_json = json!({
        "workUnitId": args.work_unit_id,
        "dependencies": Value::Object(deps),
    })
    .to_string();

    match add_dependencies::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The core returns a JSON `{success:true, added:N}` payload.
            // The TS CLI renders `✓ Added <n> dependencies successfully` —
            // we parse the JSON and synthesize the same line here so the
            // core stays output-agnostic.
            let added = serde_json::from_str::<Value>(&rendered)
                .ok()
                .and_then(|v| v["added"].as_u64())
                .unwrap_or(0);
            println!("✓ Added {added} dependencies successfully");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to add dependencies:', error.message)`.
            // Use `render_core_error` so the dispatcher-only
            // `"Invalid args for fspec command add-dependencies: "`
            // envelope is stripped before printing — shell stderr must
            // match `node dist/index.js` byte-for-byte.
            eprintln!("✗ Failed to add dependencies: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
