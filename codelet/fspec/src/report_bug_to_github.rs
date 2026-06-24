//! `report-bug-to-github` shell-facing CLI bridge (RPC-285,
//! DETERMINISTIC-CORE scope).
//!
//! Feature: spec/features/report-bug-to-github-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::ReportBugToGitHub` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::report_bug_to_github::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::report_bug_to_github::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::report_bug_to_github::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/report-bug-to-github.ts:266-267`).
//!
//! ## Scope
//!
//! DETERMINISTIC-CORE (see core module docs): the browser launch and the
//! interactive stdin prompts are DEFERRED. The CLI prints the gathering banner
//! and the constructed GitHub issue URL; it never launches a browser.
//!
//! ## Output (mirrors report-bug-to-github.ts:384-406)
//!
//! `run` returns a JSON envelope `{ title, markdown, url, browserOpened, ... }`.
//! This bridge:
//!   1. Prints `\nGathering system context...\n` (TS `output.log`).
//!   2. Prints the constructed GitHub issue URL so the user can open it
//!      manually (browser launch deferred).
//!
//! Exit-code contract: 0 on success; 1 on any [`FspecCoreError`] with the bare
//! message on stderr (parity with the TS `output.error('Error:', message)` +
//! `process.exit(1)` catch path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::report_bug_to_github;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/report-bug-to-github.ts:364-374`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub project_root: Option<String>,
    pub bug_description: Option<String>,
    pub expected_behavior: Option<String>,
    pub actual_behavior: Option<String>,
    pub interactive: bool,
}

/// Entry point invoked from `main.rs` for the `report-bug-to-github` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by
    // fspec_core::commands::report_bug_to_github::run.
    let mut obj = Map::new();
    if let Some(p) = &args.project_root {
        obj.insert("projectRoot".to_string(), json!(p));
    }
    if let Some(b) = &args.bug_description {
        obj.insert("bugDescription".to_string(), json!(b));
    }
    if let Some(e) = &args.expected_behavior {
        obj.insert("expectedBehavior".to_string(), json!(e));
    }
    if let Some(a) = &args.actual_behavior {
        obj.insert("actualBehavior".to_string(), json!(a));
    }
    obj.insert("interactive".to_string(), json!(args.interactive));
    let args_json = Value::Object(obj).to_string();

    // TS `output.log('\nGathering system context...\n')`.
    println!("\nGathering system context...\n");

    match report_bug_to_github::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let envelope: Value =
                serde_json::from_str(&rendered).context("decode report-bug-to-github envelope")?;
            print_result(&envelope);
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', message)` path.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Render the bug-report envelope to stdout. The browser launch is deferred,
/// so we surface the constructed GitHub issue URL for manual review.
fn print_result(envelope: &Value) {
    let url = envelope.get("url").and_then(Value::as_str).unwrap_or("");
    println!("Open the following pre-filled GitHub issue in your browser:\n");
    println!("{url}");
}
