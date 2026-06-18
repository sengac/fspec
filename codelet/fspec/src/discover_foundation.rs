//! `discover-foundation` shell-facing CLI bridge (RPC-226).
//!
//! Feature: spec/features/discover-foundation-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive as
//! the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::DiscoverFoundation` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::discover_foundation::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::discover_foundation::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::discover_foundation::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/discover-foundation.ts:306`).
//!
//! ## Output split (mirrors discover-foundation.ts:804-857)
//!
//! `run` returns a JSON envelope `{ valid, systemReminder?, validationErrors?,
//! completionMessage?, draftPath?, finalPath?, mdGenerated?, workUnitCreated?,
//! workUnitId?, forceOverwriteWarning? }`. This bridge:
//!   0. Emits a STDERR `--force` overwrite warning when
//!      `forceOverwriteWarning` is true (TS `output.warn`).
//!   1. Prints `systemReminder` to STDOUT first when present (TS `output.log`).
//!   2. Finalize mode:
//!      * invalid → STDERR `✗ Foundation validation failed` + the
//!        `validationErrors` block; exit 1.
//!      * valid → STDOUT `✓ Generated <finalPath>`, optional
//!        `✓ Generated spec/FOUNDATION.md`, `✓ Foundation discovered and
//!        validated successfully`, then (when a FOUND task was created) the
//!        `✓ Created work unit <id>: Foundation Event Storm` + `  Run: fspec
//!        show-work-unit <id>` lines; exit 0.
//!   3. Draft mode:
//!      * invalid → STDERR `✗ Failed to create draft`; exit 1.
//!      * valid → STDOUT `✓ Generated <draftPath>`, `Next steps:` and the two
//!        guidance lines; exit 0.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::discover_foundation;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/discover-foundation.ts:763-787`.
#[derive(Debug)]
pub struct CliArgs {
    pub finalize: bool,
    pub output: Option<String>,
    pub draft_path: Option<String>,
    /// Defaults to TRUE (parity with the TS `--auto-generate-md` default).
    pub auto_generate_md: bool,
    pub force: bool,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            finalize: false,
            output: None,
            draft_path: None,
            auto_generate_md: true,
            force: false,
        }
    }
}

/// Entry point invoked from `main.rs` for the `discover-foundation` clap
/// subcommand. Returns the process exit code so `main` can propagate it.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by
    // fspec_core::commands::discover_foundation::run.
    let mut obj = Map::new();
    obj.insert("finalize".to_string(), json!(args.finalize));
    if let Some(o) = &args.output {
        obj.insert("output".to_string(), json!(o));
    }
    if let Some(d) = &args.draft_path {
        obj.insert("draftPath".to_string(), json!(d));
    }
    obj.insert("autoGenerateMd".to_string(), json!(args.auto_generate_md));
    obj.insert("force".to_string(), json!(args.force));
    let args_json = Value::Object(obj).to_string();

    match discover_foundation::run(&args_json, &project_root).await {
        Ok(rendered) => {
            let envelope: Value = serde_json::from_str(&rendered)
                .context("decode discover-foundation envelope")?;
            Ok(render(&envelope, args.finalize))
        }
        Err(err) => {
            // True I/O / parse failures (e.g. finalize on a missing draft).
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Render the envelope to stdout/stderr and compute the exit code.
fn render(envelope: &Value, finalize: bool) -> u8 {
    let valid = envelope.get("valid").and_then(Value::as_bool).unwrap_or(false);

    // 0. `--force` over an existing draft → stderr warning (parity with the TS
    //    `output.warn` at discover-foundation.ts:669-679, emitted before the
    //    STDOUT system-reminder banner).
    if envelope
        .get("forceOverwriteWarning")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        eprintln!("⚠️  Warning: Overwriting existing foundation.json.draft with --force flag");
    }

    // 1. systemReminder → STDOUT first (TS `output.log`).
    if let Some(reminder) = envelope.get("systemReminder").and_then(Value::as_str) {
        if !reminder.is_empty() {
            println!("{reminder}");
        }
    }

    if finalize {
        if !valid {
            eprintln!("✗ Foundation validation failed");
            if let Some(errors) = envelope.get("validationErrors").and_then(Value::as_str) {
                eprintln!("\n{errors}");
            }
            return 1;
        }
        let final_path = envelope
            .get("finalPath")
            .and_then(Value::as_str)
            .unwrap_or("spec/foundation.json");
        println!("✓ Generated {final_path}");
        if envelope.get("mdGenerated").and_then(Value::as_bool).unwrap_or(false) {
            println!("✓ Generated spec/FOUNDATION.md");
        }
        println!("✓ Foundation discovered and validated successfully");
        // Foundation Event Storm work unit (parity with the TS CLI action,
        // discover-foundation.ts:826-835). Only printed when a NEW FOUND task
        // was created AND an id is present.
        let created = envelope
            .get("workUnitCreated")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if created {
            if let Some(id) = envelope.get("workUnitId").and_then(Value::as_str) {
                println!("✓ Created work unit {id}: Foundation Event Storm");
                println!("  Run: fspec show-work-unit {id}");
            }
        }
        0
    } else {
        if !valid {
            eprintln!("✗ Failed to create draft");
            return 1;
        }
        let draft_path = envelope
            .get("draftPath")
            .and_then(Value::as_str)
            .unwrap_or("spec/foundation.json.draft");
        println!("✓ Generated {draft_path}");
        println!("\nNext steps:");
        println!("1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders");
        println!("2. When complete, run: fspec discover-foundation --finalize");
        0
    }
}
