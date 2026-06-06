//! `list-scenario-tags` shell-facing CLI bridge (RPC-249).
//!
//! Feature: spec/features/list-scenario-tags-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::ListScenarioTags` clap variant
//! in [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_scenario_tags::run`] — the
//! SAME function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::list_scenario_tags::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_scenario_tags::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/list-scenario-tags.ts:27`). The clap subcommand
//! exposes two required positionals plus a boolean flag.
//!
//! No Gherkin parse / tag accumulation / category lookup logic is
//! duplicated here — the bridge's only computation is JSON arg
//! marshalling, exit-code mapping, and delegation to
//! `fspec_core::commands::list_scenario_tags::render_text_from_json`
//! for the canonical TS-parity text rendering.
//!
//! ## Exit-code contract (TS-parity)
//!
//! fspec_core returns the JSON-format inner payload {success, tags,
//! message?, error?, categorizedTags?}. The CLI bridge requests the
//! `format=json` rendering, parses the result, and:
//!
//!   - inner success=true → delegate text rendering to fspec_core,
//!     write to stdout, exit 0
//!   - inner success=false → write canonical 'Error: <inner.error>' to
//!     stderr, exit 1 (parity with TS `output.error('Error:',
//!     result.error); process.exit(1)`)
//!
//! Only an outer `FspecCoreError` (arg-parse failure) results in
//! `eprintln!('Error: <wrapped>')` + exit 1; recoverable errors NEVER
//! take the FspecCoreError path.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_scenario_tags;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// positional + flag surface for `list-scenario-tags`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Feature file path RELATIVE to the resolved project root.
    pub file: String,
    /// Scenario name to match (exact, case-sensitive).
    pub scenario: String,
    /// When true, request the categorised-tag rendering.
    pub show_categories: bool,
}

/// Entry point invoked from `main.rs` for the `list-scenario-tags`
/// clap subcommand. Returns the process exit code so `main` can
/// propagate it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "file": args.file,
        "scenario": args.scenario,
        "showCategories": args.show_categories,
        "format": "json",
    })
    .to_string();

    let json_text = match list_scenario_tags::run(&args_json, &project_root).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(1);
        }
    };

    let value: Value = serde_json::from_str(&json_text)
        .context("fspec_core returned non-JSON despite format=json")?;

    let success = value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !success {
        let err = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        eprintln!("Error: {err}");
        return Ok(1);
    }

    // Happy path — delegate rendering to fspec_core so no presentation
    // logic is duplicated in this bridge.
    let rendered = list_scenario_tags::render_text_from_json(&args.scenario, &value);
    print!("{rendered}");
    if !rendered.ends_with('\n') {
        println!();
    }
    Ok(0)
}
