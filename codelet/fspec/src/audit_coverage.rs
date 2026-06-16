//! `audit-coverage` shell-facing CLI bridge (RPC-197).
//!
//! Feature: spec/features/audit-coverage-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This thin façade resolves the project root from
//! CWD (parity with the TS `process.cwd()` default at
//! `src/commands/audit-coverage.ts:29`), marshals the positional
//! `<feature-name>` into JSON, and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::audit_coverage::run`] — the SAME function
//! the LLM-facing dispatcher invokes.
//!
//! This bridge contains NO file-existence checking or report rendering — it
//! only marshals args and decodes the `{output, exitCode}` envelope.
//!
//! Exit-code contract (parity with `auditCoverageCommand` at
//! `src/commands/audit-coverage.ts:120-124`):
//!   - 0 → all referenced files found; report printed to stdout.
//!   - 1 → coverage file missing OR one or more referenced files missing;
//!     report printed to stdout.
//!   - 1 (with `Error:` on stderr) → any [`codelet_fspec_core::FspecCoreError`]
//!     (e.g. malformed coverage JSON).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::audit_coverage;
use serde::Deserialize;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/audit-coverage.ts:126-136`.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Positional `<feature-name>` from clap.
    pub feature_name: String,
}

/// The `{output, exitCode}` envelope returned by the core command.
#[derive(Debug, Deserialize)]
struct Outcome {
    #[serde(default)]
    output: String,
    #[serde(default, rename = "exitCode")]
    exit_code: u8,
}

/// Entry point invoked from `main.rs` for the `audit-coverage` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    let payload = json!({ "featureName": args.feature_name });
    let args_json = payload.to_string();

    match audit_coverage::run(&args_json, &project_root).await {
        Ok(envelope) => {
            let outcome: Outcome =
                serde_json::from_str(&envelope).context("parse audit-coverage JSON payload")?;
            // TS parity: `output.log(result.output)` always appends a trailing
            // newline regardless of the string's own terminator.
            println!("{}", outcome.output);
            Ok(outcome.exit_code)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
