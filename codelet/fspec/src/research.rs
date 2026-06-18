//! `research` shell-facing CLI bridge (RPC-286, LIST-only scope).
//!
//! Feature: spec/features/research-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::Research` clap variant in [`crate::main`]) and delegates
//! to the single source-of-truth in
//! [`codelet_fspec_core::commands::research::run`] — the SAME function the
//! LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::research::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::research::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/research.ts:285`).
//!
//! ## Scope
//!
//! LIST-only (see core module docs): the EXECUTE path (tool selected via
//! `--tool` and actually run) is deferred. A KNOWN tool selected via `--tool`
//! surfaces a not-yet-ported reason; an UNKNOWN tool surfaces the
//! `Research tool not found: <name>` guard. Both arrive as
//! `FspecCoreError::InvalidArgs` and are rendered to stderr with exit 1.
//!
//! ## Output (mirrors registerResearchCommand listing — research.ts:288-301)
//!
//! `run` returns a JSON envelope `{ tools: [...], executed:false,
//! discoveryMethod:"registry" }`. This bridge prints the listing exactly as
//! the TS CLI path does:
//!   Available Research Tools:\n              ← header + blank line
//!     <indicator> <name>
//!       <description>
//!       Usage: fspec research --tool=<name> <args>
//!       Config: <first guidance line>         ← only when present
//!     <blank line>
//!
//! Exit-code contract: 0 on success; 1 on any [`FspecCoreError`] with the
//! bare message on stderr (parity with the TS `output.error(error.message)` +
//! `process.exit(1)` catch path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::research;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/research.ts:277-282` (`--tool <name>`, `--work-unit <id>`,
/// plus forwarded positional/unknown args). In the LIST-only port only
/// `--tool` affects behaviour; the rest are accepted for surface parity and
/// forwarded into the JSON args object so future EXECUTE work threads through
/// without an API break.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Research tool to execute. `None` ⇒ list available tools.
    pub tool: Option<String>,
    /// Work unit context (forwarded to the tool when EXECUTE lands).
    pub work_unit: Option<String>,
}

/// Entry point invoked from `main.rs` for the `research` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by
    // fspec_core::commands::research::run.
    let mut obj = Map::new();
    if let Some(t) = &args.tool {
        obj.insert("tool".to_string(), json!(t));
    }
    if let Some(w) = &args.work_unit {
        obj.insert("workUnit".to_string(), json!(w));
    }
    let args_json = Value::Object(obj).to_string();

    match research::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let envelope: Value =
                serde_json::from_str(&rendered).context("decode research envelope")?;
            print_listing(&envelope);
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error(error.message)` path: bare message
            // on stderr (the dispatcher envelope is stripped), exit 1.
            eprintln!("{}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Render the LIST-mode envelope to stdout in the TS CLI listing format.
fn print_listing(envelope: &Value) {
    // Header + blank line (TS `output.log('Available Research Tools:\n')`).
    println!("Available Research Tools:\n");

    let tools = envelope
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for tool in &tools {
        let indicator = tool
            .get("statusIndicator")
            .and_then(Value::as_str)
            .unwrap_or("✗");
        let name = tool.get("name").and_then(Value::as_str).unwrap_or("");
        let description = tool.get("description").and_then(Value::as_str).unwrap_or("");

        println!("  {indicator} {name}");
        println!("    {description}");
        println!("    Usage: fspec research --tool={name} <args>");
        if let Some(guidance) = tool.get("configGuidance").and_then(Value::as_str) {
            // TS prints only the first line: `tool.configGuidance.split('\n')[0]`.
            let first = guidance.lines().next().unwrap_or("");
            println!("    Config: {first}");
        }
        // Trailing blank line per tool (TS `output.log()`).
        println!();
    }
}
