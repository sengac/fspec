//! `delete-work-unit` — Rust port of `src/commands/delete-work-unit.ts` (RPC-223).
//!
//! Permanently removes a single work unit from `spec/work-units.json` and
//! tidies up every back-reference to it:
//!
//! * **Children guard** — a unit that still has `children` cannot be deleted
//!   (TS `src/commands/delete-work-unit.ts:40-44`).
//! * **Dependency guard** — a unit carrying any `blocks` / `blockedBy` /
//!   `dependsOn` / `relatesTo` reference is refused unless
//!   `--cascade-dependencies` is supplied (`:47-57`).
//! * **Blocks warning** — when the unit blocks others, a `⚠` warning line is
//!   emitted listing the blocked ids (`:60-64`).
//! * **Cascade cleanup** — with `cascadeDependencies` set, the inverse side
//!   of `blocks` (`blockedBy`), `blockedBy` (`blocks`) and `relatesTo`
//!   (`relatesTo`) references on the *target* units are stripped. `dependsOn`
//!   is intentionally NOT cascaded (`:66-109`).
//! * **Parent cleanup** — the unit id is filtered out of its parent's
//!   `children` array (`:111-117`).
//! * **State cleanup** — the id is removed from every Kanban state array
//!   (`:119-126`).
//!
//! The relationship fields (`children`, `blocks`, `blockedBy`, `dependsOn`,
//! `relatesTo`, `parent`) all live in the [`WorkUnit`]'s `extra` map, so this
//! command reads and mutates them through `serde_json::Value` rather than
//! typed accessors.
//!
//! Reuses shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load.
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand route through this single `run`. The CLI bridge at
//! `codelet/fspec/src/delete_work_unit.rs` is JSON marshalling only — no
//! domain logic.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `delete-work-unit`. Mirrors the TS
/// `DeleteWorkUnitOptions` interface at `src/commands/delete-work-unit.ts:9-15`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteWorkUnitArgs {
    work_unit_id: String,
    /// Accepted for Commander.js surface parity (`--force`) but never read
    /// by the TS implementation — declared-yet-unused.
    #[serde(default)]
    #[allow(dead_code)]
    force: Option<bool>,
    /// Accepted for parity (`--skip-confirmation`); the non-interactive
    /// Rust/dispatcher path never prompts, so it is a no-op here too.
    #[serde(default)]
    #[allow(dead_code)]
    skip_confirmation: Option<bool>,
    /// When true, strips inverse dependency references before deleting.
    #[serde(default)]
    cascade_dependencies: Option<bool>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteWorkUnitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-work-unit",
            reason: format!("failed to parse args: {e}"),
        })?;

    let id = args.work_unit_id;
    let cascade = args.cascade_dependencies.unwrap_or(false);

    let mut data = ensure_work_units_file(project_root)?;

    // Existence check (mirrors src/commands/delete-work-unit.ts:33-35).
    if !data.work_units.contains_key(&id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-work-unit",
            reason: format!("Work unit '{id}' does not exist"),
        });
    }

    // Snapshot the relationship arrays before mutating anything.
    let (children, blocks, blocked_by, depends_on, relates_to, parent) = {
        let wu = &data.work_units[&id];
        (
            str_array(&wu.extra, "children"),
            str_array(&wu.extra, "blocks"),
            str_array(&wu.extra, "blockedBy"),
            str_array(&wu.extra, "dependsOn"),
            str_array(&wu.extra, "relatesTo"),
            wu.extra
                .get("parent")
                .and_then(Value::as_str)
                .map(str::to_string),
        )
    };

    // Children guard (mirrors :40-44).
    if !children.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-work-unit",
            reason: format!(
                "Cannot delete work unit with children: {}. Delete children first or remove parent relationship.",
                children.join(", ")
            ),
        });
    }

    // Dependency guard (mirrors :47-57).
    let has_dependencies = !blocks.is_empty()
        || !blocked_by.is_empty()
        || !depends_on.is_empty()
        || !relates_to.is_empty();
    if has_dependencies && !cascade {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-work-unit",
            reason: format!(
                "Work unit '{id}' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete."
            ),
        });
    }

    // Blocks warning (mirrors :60-64).
    let mut warnings: Vec<String> = Vec::new();
    if !blocks.is_empty() {
        warnings.push(format!(
            "This work unit blocks {} work unit(s): {}",
            blocks.len(),
            blocks.join(", ")
        ));
    }

    // Cascade cleanup of inverse references (mirrors :66-109). `dependsOn`
    // is deliberately NOT cascaded — TS only walks blocks/blockedBy/relatesTo.
    if cascade {
        for target_id in &blocks {
            remove_reference(&mut data, target_id, "blockedBy", &id);
        }
        for target_id in &blocked_by {
            remove_reference(&mut data, target_id, "blocks", &id);
        }
        for target_id in &relates_to {
            remove_reference(&mut data, target_id, "relatesTo", &id);
        }
    }

    // Parent cleanup (mirrors :111-117): filter the id out of the parent's
    // `children` array, leaving the (possibly now-empty) array in place.
    if let Some(parent_id) = parent {
        if data.work_units.contains_key(&parent_id) {
            if let Some(wu) = data.work_units.get_mut(&parent_id) {
                if let Some(Value::Array(arr)) = wu.extra.get_mut("children") {
                    arr.retain(|v| v.as_str() != Some(id.as_str()));
                }
            }
        }
    }

    // State-index cleanup (mirrors :119-126): drop the id from every Kanban
    // state array.
    for state in [
        &mut data.states.backlog,
        &mut data.states.specifying,
        &mut data.states.testing,
        &mut data.states.implementing,
        &mut data.states.validating,
        &mut data.states.done,
        &mut data.states.blocked,
    ] {
        state.retain(|x| x != &id);
    }

    // Remove the work unit itself, preserving insertion order of the rest.
    data.work_units.shift_remove(&id);

    // Single atomic write (TS uses `fileManager.transaction`).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Render the CLI-equivalent output. The dispatcher exposes this verbatim
    // as `DispatchResult.data`; the CLI bridge prints it to stdout.
    let mut out = format!("✓ Work unit {id} deleted successfully\n");
    for warning in &warnings {
        out.push_str(&format!("⚠ {warning}\n"));
    }
    Ok(out)
}

/// Read a string array from a work unit's `extra` map, returning an empty
/// vector when the key is missing or not an array of strings.
fn str_array(extra: &Map<String, Value>, key: &str) -> Vec<String> {
    extra
        .get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Strip `id` from the `field` array on the `target` work unit, removing the
/// field entirely when the resulting array is empty (parity with the TS
/// `delete workUnit.<field>` branches at `src/commands/delete-work-unit.ts`).
fn remove_reference(data: &mut WorkUnitsData, target: &str, field: &str, id: &str) {
    if let Some(wu) = data.work_units.get_mut(target) {
        let now_empty = if let Some(Value::Array(arr)) = wu.extra.get_mut(field) {
            arr.retain(|v| v.as_str() != Some(id));
            arr.is_empty()
        } else {
            false
        };
        if now_empty {
            wu.extra.remove(field);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","cascadeDependencies":true}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.cascade_dependencies, Some(true));
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<DeleteWorkUnitArgs>("{}").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("workunitid"));
    }

    #[test]
    fn str_array_reads_string_arrays_and_tolerates_absence() {
        let mut m = Map::new();
        m.insert("blocks".into(), serde_json::json!(["A", "B"]));
        assert_eq!(str_array(&m, "blocks"), vec!["A".to_string(), "B".to_string()]);
        assert!(str_array(&m, "missing").is_empty());
    }
}
