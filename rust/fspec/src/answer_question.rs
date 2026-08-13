//! `answer-question` shell-facing CLI bridge (RPC-196).
//!
//! Feature: spec/features/answer-question-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that
//! parses argv (the `Mode::AnswerQuestion` clap variant in [`crate::main`])
//! and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::answer_question::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Bridge scope (per Gherkin rule "bridge contains no domain logic"):
//!   - clap arg parsing (workUnitId, index, --answer, --add-to)
//!   - JSON arg marshalling
//!   - stdout/stderr rendering with TS-canonical prefixes
//!
//! The bridge MUST NOT embed RuleItem construction, status guards,
//! questions lookup, or disk I/O.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::answer_question;
use serde_json::{json, Map, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TS Commander.js registration at
/// `src/commands/answer-question.ts:127-138`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub index: i64,
    pub answer: Option<String>,
    pub add_to: Option<String>,
}

/// Entry point invoked from `main.rs` for the `answer-question` clap
/// subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal clap args → JSON object expected by the core.
    let mut body = Map::new();
    body.insert(
        "workUnitId".to_string(),
        Value::String(args.work_unit_id.clone()),
    );
    body.insert("index".to_string(), Value::from(args.index));
    if let Some(a) = args.answer.clone() {
        body.insert("answer".to_string(), Value::String(a));
    }
    if let Some(a) = args.add_to.clone() {
        body.insert("addTo".to_string(), Value::String(a));
    }
    let args_json = json!(body).to_string();

    match answer_question::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // Parse the result JSON to render the TS-canonical stdout
            // lines. The core returns {success, question, addedTo?,
            // addedContent?} — the CLI surface mirrors TS
            // `output.log` lines at src/commands/answer-question.ts:151-160.
            let v: Value = serde_json::from_str(&rendered).unwrap_or(Value::Null);
            let question_text = v["question"].as_str().unwrap_or("");
            println!("✓ Answered question: \"{question_text}\"");
            if let Some(a) = args.answer.as_deref() {
                println!("  Answer: \"{a}\"");
            }
            let added_to = v["addedTo"].as_str();
            let added_content = v["addedContent"].as_str();
            if let (Some(target), Some(content)) = (added_to, added_content) {
                println!("  Added to {target}: \"{content}\"");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to answer question: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
