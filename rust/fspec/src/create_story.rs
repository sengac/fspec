//! `create-story` shell-facing CLI bridge (RPC-214).
//!
//! Feature: spec/features/create-story-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::CreateStory` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::create_story::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::create_story::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::create_story::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with TS
//! `process.cwd()` default at `src/commands/create-story.ts:35`).
//!
//! Exit-code contract:
//!   - 0 on success; the rendered success block (✓ Created story... + Title +
//!     optional Epic/Parent/Description + Example-Mapping <system-reminder>)
//!     is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is written
//!     to stderr prefixed with `Error:` (parity with the TS chalk-red error
//!     path at `src/commands/create-story.ts:268-272`).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::create_story;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/create-story.ts:277-286`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub prefix: String,
    pub title: String,
    pub description: Option<String>,
    pub epic: Option<String>,
    pub parent: Option<String>,
}

/// Entry point invoked from `main.rs` for the `create-story` clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by fspec_core::commands::
    // create_story::run. The dispatcher and CLI both feed the SAME serde
    // shape.
    let mut obj = serde_json::Map::new();
    obj.insert("prefix".to_string(), json!(args.prefix));
    obj.insert("title".to_string(), json!(args.title));
    if let Some(d) = args.description {
        obj.insert("description".to_string(), json!(d));
    }
    if let Some(e) = args.epic {
        obj.insert("epic".to_string(), json!(e));
    }
    if let Some(p) = args.parent {
        obj.insert("parent".to_string(), json!(p));
    }
    let args_json = Value::Object(obj).to_string();

    match create_story::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // The rendered block carries BOTH the ✓ success lines AND the
            // trailing <system-reminder>. The TS CLI splits these across
            // stdout (success via output.log) and stderr (reminder via
            // output.error); we mirror that split so the CLI surface matches
            // create-story.ts:238-253.
            match rendered.split_once("<system-reminder>") {
                Some((stdout_part, reminder)) => {
                    print!("{}", stdout_part.trim_end_matches('\n'));
                    println!();
                    eprint!("<system-reminder>{reminder}");
                    if !reminder.ends_with('\n') {
                        eprintln!();
                    }
                }
                None => {
                    print!("{rendered}");
                    if !rendered.ends_with('\n') {
                        println!();
                    }
                }
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('Error:', error.message)`.
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
