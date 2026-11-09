//! `list-work-units` shell-facing CLI bridge (RPC-253, CLI-017).
//!
//! Feature: spec/features/list-work-units-cli-subcommand.feature
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! Thin façade: clap parses the board-query flags into [`Flags`], which
//! serialises to the JSON args shape that
//! [`codelet_fspec_core::commands::list_work_units::run`] validates — the
//! same function the LLM-facing dispatcher invokes. No filter or rendering
//! logic lives here. The core reads the system clock and the local zone
//! (`TZ` is honoured).
//!
//! Exit-code contract (rules [13] / [14] on RPC-253): 0 on success with the
//! rendered output on stdout; 1 on any error, written to stderr prefixed
//! with `Error:`.

use crate::common::spec_project_root;
use std::path::PathBuf;

use anyhow::Result;
use codelet_fspec_core::commands::list_work_units;
use serde::Serialize;

fn is_false(b: &bool) -> bool {
    !*b
}

/// The board-query flags. List flags take a comma list or repeat.
#[derive(Debug, Default, clap::Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flags {
    /// Statuses (comma list).
    #[arg(short, long, value_name = "STATUS", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<String>,
    /// Types (comma list): story, task, bug.
    #[arg(short = 't', long = "type", value_name = "TYPE", value_delimiter = ',')]
    #[serde(rename = "type", skip_serializing_if = "Vec::is_empty")]
    pub r#type: Vec<String>,
    /// Registered prefixes (comma list). No short flag: -p is --parent.
    #[arg(long, value_name = "PREFIX", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prefix: Vec<String>,
    /// Registered epics (comma list).
    #[arg(short, long, value_name = "EPIC", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub epic: Vec<String>,
    /// Cards or ranges such as BUG-600..BUG-620 (comma list).
    #[arg(long = "work-unit", value_name = "workUnitId", value_delimiter = ',')]
    #[serde(rename = "workUnitId", skip_serializing_if = "Vec::is_empty")]
    pub work_unit: Vec<String>,
    /// Cards whose parent is this card.
    #[arg(short, long, value_name = "workUnitId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Cards with child cards.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_children: bool,
    /// Estimate at least N.
    #[arg(long, value_name = "N")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate_min: Option<i64>,
    /// Estimate at most N.
    #[arg(long, value_name = "N")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate_max: Option<i64>,
    /// Cards with an estimate.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_estimate: bool,
    /// Cards without an estimate.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub no_estimate: bool,
    /// Created at or after TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_since: Option<String>,
    /// Created before TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_until: Option<String>,
    /// Updated at or after TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_since: Option<String>,
    /// Updated before TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_until: Option<String>,
    /// Done at or after TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_since: Option<String>,
    /// Done before TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_until: Option<String>,
    /// Cards with a stateHistory transition into STATUS.
    #[arg(long, value_name = "STATUS")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entered: Option<String>,
    /// With --entered: a transition at or after TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entered_since: Option<String>,
    /// With --entered: a transition before TIME.
    #[arg(long, value_name = "TIME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entered_until: Option<String>,
    /// Case-insensitive substring of the title or description.
    #[arg(long, value_name = "TEXT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Case-insensitive substring of the title.
    #[arg(long, value_name = "TEXT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Cards with an open question.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_open_questions: bool,
    /// Cards with assumptions.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_assumptions: bool,
    /// Cards with rules.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_rules: bool,
    /// Cards without rules.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub no_rules: bool,
    /// Cards with examples.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_examples: bool,
    /// Cards with attachments.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_attachments: bool,
    /// Cards with architecture notes.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_architecture_notes: bool,
    /// Feature-file tags (comma list), e.g. @cli.
    #[arg(long, value_name = "TAG", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<String>,
    /// dependsOn holds the id (any: not empty).
    #[arg(long, value_name = "workUnitId", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// blockedBy holds the id (any: not empty).
    #[arg(long, value_name = "workUnitId", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// blocks holds the id (any: not empty).
    #[arg(long, value_name = "workUnitId", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    /// relatesTo holds the id (any: not empty).
    #[arg(long, value_name = "workUnitId", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relates_to: Vec<String>,
    /// Status blocked, or blocked by a card that is not done.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub blocked: bool,
    /// Output fields (comma list).
    #[arg(long, value_name = "FIELDS", value_delimiter = ',')]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// table (default), ids, json, jsonl, count or csv.
    #[arg(long, value_name = "FORMAT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// status, type, prefix, epic, day or week.
    #[arg(long, value_name = "KEY")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    /// Sort by an output field.
    #[arg(long, value_name = "FIELD")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Reverse the --sort order.
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub desc: bool,
    /// Keep the first N cards after sorting.
    #[arg(long, value_name = "N")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Entry point invoked from `main.rs` for the `list-work-units` clap
/// subcommand. Returns the process exit code.
pub async fn run(flags: Flags) -> Result<u8> {
    // Resolve project root from CWD (parity with the TS `process.cwd()`).
    let project_root: PathBuf = spec_project_root()?;
    let args_json = serde_json::to_string(&flags)?;

    match list_work_units::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.is_empty() && !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}

// CLI-020: the guard that every card reference uses one name (needs the
// binary-private `Cli`, so it lives in src/).
#[cfg(test)]
mod work_unit_name_guard_tests;
