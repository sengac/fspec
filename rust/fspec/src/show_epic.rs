//! `show-epic` shell-facing CLI bridge (RPC-302).
//!
//! Feature: spec/features/show-epic-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::ShowEpic` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::show_epic::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused for
//! RPC-302):
//!   - Shell argv         → clap → this module → fspec_core::commands::show_epic::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::show_epic::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/show-epic.ts:41`). The clap
//! subcommand exposes one REQUIRED positional `<EPIC_ID>` and one
//! `--format / -f` flag — matching the TS Commander.js registration at
//! `src/commands/show-epic.ts:136-142`.
//!
//! No epic-lookup, aggregation, percentage-rounding, or rendering logic is
//! duplicated here — the bridge's only computation is JSON arg marshalling.
//! The CLI-delegation test
//! `scenario_cli_delegates_to_same_fspec_core_function` scans this file
//! for forbidden substrings that would indicate inlined business logic.
//!
//! Exit-code contract:
//!   - 0 on success — including the work-units-missing case where the
//!     zero-percent progress line is printed.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `Error:` (parity with the TS chalk-red error
//!     path).
//!   - 2 (clap's own usage error) when the required positional is omitted —
//!     clap validates before this module is reached, so we never see that
//!     case here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::show_epic;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js surface for
/// `show-epic` (`src/commands/show-epic.ts:136-142`). The TS registration
/// declares one required positional `<epicId>` and a single
/// `-f, --format <format>` flag with default `'text'`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required positional — the epic slug to display.
    pub epic_id: String,
    /// Optional rendering mode. `None` → use fspec_core's text default;
    /// `Some("json")` → emit pretty-printed JSON.
    pub format: Option<String>,
}

/// Entry point invoked from `main.rs` for the `show-epic` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim via
/// `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The dispatcher arg keys are `epicId` and `format` (camelCase) —
    // verified by reading `ShowEpicArgs` in
    // `rust/fspec-core/src/commands/show_epic.rs` which uses
    // `#[serde(rename_all = "camelCase")]`. Only thread `format` through
    // when the CLI flag was supplied so fspec_core's default-arm
    // (`format: None` → text) drives unflagged invocations.
    let mut obj = serde_json::Map::new();
    obj.insert("epicId".into(), json!(args.epic_id));
    if let Some(fmt) = args.format.as_deref() {
        obj.insert("format".into(), json!(fmt));
    }
    let args_json = serde_json::Value::Object(obj).to_string();

    match show_epic::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Text format embeds its own leading + trailing newline
            // structure (see `render_text` in fspec-core); print as-is.
            // The JSON format from `to_string_pretty` does not end with
            // a newline, so we append one for shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗', error.message)` path: stderr,
            // prefixed with `Error:`, no ANSI required for parity with the
            // cross-port error contract.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
