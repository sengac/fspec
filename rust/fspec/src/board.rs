//! `board` shell-facing CLI bridge (RPC-199).
//!
//! Feature: spec/features/board-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default at
//! `src/commands/display-board.ts:98`), marshals the `--format`/`--limit`
//! options into JSON, and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::board::run`] — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! This bridge contains NO column-building, point-summing, or summary-string
//! logic — it only marshals args and renders the `{columns, board, summary}`
//! envelope.
//!
//! ## Framing-A: headless rendering (RPC-199, APPROVED)
//!
//! The TS CLI's default (`--format=text`) renders an interactive Ink TUI. The
//! Rust standalone binary is headless, so the default text path renders a
//! plain-text board (column headers + work-unit lines + the summary line)
//! instead of a TUI. `--format=json` emits the canonical 2-space-indented
//! `{columns, board, summary}` payload verbatim from the core.
//!
//! Exit-code contract:
//!   - 0 → board computed; rendering printed to stdout.
//!   - 1 → any [`codelet_fspec_core::FspecCoreError`] (e.g. missing
//!     foundation.json); message printed to stderr prefixed with
//!     `✗ Failed to display board:` (parity with the TS catch at
//!     `src/commands/display-board.ts:123-126`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::board;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/display-board.ts:90-96`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `--format <format>`: `"text"` (default) or `"json"`.
    pub format: Option<String>,
    /// `--limit <limit>`: max items per column in text mode. Default 25.
    pub limit: Option<usize>,
}

/// Entry point invoked from `main.rs` for the `board` clap subcommand.
/// Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let format = args.format.as_deref().unwrap_or("text").to_string();
    let limit = args.limit.unwrap_or(25);

    let payload = json!({ "format": format });
    let args_json = payload.to_string();

    match board::run(&args_json, &project_root).await {
        Ok(rendered) => {
            if format == "json" {
                // TS parity: `console.log(JSON.stringify(result, null, 2))`.
                println!("{rendered}");
            } else {
                // Headless plain-text rendering of the structured envelope.
                let value: Value =
                    serde_json::from_str(&rendered).context("parse board JSON payload")?;
                print!("{}", render_text(&value, limit));
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to display board: {err}");
            Ok(1)
        }
    }
}

/// Render the structured `{columns, board, summary}` envelope as a plain-text
/// board for the headless binary. This is presentation only — it derives no
/// new domain data, merely formatting the columns/summary computed by the
/// core. `limit` caps the number of work units shown per column.
fn render_text(value: &Value, limit: usize) -> String {
    let mut out = String::new();
    out.push('\n');

    if let Some(columns) = value.get("columns").and_then(Value::as_object) {
        for (status, entries) in columns {
            let arr = entries.as_array().map(Vec::as_slice).unwrap_or(&[]);
            out.push_str(&format!("{} ({})\n", status, arr.len()));
            for entry in arr.iter().take(limit) {
                let id = entry.get("id").and_then(Value::as_str).unwrap_or("");
                let title = entry.get("title").and_then(Value::as_str).unwrap_or("");
                match entry.get("estimate").and_then(Value::as_u64) {
                    Some(pts) => out.push_str(&format!("  {id}  {title}  [{pts}]\n")),
                    None => out.push_str(&format!("  {id}  {title}\n")),
                }
            }
            if arr.len() > limit {
                out.push_str(&format!("  … {} more\n", arr.len() - limit));
            }
            out.push('\n');
        }
    }

    if let Some(summary) = value.get("summary").and_then(Value::as_str) {
        out.push_str(summary);
        out.push('\n');
    }

    out
}
