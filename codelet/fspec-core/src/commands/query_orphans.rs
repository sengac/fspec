//! `query-orphans` — Rust port of `src/commands/query-orphans.ts` (RPC-262).
//!
//! Detects work units with NO epic assignment AND NO dependency relationships
//! (blocks, blockedBy, dependsOn, relatesTo). Both invocation paths (LLM
//! dispatcher AND standalone CLI) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## TS parity rules
//!
//! * **Auto-create**: ENOENT on `spec/work-units.json` → empty canonical store.
//! * **Malformed JSON escalates** as `FspecCoreError::ParseJson` with
//!   `file = "work-units.json"`.
//! * **Epic check**: `wu.epic && wu.epic.trim().length > 0`. Whitespace-only
//!   epic is treated as no-epic (parity with TS `.trim().length > 0`).
//! * **Relationship check**: any of `blocks`, `blockedBy`, `dependsOn`,
//!   `relatesTo` arrays with `length > 0`. Missing OR empty array → no
//!   relationship.
//! * **`excludeDone`**: optional flag — when true, units with status `done`
//!   are filtered out even if orphaned.
//! * **Iteration order**: `Object.values(data.workUnits)` — insertion order
//!   preserved via `IndexMap`.
//! * **JSON field order**: id, title, status, suggestedActions.
//! * **`suggestedActions`**: literal `["Assign epic", "Add relationship", "Delete"]`
//!   per orphan.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::WorkUnit;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryOrphansArgs {
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    exclude_done: Option<bool>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shape
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrphanedWorkUnit {
    id: String,
    title: String,
    status: String,
    suggested_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct QueryOrphansResult {
    orphans: Vec<OrphanedWorkUnit>,
}

const SUGGESTED_ACTIONS: [&str; 3] = ["Assign epic", "Add relationship", "Delete"];

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryOrphansArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-orphans",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = ensure_work_units_file(project_root)?;
    let exclude_done = args.exclude_done.unwrap_or(false);
    let result = compute_orphans(data.work_units.values(), exclude_done);

    match args.output.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "query-orphans",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        _ => Ok(render_text(&result)),
    }
}

/// Render the text-mode output matching TS `src/commands/query-orphans.ts:99-140`.
fn render_text(result: &QueryOrphansResult) -> String {
    if result.orphans.is_empty() {
        let mut out = String::new();
        out.push_str("✓ No orphaned work units found.\n");
        out.push_str("All work units have either an epic assignment or dependency relationships.");
        return out;
    }
    let mut out = String::new();
    out.push_str(&format!(
        "\nFound {} orphaned work unit(s):\n\n",
        result.orphans.len()
    ));
    for (i, orphan) in result.orphans.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} - {} ({})\n",
            i + 1,
            orphan.id,
            orphan.title,
            orphan.status
        ));
        out.push_str("   ⚠ No epic or dependency relationships\n");
        out.push_str("   Suggested actions:\n");
        for action in &orphan.suggested_actions {
            out.push_str(&format!("     • {action}\n"));
        }
        out.push('\n');
    }
    out.push_str("To fix orphaned work units:\n");
    out.push_str("  fspec update-work-unit <id> --epic=<epic-name>\n");
    out.push_str(
        "  fspec add-dependency <id> --depends-on=<other-id>  (or --blocks, --relates-to)\n",
    );
    out.push_str("  fspec delete-work-unit <id>");
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Computation
// ─────────────────────────────────────────────────────────────────────────

fn compute_orphans<'a, I>(work_units: I, exclude_done: bool) -> QueryOrphansResult
where
    I: IntoIterator<Item = &'a WorkUnit>,
{
    let mut orphans: Vec<OrphanedWorkUnit> = Vec::new();

    for wu in work_units {
        let has_epic = wu
            .epic
            .as_deref()
            .map(|e| !e.trim().is_empty())
            .unwrap_or(false);

        let has_relationships = has_non_empty_array(wu, "blocks")
            || has_non_empty_array(wu, "blockedBy")
            || has_non_empty_array(wu, "dependsOn")
            || has_non_empty_array(wu, "relatesTo");

        let is_orphaned = !has_epic && !has_relationships;
        if !is_orphaned {
            continue;
        }

        if exclude_done && wu.status.as_str() == "done" {
            continue;
        }

        orphans.push(OrphanedWorkUnit {
            id: wu.id.clone(),
            title: wu.title.clone(),
            status: wu.status.as_str().to_string(),
            suggested_actions: SUGGESTED_ACTIONS
                .iter()
                .copied()
                .map(String::from)
                .collect(),
        });
    }

    QueryOrphansResult { orphans }
}

fn has_non_empty_array(wu: &WorkUnit, field: &str) -> bool {
    matches!(wu.extra.get(field), Some(Value::Array(arr)) if !arr.is_empty())
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, status: &str, epic: Option<&str>, extras: serde_json::Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": format!("title {id}"),
            "status": status,
            "createdAt": "x",
            "updatedAt": "x"
        });
        if let Some(e) = epic {
            v["epic"] = json!(e);
        }
        if let serde_json::Value::Object(extra) = extras {
            if let serde_json::Value::Object(ref mut base) = v {
                for (k, val) in extra {
                    base.insert(k, val);
                }
            }
        }
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn non_blank_epic_is_not_orphaned() {
        let wu = make_wu("A", "backlog", Some("auth"), json!({}));
        let result = compute_orphans(std::iter::once(&wu), false);
        assert_eq!(result.orphans.len(), 0);
    }

    #[test]
    fn whitespace_epic_treated_as_no_epic() {
        let wu = make_wu("A", "backlog", Some("   "), json!({}));
        let result = compute_orphans(std::iter::once(&wu), false);
        assert_eq!(result.orphans.len(), 1);
    }

    #[test]
    fn non_empty_blocks_not_orphaned() {
        let wu = make_wu("A", "backlog", None, json!({"blocks": ["X"]}));
        let result = compute_orphans(std::iter::once(&wu), false);
        assert_eq!(result.orphans.len(), 0);
    }

    #[test]
    fn empty_arrays_treated_as_no_relationships() {
        let wu = make_wu(
            "A",
            "backlog",
            None,
            json!({"blocks": [], "blockedBy": [], "dependsOn": [], "relatesTo": []}),
        );
        let result = compute_orphans(std::iter::once(&wu), false);
        assert_eq!(result.orphans.len(), 1);
    }

    #[test]
    fn no_epic_no_relations_is_orphaned() {
        let wu = make_wu("A", "backlog", None, json!({}));
        let result = compute_orphans(std::iter::once(&wu), false);
        assert_eq!(result.orphans.len(), 1);
        assert_eq!(result.orphans[0].id, "A");
        assert_eq!(result.orphans[0].suggested_actions.len(), 3);
        assert_eq!(result.orphans[0].suggested_actions[0], "Assign epic");
    }

    #[test]
    fn exclude_done_filters_done_orphans() {
        let done = make_wu("DONE-1", "done", None, json!({}));
        let open = make_wu("OPEN-1", "backlog", None, json!({}));
        let r1 = compute_orphans([&done, &open].iter().copied(), false);
        let ids1: Vec<&str> = r1.orphans.iter().map(|o| o.id.as_str()).collect();
        assert!(ids1.contains(&"DONE-1"));
        assert!(ids1.contains(&"OPEN-1"));

        let r2 = compute_orphans([&done, &open].iter().copied(), true);
        let ids2: Vec<&str> = r2.orphans.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids2, vec!["OPEN-1"]);
    }

    #[test]
    fn field_declaration_order() {
        let wu = make_wu("ORPH-1", "backlog", None, json!({}));
        let result = compute_orphans(std::iter::once(&wu), false);
        let s = serde_json::to_string_pretty(&result).unwrap();
        let expected = ["\"id\"", "\"title\"", "\"status\"", "\"suggestedActions\""];
        let mut positions = Vec::new();
        for f in &expected {
            positions.push(s.find(f).unwrap_or_else(|| panic!("missing {f}\n{s}")));
        }
        for w in positions.windows(2) {
            assert!(w[0] < w[1], "field order violated: {positions:?}");
        }
    }
}
