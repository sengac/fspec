//! `update-foundation` shell-facing CLI bridge (RPC-312).
//!
//! Feature: spec/features/update-foundation-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::UpdateFoundation` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::update_foundation::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::update_foundation::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::update_foundation::run
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! ALL draft probing, field mapping, validation, and disk I/O live in the
//! core. The bridge MUST NOT duplicate any of that logic. The draft-vs-final
//! follow-on lines mirror the TS `updateFoundationCommand` branch, keyed off
//! whether the returned message refers to the draft (`src/commands/update-foundation.ts:296-314`).
//! On the draft path the core also returns a `systemReminder` (the next
//! field-by-field guidance from `discoverFoundation({scanOnly})`), which this
//! bridge prints verbatim after the "Updated:" line — D1 parity.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::update_foundation;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub section: String,
    pub content: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let args_json = json!({
        "section": args.section,
        "content": args.content,
    })
    .to_string();

    match update_foundation::run(&args_json, &project_root).await {
        Ok(data_json) => {
            let parsed: Value = serde_json::from_str(&data_json).unwrap_or(Value::Null);
            let message = parsed
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();

            println!("✓ {message}");

            // The TS command shows different follow-on lines depending on
            // whether the draft or the final foundation was updated. The
            // core message is the discriminator (it names the draft file when
            // a draft was the write target).
            if message.contains("draft") {
                println!("  Updated: spec/foundation.json.draft");
                // D1 parity: the TS `updateFoundationCommand` chains to
                // `discoverFoundation({scanOnly})` and prints the resulting
                // field-by-field `<system-reminder>` after the draft line
                // (src/commands/update-foundation.ts:300-310). The core now
                // returns that reminder in `systemReminder`; print it verbatim.
                if let Some(reminder) = parsed.get("systemReminder").and_then(Value::as_str) {
                    if !reminder.is_empty() {
                        println!("{reminder}");
                    }
                }
                // DISC-003 rule 4/14: print the progress trailer (draft path).
                crate::common::print_next_steps(&parsed);
            } else {
                println!("  Updated: spec/foundation.json");
                println!("  Regenerated: spec/FOUNDATION.md");
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', result.error)` path:
            // stderr, prefixed, unwrapped reason (no dispatcher envelope).
            let reason = render_core_error(&err);
            eprintln!("Error: {reason}");
            Ok(1)
        }
    }
}
