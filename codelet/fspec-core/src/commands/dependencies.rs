//! `dependencies` — Rust port of `showDependencies` in
//! `src/commands/dependencies.ts` (RPC-224).
//!
//! Read-only command: shows all dependency relationships for a single work
//! unit. Two rendering modes:
//!
//! * **default (text)** — a header line `Dependencies for <id>:` followed by
//!   one indented line per non-empty relationship array, in the FIXED order
//!   `Blocks`, `Blocked by`, `Depends on`, `Related to`. Each emitted line —
//!   including the header — ends with `\n` (parity with the TS string
//!   concatenation at `src/commands/dependencies.ts:817-830`).
//! * **`--graph`** — a depth-first `blocks`-only tree. EVERY visited node is
//!   printed as `<indent><id>` (the root AND each recursed child), and each
//!   `blocks` edge is rendered as `<indent>  blocks → <target>`. Recursion
//!   increases the indent by TWO levels (four spaces) per the TS
//!   `traverse(blockedId, indent + 2)` call at
//!   `src/commands/dependencies.ts:850`. The rendered tree is joined with `\n`
//!   WITHOUT a trailing newline (parity with the TS `graphLines.join('\n')` at
//!   `src/commands/dependencies.ts:856`).
//!
//! ## Relationship resolution (default mode)
//!
//! Mirrors the TS `workUnit.<field> || workUnit.relationships?.<field> || []`
//! short-circuit (`src/commands/dependencies.ts:809-815`): a legacy top-level
//! array (`blocks`, `blockedBy`, `dependsOn`, `relatesTo`) takes precedence;
//! otherwise the same-named key inside the `relationships` object is used.
//! Graph mode reads ONLY `relationships.blocks` (parity with the TS
//! `unit.relationships?.blocks` traversal at line 847).
//!
//! ## Errors
//!
//! A missing work unit surfaces as `FspecCoreError::InvalidArgs` whose reason
//! is `Work unit '<id>' does not exist` — the dispatcher maps this to
//! `{ success: false, error: Some("...does not exist...") }`. The work-units
//! store is read directly (NO auto-create) — parity with the TS `loadWorkUnits`
//! which `readFile`s `spec/work-units.json` without an ensure step.
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge
//! both call this single function — no inline rendering elsewhere.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DependenciesArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    graph: bool,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DependenciesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "dependencies",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args.work_unit_id.ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "dependencies",
        reason: "missing required argument: workUnitId".to_string(),
    })?;

    let data = load_work_units(project_root)?;

    // TS: `if (!workUnitsData.workUnits[workUnitId]) throw ...does not exist`.
    if !data.work_units.contains_key(&work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "dependencies",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        });
    }

    if args.graph {
        Ok(render_graph(&data, &work_unit_id))
    } else {
        let wu = &data.work_units[&work_unit_id];
        Ok(render_text(&work_unit_id, wu))
    }
}

/// Read `spec/work-units.json` directly WITHOUT auto-creating it (parity with
/// the TS `loadWorkUnits` which `readFile`s the file). A missing file escalates
/// as [`FspecCoreError::Io`]; a malformed file escalates as
/// [`FspecCoreError::ParseJson`] with `file = "work-units.json"`.
fn load_work_units(project_root: &Path) -> Result<WorkUnitsData, FspecCoreError> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "dependencies",
        source,
    })?;
    serde_json::from_str::<WorkUnitsData>(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "work-units.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })
}

/// Render the default (non-graph) text view. Mirrors the TS string
/// concatenation at `src/commands/dependencies.ts:817-830`: header line plus
/// one indented line per non-empty relationship array, each terminated with
/// `\n`.
fn render_text(work_unit_id: &str, wu: &WorkUnit) -> String {
    let blocks = resolve_relationship(wu, "blocks");
    let blocked_by = resolve_relationship(wu, "blockedBy");
    let depends_on = resolve_relationship(wu, "dependsOn");
    let relates_to = resolve_relationship(wu, "relatesTo");

    let mut out = format!("Dependencies for {work_unit_id}:\n");
    if !blocks.is_empty() {
        out.push_str(&format!("  Blocks: {}\n", blocks.join(", ")));
    }
    if !blocked_by.is_empty() {
        out.push_str(&format!("  Blocked by: {}\n", blocked_by.join(", ")));
    }
    if !depends_on.is_empty() {
        out.push_str(&format!("  Depends on: {}\n", depends_on.join(", ")));
    }
    if !relates_to.is_empty() {
        out.push_str(&format!("  Related to: {}\n", relates_to.join(", ")));
    }
    out
}

/// Render the `--graph` depth-first `blocks`-only tree. EVERY visited node
/// prints `<indent><id>` (root AND each recursed child); each `blocks` edge
/// becomes `<indent>  blocks → <target>` and recursion increases the indent
/// by TWO levels (four spaces) per the TS `traverse(blockedId, indent + 2)`
/// call. Cycles are broken via a `visited` set. Joined with `\n` WITHOUT a
/// trailing newline (parity with `graphLines.join('\n')`).
fn render_graph(data: &WorkUnitsData, root: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut visited = std::collections::HashSet::new();
    traverse(data, root, 0, &mut visited, &mut lines);
    lines.join("\n")
}

fn traverse(
    data: &WorkUnitsData,
    id: &str,
    indent: usize,
    visited: &mut std::collections::HashSet<String>,
    lines: &mut Vec<String>,
) {
    if visited.contains(id) {
        return;
    }
    visited.insert(id.to_string());

    let Some(wu) = data.work_units.get(id) else {
        return;
    };

    // Parity with TS `traverse` (src/commands/dependencies.ts:837-853):
    // EVERY visited node prints `<prefix><id>` (root AND recursed children),
    // and recursion increases indent by 2 (not 1).
    let prefix = "  ".repeat(indent);
    lines.push(format!("{prefix}{id}"));

    let blocks = relationship_blocks(wu);
    for child in blocks {
        lines.push(format!("{prefix}  blocks → {child}"));
        traverse(data, &child, indent + 2, visited, lines);
    }
}

/// Resolve a relationship array with the TS short-circuit
/// `workUnit.<field> || workUnit.relationships?.<field> || []`: a legacy
/// top-level array takes precedence; otherwise the same-named key inside the
/// `relationships` object is used. Returns the string ids (non-string entries
/// are skipped defensively).
fn resolve_relationship(wu: &WorkUnit, field: &str) -> Vec<String> {
    if let Some(Value::Array(arr)) = wu.extra.get(field) {
        return string_ids(arr);
    }
    if let Some(Value::Object(rel)) = wu.extra.get("relationships") {
        if let Some(Value::Array(arr)) = rel.get(field) {
            return string_ids(arr);
        }
    }
    Vec::new()
}

/// Graph traversal reads ONLY `relationships.blocks` (parity with the TS
/// `unit.relationships?.blocks` traversal at `src/commands/dependencies.ts:847`).
fn relationship_blocks(wu: &WorkUnit) -> Vec<String> {
    if let Some(Value::Object(rel)) = wu.extra.get("relationships") {
        if let Some(Value::Array(arr)) = rel.get("blocks") {
            return string_ids(arr);
        }
    }
    Vec::new()
}

fn string_ids(arr: &[Value]) -> Vec<String> {
    arr.iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, extra: Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": "t",
            "status": "backlog",
            "createdAt": "x",
            "updatedAt": "x"
        });
        if let (Value::Object(base), Value::Object(ex)) = (&mut v, extra) {
            for (k, val) in ex {
                base.insert(k, val);
            }
        }
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn args_parse_defaults() {
        let a: DependenciesArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(!a.graph);
    }

    #[test]
    fn resolve_prefers_top_level_then_relationships() {
        let wu = make_wu("A", json!({ "dependsOn": ["B", "C"] }));
        assert_eq!(resolve_relationship(&wu, "dependsOn"), vec!["B", "C"]);

        let wu = make_wu("A", json!({ "relationships": { "blocks": ["X"] } }));
        assert_eq!(resolve_relationship(&wu, "blocks"), vec!["X"]);
        assert!(resolve_relationship(&wu, "dependsOn").is_empty());
    }

    #[test]
    fn render_text_header_only_when_no_relationships() {
        let wu = make_wu("MCP-001", json!({}));
        assert_eq!(render_text("MCP-001", &wu), "Dependencies for MCP-001:\n");
    }

    #[test]
    fn render_text_fixed_order() {
        let wu = make_wu(
            "AUTH-001",
            json!({ "relationships": {
                "blocks": ["AUTH-002", "AUTH-003"],
                "blockedBy": ["INFRA-001"],
                "dependsOn": ["SCHEMA-001"],
                "relatesTo": ["DOC-001"]
            }}),
        );
        let expected = "Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001\n";
        assert_eq!(render_text("AUTH-001", &wu), expected);
    }

    #[test]
    fn render_graph_prints_every_node_and_indents_by_two_levels() {
        // Parity with TS `traverse` (src/commands/dependencies.ts:837-856):
        // every visited node prints `<prefix><id>`, recursion is `indent + 2`,
        // and the joined tree has NO trailing newline.
        let mut data = WorkUnitsData::initial("x");
        for (id, extra) in [
            (
                "AUTH-001",
                json!({ "relationships": { "blocks": ["AUTH-002", "AUTH-003"] } }),
            ),
            (
                "AUTH-002",
                json!({ "relationships": { "blocks": ["AUTH-004"] } }),
            ),
            ("AUTH-003", json!({})),
            ("AUTH-004", json!({})),
        ] {
            data.work_units.insert(id.to_string(), make_wu(id, extra));
        }
        let expected = "AUTH-001\n  blocks → AUTH-002\n    AUTH-002\n      blocks → AUTH-004\n        AUTH-004\n  blocks → AUTH-003\n    AUTH-003";
        assert_eq!(render_graph(&data, "AUTH-001"), expected);
    }
}
