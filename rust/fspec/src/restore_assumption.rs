//! `restore-assumption` shell-facing CLI bridge (BUG-633).
//!
//! Feature: spec/features/remove-and-restore-assumptions.feature
//!
//! JSON marshalling only: delegates to
//! [`codelet_fspec_core::commands::restore_assumption::run`], the function the
//! LLM-facing dispatcher calls too. Exit 0 with the success line on stdout;
//! exit 1 with `✗ Failed to restore assumption: <reason>` on stderr.

use crate::common::spec_project_root;

use anyhow::Result;
use codelet_fspec_core::commands::restore_assumption;
use serde_json::json;

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    pub index: u64,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root = spec_project_root()?;
    let args_json = json!({
        "workUnitId": args.work_unit_id,
        "index": args.index,
    })
    .to_string();
    match restore_assumption::run(&args_json, &project_root).await {
        Ok(data) => {
            let parsed: serde_json::Value = serde_json::from_str(&data)?;
            let text = parsed
                .get("restoredAssumption")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("restore-assumption returned no restoredAssumption: {data}")
                })?;
            println!("✓ Restored assumption: \"{text}\"");
            if let Some(message) = parsed.get("message").and_then(serde_json::Value::as_str) {
                println!("  {message}");
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!(
                "✗ Failed to restore assumption: {}",
                render_core_error(&err)
            );
            Ok(1)
        }
    }
}
