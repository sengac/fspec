//! `check` shell-facing CLI bridge (RPC-201).
//!
//! Feature: spec/features/check-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root
//! from CWD (parity with the TS `process.cwd()` default at
//! `src/commands/check.ts:29`), marshals the `-v/--verbose` flag into JSON,
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::check::run`] — the SAME function the
//! LLM-facing dispatcher invokes.
//!
//! This bridge contains NO Gherkin parsing, tag-validation, or
//! check-aggregation logic — it only marshals args, decodes the `CheckResult`
//! envelope, and RENDERS the display block. Rendering is presentation only;
//! all pass/fail determination is computed by the core.
//!
//! Exit-code contract (parity with `checkCommand` at
//! `src/commands/check.ts:160-226`):
//!   - 0 → `success == true` (all contributing checks pass).
//!   - 1 → `success == false`, OR any [`codelet_fspec_core::FspecCoreError`]
//!     (rendered as `Error: <msg>` on stderr).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::check;
use serde_json::{json, Value};

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/check.ts:229-234`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// `-v/--verbose`: show detailed validation output.
    pub verbose: bool,
}

/// Entry point invoked from `main.rs` for the `check` clap subcommand.
/// Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let payload = json!({ "verbose": args.verbose });
    let args_json = payload.to_string();

    match check::run(&args_json, &project_root).await {
        Ok(envelope) => {
            let v: Value = serde_json::from_str(&envelope).context("parse check JSON payload")?;
            let success = v.get("success").and_then(Value::as_bool).unwrap_or(false);
            print!("{}", render(&v));
            Ok(if success { 0 } else { 1 })
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}

/// Render the `CheckResult` envelope to the display block. Presentation only —
/// mirrors `src/commands/check.ts:168-220` line ordering (the chalk colour
/// wrapping is dropped; the status WORDS PASS/FAIL/SKIP are preserved).
fn render(v: &Value) -> String {
    let mut out = String::new();
    out.push_str("\nRunning validation checks...\n\n");

    let file_count = v.get("fileCount").and_then(Value::as_u64);
    if let Some(n) = file_count {
        if n > 0 {
            out.push_str(&format!("Checked {n} feature file(s)\n\n"));
        }
    }

    if let Some(s) = v.get("gherkinStatus").and_then(Value::as_str) {
        out.push_str(&format!("Gherkin syntax: {s}\n"));
    }
    if let Some(s) = v.get("tagStatus").and_then(Value::as_str) {
        out.push_str(&format!("Tag validation: {s}\n"));
    }
    if let Some(s) = v.get("formatStatus").and_then(Value::as_str) {
        out.push_str(&format!("Formatting: {s}\n"));
    }

    if let Some(errors) = v.get("errors").and_then(Value::as_array) {
        if !errors.is_empty() {
            out.push_str("\nErrors:\n");
            for err in errors {
                if let Some(msg) = err.as_str() {
                    out.push_str(&format!("  - {msg}\n"));
                }
            }
        }
    }

    out.push('\n');
    let success = v.get("success").and_then(Value::as_bool).unwrap_or(false);
    if success {
        let message = v
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("All checks passed");
        out.push_str(&format!("\u{2713} {message}\n"));
    } else {
        out.push_str("\u{2717} Some checks failed\n");
    }

    out
}
