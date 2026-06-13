//! `query-bottlenecks` — Rust port of `src/commands/query-bottlenecks.ts` (RPC-256).
//!
//! Identifies work units that block 2+ downstream work units (direct + transitive),
//! ranked by descending bottleneck score. Both invocation paths (LLM dispatcher
//! AND standalone CLI) call this single function — RPC-003 §7/§11 two-front-doors
//! invariant.
//!
//! ## TS parity rules
//!
//! * **Auto-create**: ENOENT on `spec/work-units.json` → empty canonical store
//!   materialised on disk via [`ensure_work_units_file`]. Matches
//!   `queryBottlenecks` calling `ensureWorkUnitsFile(cwd)`.
//! * **Malformed JSON escalates** as `FspecCoreError::ParseJson { file:
//!   "work-units.json", .. }` whose Display contains `"Failed to parse
//!   work-units.json"`.
//! * **Status filter**: `done` and `blocked` are skipped (cannot be bottlenecks).
//! * **Empty blocks**: a work unit with missing OR empty `blocks` is skipped.
//! * **Score threshold**: only bottlenecks with `score >= 2` are emitted.
//! * **DFS visited-set**: per-branch clone (TS `new Set(visited)` semantic) —
//!   cycles bottom out at `0`, the source unit still contributes via its own
//!   non-empty `blocks` array. A cycle A→B→A yields `score=2` because both A
//!   and B contribute to the blocked set even though the cycle terminates.
//! * **Sort**: descending by score. Rust `sort_by` is stable so ties preserve
//!   IndexMap insertion order (parity with TS `Array.prototype.sort`).
//! * **Direct vs transitive**: `directBlocks` is the verbatim `blocks` array
//!   in source order. `transitiveBlocks` is the blocked-set MINUS direct
//!   entries, in DFS discovery order (preserved via IndexSet).

use std::path::Path;

use indexmap::IndexSet;
use serde::Serialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::{WorkUnit, WorkUnitStatus};

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryBottlenecksArgs {
    /// `"text"` (default) or `"json"`. Mirrors TS `--output <format>`.
    #[serde(default)]
    output: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shape — DECLARATION ORDER MATTERS (TS-parity field walk)
// ─────────────────────────────────────────────────────────────────────────

/// In-memory bottleneck record. Field declaration order is the JSON output
/// order: id, title, status, score, directBlocks, transitiveBlocks.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bottleneck {
    id: String,
    title: String,
    status: String,
    score: usize,
    direct_blocks: Vec<String>,
    transitive_blocks: Vec<String>,
}

#[derive(Debug, Serialize)]
struct QueryBottlenecksResult {
    bottlenecks: Vec<Bottleneck>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryBottlenecksArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-bottlenecks",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = ensure_work_units_file(project_root)?;
    let result = compute_bottlenecks(&data.work_units);

    match args.output.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "query-bottlenecks",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        _ => Ok(render_text(&result)),
    }
}

/// Render the text-mode output matching TS `src/commands/query-bottlenecks.ts:138-172`.
fn render_text(result: &QueryBottlenecksResult) -> String {
    if result.bottlenecks.is_empty() {
        return "✓ No bottlenecks found".to_string();
    }
    let mut out = String::new();
    out.push_str("Bottleneck Work Units (blocking 2+ work units):\n\n");
    for b in &result.bottlenecks {
        out.push_str(&format!("{} ({}) - {}\n", b.id, b.status, b.title));
        out.push_str(&format!("  Bottleneck Score: {}\n", b.score));
        out.push_str(&format!(
            "  Direct Blocks: {}\n",
            b.direct_blocks.join(", ")
        ));
        if !b.transitive_blocks.is_empty() {
            out.push_str(&format!(
                "  Transitive Blocks: {}\n",
                b.transitive_blocks.join(", ")
            ));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "\nTotal bottlenecks: {}",
        result.bottlenecks.len()
    ));
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Computation
// ─────────────────────────────────────────────────────────────────────────

fn compute_bottlenecks(
    work_units: &indexmap::IndexMap<String, WorkUnit>,
) -> QueryBottlenecksResult {
    let mut bottlenecks: Vec<Bottleneck> = Vec::new();

    for wu in work_units.values() {
        // Rule 2: skip 'done' units.
        if matches!(wu.status, WorkUnitStatus::Done) {
            continue;
        }
        // Rule 7: skip 'blocked' status units (cannot progress).
        if matches!(wu.status, WorkUnitStatus::Blocked) {
            continue;
        }

        let blocks = read_blocks(wu);
        if blocks.is_empty() {
            continue;
        }

        // Calculate transitive closure of blocked work units.
        let mut blocked_set: IndexSet<String> = IndexSet::new();
        for direct_id in &blocks {
            blocked_set.insert(direct_id.clone());
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            visited.insert(wu.id.clone());
            collect_transitive(work_units, direct_id, &mut visited, &mut blocked_set);
        }

        let score = blocked_set.len();
        if score < 2 {
            continue;
        }

        let direct_blocks: Vec<String> = blocks.clone();
        let transitive_blocks: Vec<String> = blocked_set
            .iter()
            .filter(|id| !blocks.contains(id))
            .cloned()
            .collect();

        bottlenecks.push(Bottleneck {
            id: wu.id.clone(),
            title: wu.title.clone(),
            status: wu.status.as_str().to_string(),
            score,
            direct_blocks,
            transitive_blocks,
        });
    }

    // Rule 4: rank by score descending. Stable sort preserves IndexMap order
    // for ties (TS Array.prototype.sort parity).
    bottlenecks.sort_by_key(|b| std::cmp::Reverse(b.score));

    QueryBottlenecksResult { bottlenecks }
}

/// DFS over the `blocks` adjacency from `node_id`, inserting each visited
/// (non-cycling) ID into `accumulator`. The visited set is cloned per branch
/// to mirror TS `new Set(visited)` semantics — siblings do not pollute each
/// other's traversal.
fn collect_transitive(
    work_units: &indexmap::IndexMap<String, WorkUnit>,
    node_id: &str,
    visited: &mut std::collections::HashSet<String>,
    accumulator: &mut IndexSet<String>,
) {
    if visited.contains(node_id) {
        return;
    }
    visited.insert(node_id.to_string());

    let Some(wu) = work_units.get(node_id) else {
        return;
    };

    let children = read_blocks(wu);
    for child_id in &children {
        accumulator.insert(child_id.clone());
        let mut branch_visited = visited.clone();
        collect_transitive(work_units, child_id, &mut branch_visited, accumulator);
    }
}

/// Read the `blocks` array from a WorkUnit's `extra` map. Returns empty Vec
/// when missing OR not an array.
fn read_blocks(wu: &WorkUnit) -> Vec<String> {
    match wu.extra.get("blocks") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;

    fn make_wu(id: &str, status: &str, deps: serde_json::Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": format!("title {id}"),
            "status": status,
            "createdAt": "x",
            "updatedAt": "x"
        });
        if let serde_json::Value::Object(extra) = deps {
            if let serde_json::Value::Object(ref mut base) = v {
                for (k, val) in extra {
                    base.insert(k, val);
                }
            }
        }
        serde_json::from_value(v).unwrap()
    }

    fn map_of(entries: Vec<WorkUnit>) -> IndexMap<String, WorkUnit> {
        let mut m = IndexMap::new();
        for wu in entries {
            m.insert(wu.id.clone(), wu);
        }
        m
    }

    #[test]
    fn empty_workspace_yields_no_bottlenecks() {
        let result = compute_bottlenecks(&IndexMap::new());
        assert_eq!(result.bottlenecks.len(), 0);
    }

    #[test]
    fn done_status_is_excluded() {
        let units = map_of(vec![
            make_wu("A", "done", json!({"blocks": ["B", "C"]})),
            make_wu("B", "backlog", json!({})),
            make_wu("C", "backlog", json!({})),
        ]);
        let result = compute_bottlenecks(&units);
        assert!(result.bottlenecks.iter().all(|b| b.id != "A"));
    }

    #[test]
    fn blocked_status_is_excluded() {
        let units = map_of(vec![
            make_wu("A", "blocked", json!({"blocks": ["B", "C"]})),
            make_wu("B", "backlog", json!({})),
            make_wu("C", "backlog", json!({})),
        ]);
        let result = compute_bottlenecks(&units);
        assert!(result.bottlenecks.iter().all(|b| b.id != "A"));
    }

    #[test]
    fn cycle_yields_score_two() {
        let units = map_of(vec![
            make_wu("A", "backlog", json!({"blocks": ["B"]})),
            make_wu("B", "backlog", json!({"blocks": ["A"]})),
        ]);
        let result = compute_bottlenecks(&units);
        let a = result.bottlenecks.iter().find(|b| b.id == "A").unwrap();
        assert_eq!(a.score, 2);
        assert_eq!(a.direct_blocks, vec!["B"]);
        assert_eq!(a.transitive_blocks, vec!["A"]);
    }

    #[test]
    fn direct_plus_transitive_yields_score_three() {
        let units = map_of(vec![
            make_wu("A", "backlog", json!({"blocks": ["B", "C"]})),
            make_wu("B", "backlog", json!({"blocks": ["D"]})),
            make_wu("C", "backlog", json!({})),
            make_wu("D", "backlog", json!({})),
        ]);
        let result = compute_bottlenecks(&units);
        assert_eq!(result.bottlenecks.len(), 1);
        assert_eq!(result.bottlenecks[0].id, "A");
        assert_eq!(result.bottlenecks[0].score, 3);
        assert_eq!(result.bottlenecks[0].direct_blocks, vec!["B", "C"]);
        assert_eq!(result.bottlenecks[0].transitive_blocks, vec!["D"]);
    }

    #[test]
    fn sort_descending_by_score() {
        let units = map_of(vec![
            make_wu("E", "backlog", json!({"blocks": ["F", "G", "H"]})),
            make_wu("F", "backlog", json!({})),
            make_wu("G", "backlog", json!({})),
            make_wu("H", "backlog", json!({})),
            make_wu("A", "backlog", json!({"blocks": ["B", "C", "D"]})),
            make_wu("B", "backlog", json!({"blocks": ["X"]})),
            make_wu("C", "backlog", json!({})),
            make_wu("D", "backlog", json!({})),
            make_wu("X", "backlog", json!({})),
        ]);
        let result = compute_bottlenecks(&units);
        assert_eq!(result.bottlenecks[0].id, "A");
        assert!(result.bottlenecks[0].score >= 4);
        assert_eq!(result.bottlenecks[1].id, "E");
        assert_eq!(result.bottlenecks[1].score, 3);
    }

    #[test]
    fn field_declaration_order_in_json() {
        let units = map_of(vec![
            make_wu("A", "backlog", json!({"blocks": ["B", "C"]})),
            make_wu("B", "backlog", json!({})),
            make_wu("C", "backlog", json!({})),
        ]);
        let result = compute_bottlenecks(&units);
        let s = serde_json::to_string_pretty(&result).unwrap();
        let expected = [
            "\"id\"",
            "\"title\"",
            "\"status\"",
            "\"score\"",
            "\"directBlocks\"",
            "\"transitiveBlocks\"",
        ];
        let mut positions = Vec::new();
        for f in &expected {
            positions.push(s.find(f).unwrap_or_else(|| panic!("missing {f}\n{s}")));
        }
        for w in positions.windows(2) {
            assert!(w[0] < w[1], "field order violated: {positions:?}");
        }
    }
}
