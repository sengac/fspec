//! `astgrep` shell-facing CLI bridge (CLI-015).
//!
//! Feature: spec/features/fspec-astgrep-cli-subcommand.feature
//!
//! The `fspec astgrep` subcommand gives one-shot CLI users the same AST code
//! search the native agent gets from the AstGrep tool. Two front doors, one
//! source of truth: this bridge is JSON marshalling + stdout/stderr rendering
//! only — the search itself delegates to
//! [`codelet_tools::AstGrepTool::execute`] (the rig tool's internal
//! implementation) constructed with a nil session id, so no worktree
//! isolation applies and paths pass through relative to CWD.
//!
//! Exit-code contract:
//! - 0: matches printed to stdout in `file:line:column:text` format
//!   (including the `No matches found` success path).
//! - 1: tool error (invalid pattern, unsupported language, ...) printed to
//!   stderr as `Error: <message>`.
//!
//! `main` runs under `#[tokio::main]`, so the async `execute` call genuinely
//! awaits here — no new runtime is ever constructed.
//!
//! The bridge calls `AstGrepTool::execute` (the rig tool's internal
//! implementation, made `pub` for CLI-015) with a JSON `Value` rather than
//! the rig `Tool::call` entry point, because `codelet-fspec` has no
//! dependency on `rig-core`; `execute` performs the identical search and
//! keeps the CLI and the harness tool on one source of truth.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_tools::AstGrepTool;
use serde_json::json;
use uuid::Uuid;
/// Strongly-typed args for the `fspec astgrep` subcommand.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// AST pattern to match (required).
    pub pattern: Option<String>,
    /// Programming language (required; maps to AstGrep's `language` param).
    pub lang: Option<String>,
    /// File or directory to search (optional, defaults to current dir).
    pub path: Option<String>,
}

/// Entry point invoked from `main.rs` for the `astgrep` clap subcommand.
/// Returns the process exit code so `main` can propagate it verbatim.
pub async fn run(args: CliArgs) -> Result<u8> {
    let _project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Nil session id ⇒ no isolation context registered ⇒ paths pass through.
    let tool = AstGrepTool::new(Uuid::nil());

    // Missing required args (pattern/lang) are caught by clap up front
    // (`required = true`), so both are `Some` here.
    let pattern = args.pattern.expect("clap enforces required --pattern");
    let language = args.lang.expect("clap enforces required --lang");

    let mut value_map = serde_json::Map::new();
    value_map.insert("pattern".to_string(), json!(pattern));
    value_map.insert("language".to_string(), json!(language));
    if let Some(path) = args.path {
        value_map.insert("path".to_string(), json!(path));
    }

    let out = tool
        .execute(serde_json::Value::Object(value_map))
        .await;

    match out {
        Ok(content) if !content.is_error => {
            println!("{}", content.content);
            Ok(0)
        }
        _ => {
            // Tool error (execute returns Err on unexpected failure) or the
            // `ToolOutput::error(...)` variant (invalid pattern, unsupported
            // language, ...) — both render as `Error: <message>` on stderr.
            let msg = match out {
                Ok(c) => c.content,
                Err(e) => e.to_string(),
            };
            // The tool's error output already carries an `Error: ` prefix;
            // avoid double-prefixing it.
            if msg.starts_with("Error: ") {
                eprintln!("{msg}");
            } else {
                eprintln!("Error: {msg}");
            }
            Ok(1)
        }
    }
}
