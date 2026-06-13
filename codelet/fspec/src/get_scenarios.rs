//! `get-scenarios` shell-facing CLI bridge (RPC-237).
//!
//! Feature: spec/features/get-scenarios-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::get_scenarios::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::get_scenarios::run
//!
//! This module performs NO domain computation (no file walking, no feature
//! parsing, no tag filtering): it only marshals the parsed clap arguments into
//! the JSON arg shape, calls the single fspec-core entry point, and renders the
//! returned envelope for the shell user. The dispatcher returns the FULL
//! envelope; for `--format json` the CLI prints ONLY the `scenarios` array.
//!
//! Exit-code contract:
//!   - 0 on success.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the bare reason is
//!     written to stderr prefixed with `Error: ` (parity with the TS
//!     `output.error('Error:', ...)` path).

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::get_scenarios;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration
/// (`src/commands/get-scenarios.ts:220-232`): repeatable `--tag` and a
/// `--format` string defaulting to `text`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub tags: Vec<String>,
    pub format: String,
}

/// Entry point invoked from `main.rs` for the `get-scenarios` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let mut obj = serde_json::Map::new();
    obj.insert("tags".to_string(), json!(args.tags));
    obj.insert("format".to_string(), json!(args.format));
    let args_json = Value::Object(obj).to_string();

    match get_scenarios::run(&args_json, &project_root).await {
        Ok(envelope_json) => {
            let envelope: Value = serde_json::from_str(&envelope_json).unwrap_or(Value::Null);

            // Surface any warnings on stderr (parity with TS `output.warn`).
            if let Some(Value::Array(items)) = envelope.get("warnings") {
                for w in items {
                    if let Some(s) = w.as_str() {
                        eprintln!("⚠ {s}");
                    }
                }
            }

            let scen = envelope
                .get("scenarios")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));

            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&scen).unwrap_or_default());
            } else {
                if let Some(msg) = envelope.get("message").and_then(Value::as_str) {
                    println!("{msg}");
                }
                let empty: Vec<Value> = Vec::new();
                let arr = scen.as_array().unwrap_or(&empty);
                if !arr.is_empty() {
                    println!();
                    render_grouped(arr);
                }
            }
            Ok(0)
        }
        Err(err) => {
            // Parity with the TS `output.error('Error:', result.error)` path:
            // the structured early-return surfaces the bare reason
            // `spec/features directory not found` (no I/O-envelope framing).
            // The missing-dir condition is modelled in core as an `Io` error
            // whose Display wraps the message in
            // `I/O error executing fspec command get-scenarios: <reason>`;
            // strip that framing so the shell user sees only `<reason>`.
            let rendered = render_core_error(&err);
            const IO_PREFIX: &str = "I/O error executing fspec command get-scenarios: ";
            let bare = rendered.strip_prefix(IO_PREFIX).unwrap_or(&rendered);
            eprintln!("Error: {bare}");
            Ok(1)
        }
    }
}

/// Render the default text view: group entries by their `feature` field in
/// first-seen order, printing the feature path then one `  <line>: <name>`
/// row per entry (with an optional ` [<tags>]` suffix). Parity with the TS
/// `getScenariosCommand` text branch.
fn render_grouped(scenarios: &[Value]) {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, Vec<&Value>> = HashMap::new();
    for s in scenarios {
        let feature = s
            .get("feature")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !grouped.contains_key(&feature) {
            order.push(feature.clone());
        }
        grouped.entry(feature).or_default().push(s);
    }

    for feature in &order {
        println!("{feature}");
        if let Some(list) = grouped.get(feature) {
            for s in list {
                let line = s.get("line").and_then(Value::as_u64).unwrap_or(0);
                let name = s.get("name").and_then(Value::as_str).unwrap_or("");
                let suffix = match s.get("tags").and_then(Value::as_array) {
                    Some(items) => {
                        let joined = items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ");
                        format!(" [{joined}]")
                    }
                    None => String::new(),
                };
                println!("  {line}: {name}{suffix}");
            }
        }
        println!();
    }
}
