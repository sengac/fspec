//! `set-user-story` — Rust port of `src/commands/set-user-story.ts` (RPC-298).
//!
//! Captures the user story for a work unit (role, action, benefit) during
//! Example Mapping. The work unit must exist; the TypeScript implementation
//! does **not** gate on status, so the Rust port does not either. Existing
//! `userStory` records are overwritten in place.
//!
//! Reuses shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json` (parity with TS `ensureWorkUnitsFile`).
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write at
//!   the end (the TS implementation uses `fileManager.transaction`).
//! * [`crate::io::time::iso8601_now`] — millisecond-precision ISO-8601
//!   timestamps (parity with TS `new Date().toISOString()`).
//!
//! ## On-disk shape
//!
//! Per the TS `UserStory` interface (`src/types/index.ts`), each user story
//! is the literal three-field object in declared order `{role, action,
//! benefit}`:
//!
//! ```json
//! {
//!   "role": "developer",
//!   "action": "validate feature files",
//!   "benefit": "catch bugs"
//! }
//! ```
//!
//! The `userStory` key lives on the work unit's `extra` map (round-tripped
//! via `#[serde(flatten)]` on [`crate::types::work_unit::WorkUnit`]). We use
//! `serde_json::Map` (insertion-order-preserving in this workspace — see
//! `codelet/Cargo.toml`) to honour the canonical {role, action, benefit}
//! field order on disk.
//!
//! ## DispatchResult.data
//!
//! Mirrors the four-line success block emitted by the TS CLI wrapper at
//! `src/commands/set-user-story.ts:54-57`:
//!
//! ```text
//! ✓ User story set for <work-unit-id>
//!   As a <role>
//!   I want to <action>
//!   So that <benefit>
//! ```
//!
//! The CLI bridge prints this verbatim to stdout; the LLM-facing
//! dispatcher returns it in `DispatchResult.data`.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge
//! at `codelet/fspec/src/set_user_story.rs` is JSON marshalling only —
//! no domain logic.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `set-user-story`. Mirrors the TS
/// `SetUserStoryOptions` interface at `src/commands/set-user-story.ts:9-14`
/// plus the positional `<work-unit-id>` from the Commander.js registration
/// at `src/commands/set-user-story.ts:65-80`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetUserStoryArgs {
    work_unit_id: String,
    role: String,
    action: String,
    benefit: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: SetUserStoryArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "set-user-story",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run). On a brand-new workspace this writes
    // the canonical empty initial structure to disk before we return the
    // missing-source error below, matching TS `ensureWorkUnitsFile`.
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (mirrors src/commands/set-user-story.ts:25-27).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "set-user-story",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    let now = iso8601_now();

    // Build the UserStory with explicit field declaration order
    // (role, action, benefit) so on-disk JSON matches the TS
    // `{role, action, benefit}` object-literal insertion order at
    // `src/commands/set-user-story.ts:29-33`.
    let mut user_story = Map::new();
    user_story.insert("role".to_string(), Value::String(args.role.clone()));
    user_story.insert("action".to_string(), Value::String(args.action.clone()));
    user_story.insert("benefit".to_string(), Value::String(args.benefit.clone()));

    let wu = data
        .work_units
        .get_mut(&args.work_unit_id)
        .expect("work unit exists");

    // Overwrite any prior value verbatim (parity with TS direct assignment).
    wu.extra
        .insert("userStory".to_string(), Value::Object(user_story));

    // Bump updatedAt (mirrors `data.workUnits[workUnitId].updatedAt = new Date().toISOString();`).
    wu.updated_at = now.clone();

    // Bump meta.lastUpdated if present (mirrors L38-40).
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // Single atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Build the four-line success block emitted by the TS CLI wrapper at
    // `src/commands/set-user-story.ts:54-57`. Returned in
    // `DispatchResult.data` and printed verbatim by the CLI bridge.
    Ok(render_success(&args.work_unit_id, &args.role, &args.action, &args.benefit))
}

/// Render the canonical four-line success block.
fn render_success(work_unit_id: &str, role: &str, action: &str, benefit: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("✓ User story set for {work_unit_id}\n"));
    s.push_str(&format!("  As a {role}\n"));
    s.push_str(&format!("  I want to {action}\n"));
    s.push_str(&format!("  So that {benefit}\n"));
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: SetUserStoryArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","role":"dev","action":"ship","benefit":"win"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.role, "dev");
        assert_eq!(a.action, "ship");
        assert_eq!(a.benefit, "win");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<SetUserStoryArgs>(
            r#"{"role":"x","action":"y","benefit":"z"}"#,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }

    #[test]
    fn render_success_emits_four_canonical_lines() {
        let out = render_success("AUTH-001", "developer", "ship", "happiness");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "✓ User story set for AUTH-001");
        assert_eq!(lines[1], "  As a developer");
        assert_eq!(lines[2], "  I want to ship");
        assert_eq!(lines[3], "  So that happiness");
    }
}
