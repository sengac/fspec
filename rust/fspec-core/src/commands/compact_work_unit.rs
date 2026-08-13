//! `compact-work-unit` — Rust port of `src/commands/compact-work-unit.ts` (RPC-206).
//!
//! Permanently removes soft-deleted Example-Mapping items (`deleted: true`)
//! from a work unit's `rules`, `examples`, `questions`, and
//! `architectureNotes` arrays, renumbers the survivors' `id` fields
//! sequentially from 0, and resets the matching `nextRuleId` /
//! `nextExampleId` / `nextQuestionId` / `nextNoteId` counters to the new
//! array lengths.
//!
//! Behaviour parity with `src/commands/compact-work-unit.ts`:
//! * **Existence check** — a missing work unit errors with
//!   `"Work unit '<id>' does not exist"` (`:70-72`).
//! * **Force gate** — when the unit is NOT in `done` status, `--force` is
//!   required; otherwise the command errors with
//!   `"Cannot compact work unit in '<status>' status. Use --force to confirm
//!   compaction during active development."` (`:78-86`).
//! * **Always-assign arrays** — like the TS `workUnit.rules =
//!   rulesResult.filtered` assignment, every one of the four arrays is set to
//!   its compacted form (an empty array when previously absent), and all four
//!   `nextId` counters are written (`:103-132`).
//! * **Timestamps** — `updatedAt` on the unit and `meta.lastUpdated` are
//!   bumped to the current ISO-8601 instant (`:134-140`).
//!
//! All Example-Mapping arrays and `nextId` counters live in the
//! [`WorkUnit`]'s `extra` map, so this command reads and mutates them through
//! `serde_json::Value`.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand route through this single `run`. The CLI bridge at
//! `rust/fspec/src/compact_work_unit.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `compact-work-unit`. Mirrors the TS
/// `CompactWorkUnitOptions` interface at `src/commands/compact-work-unit.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactWorkUnitArgs {
    work_unit_id: String,
    /// When true, allows compaction during a non-`done` status.
    #[serde(default)]
    force: Option<bool>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CompactWorkUnitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "compact-work-unit",
            reason: format!("failed to parse args: {e}"),
        })?;

    let id = args.work_unit_id;
    let force = args.force.unwrap_or(false);

    let mut data = ensure_work_units_file(project_root)?;

    // Existence check (mirrors src/commands/compact-work-unit.ts:70-72).
    // Force gate for non-done status (mirrors :78-86).
    let status = match data.work_units.get(&id) {
        Some(w) => w.status.as_str(),
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "compact-work-unit",
                reason: format!("Work unit '{id}' does not exist"),
            });
        }
    };
    if status != "done" && !force {
        return Err(FspecCoreError::InvalidArgs {
            command: "compact-work-unit",
            reason: format!(
                "Cannot compact work unit in '{status}' status. Use --force to confirm compaction during active development."
            ),
        });
    }

    let now = iso8601_now();

    // Mutate the work unit inside a scoped borrow so `data.meta` can be
    // bumped afterwards without overlapping mutable borrows.
    let (rules_removed, examples_removed, questions_removed, notes_removed) = {
        let wu = data
            .work_units
            .get_mut(&id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "compact-work-unit",
                reason: format!("Work unit '{id}' does not exist"),
            })?;

        // Compact each array (mirrors :103-125). `compact_array_field` ensures
        // the key exists as an array (matching the unconditional TS
        // assignment), then drops `deleted: true` items and renumbers
        // survivors' ids from 0.
        let (rules_removed, _rules_remaining) = compact_array_field(&mut wu.extra, "rules");
        let (examples_removed, _examples_remaining) =
            compact_array_field(&mut wu.extra, "examples");
        let (questions_removed, _questions_remaining) =
            compact_array_field(&mut wu.extra, "questions");
        let (notes_removed, _notes_remaining) =
            compact_array_field(&mut wu.extra, "architectureNotes");

        // Reset the four nextId counters to the new array lengths (mirrors
        // :127-132). `array_len` re-reads the freshly-compacted arrays so the
        // counters never drift from the survivor count.
        let next_rule_id = array_len(&wu.extra, "rules");
        let next_example_id = array_len(&wu.extra, "examples");
        let next_question_id = array_len(&wu.extra, "questions");
        let next_note_id = array_len(&wu.extra, "architectureNotes");
        wu.extra
            .insert("nextRuleId".to_string(), Value::from(next_rule_id));
        wu.extra
            .insert("nextExampleId".to_string(), Value::from(next_example_id));
        wu.extra
            .insert("nextQuestionId".to_string(), Value::from(next_question_id));
        wu.extra
            .insert("nextNoteId".to_string(), Value::from(next_note_id));

        // Bump the work unit timestamp (mirrors :135).
        wu.updated_at = now.clone();

        (
            rules_removed,
            examples_removed,
            questions_removed,
            notes_removed,
        )
    };

    // Bump meta.lastUpdated when present (mirrors :138-140).
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // Single atomic write (TS uses `fileManager.transaction`).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Render the CLI-equivalent output. The dispatcher exposes this verbatim
    // as `DispatchResult.data`; the CLI bridge prints it to stdout.
    let total_removed = rules_removed + examples_removed + questions_removed + notes_removed;
    if total_removed == 0 {
        return Ok("No deleted items to remove\n".to_string());
    }

    let mut out = format!("✓ Compacted work unit {id}\n");
    out.push_str("  Removed items:\n");
    if rules_removed > 0 {
        out.push_str(&format!("    Rules: {rules_removed}\n"));
    }
    if examples_removed > 0 {
        out.push_str(&format!("    Examples: {examples_removed}\n"));
    }
    if questions_removed > 0 {
        out.push_str(&format!("    Questions: {questions_removed}\n"));
    }
    if notes_removed > 0 {
        out.push_str(&format!("    Architecture Notes: {notes_removed}\n"));
    }
    Ok(out)
}

/// Compact one Example-Mapping array on a work unit's `extra` map.
///
/// Ensures `key` exists as an array (matching the unconditional TS
/// `workUnit.<field> = result.filtered` assignment), drops every item whose
/// `deleted` field is `true`, renumbers the survivors' `id` fields
/// sequentially from 0, and returns `(removed, remaining)` counts.
fn compact_array_field(extra: &mut Map<String, Value>, key: &str) -> (usize, usize) {
    let entry = extra
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    let Some(arr) = entry.as_array_mut() else {
        // Unreachable: `entry` was just ensured to be an array above.
        return (0, 0);
    };

    let original = arr.len();
    arr.retain(|item| {
        !item
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    for (index, item) in arr.iter_mut().enumerate() {
        if let Some(obj) = item.as_object_mut() {
            obj.insert("id".to_string(), Value::from(index as u64));
        }
    }
    let remaining = arr.len();
    (original - remaining, remaining)
}

/// Length of the array stored at `key`, or 0 when absent / not an array.
fn array_len(extra: &Map<String, Value>, key: &str) -> u64 {
    extra
        .get(key)
        .and_then(Value::as_array)
        .map(|a| a.len() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: CompactWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","force":true}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.force, Some(true));
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<CompactWorkUnitArgs>("{}").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("workunitid"));
    }

    #[test]
    fn compact_array_field_drops_deleted_and_renumbers() {
        let mut m = Map::new();
        m.insert(
            "rules".into(),
            serde_json::json!([
                {"id": 0, "text": "a", "deleted": true},
                {"id": 1, "text": "b", "deleted": false},
                {"id": 2, "text": "c", "deleted": true},
                {"id": 3, "text": "d"}
            ]),
        );
        let (removed, remaining) = compact_array_field(&mut m, "rules");
        assert_eq!((removed, remaining), (2, 2));
        let arr = m["rules"].as_array().unwrap();
        assert_eq!(arr[0]["id"].as_u64(), Some(0));
        assert_eq!(arr[0]["text"].as_str(), Some("b"));
        assert_eq!(arr[1]["id"].as_u64(), Some(1));
        assert_eq!(arr[1]["text"].as_str(), Some("d"));
    }

    #[test]
    fn compact_array_field_creates_empty_array_when_absent() {
        let mut m = Map::new();
        let (removed, remaining) = compact_array_field(&mut m, "examples");
        assert_eq!((removed, remaining), (0, 0));
        assert!(m["examples"].as_array().unwrap().is_empty());
    }
}
