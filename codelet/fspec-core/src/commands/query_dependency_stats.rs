//! `query-dependency-stats` — Rust port of `src/commands/query-dependency-stats.ts` (RPC-257).
//!
//! Aggregates dependency statistics from `spec/work-units.json`:
//!
//! * `total{Blocks,BlockedBy,DependsOn,RelatesTo}` — total ID-count across each
//!   of the four dependency arrays present on every WorkUnit
//! * `workUnitsWithDependencies` — count of units whose any-of-four arrays
//!   are non-empty (each unit counted once)
//! * `workUnitsBlockingOthers`, `workUnitsWithBlockers`,
//!   `workUnitsWithSoftDependencies` — per-array population indicators
//! * `averageDependenciesPerUnit` — total deps / total units, rounded to 2
//!   decimal places (`Math.round(x * 100) / 100`); integer-valued results
//!   serialize as integers (`1` not `1.0`) for JS-parity
//! * `maxDependencyChainDepth` — DFS over the `blocks` adjacency only, with a
//!   per-branch `visited` set to break cycles; self-cycle and dangling refs
//!   still contribute `+1` because the source unit owns a non-empty `blocks`
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## TS parity rules
//!
//! * **Auto-create** of `spec/work-units.json`: ENOENT → empty canonical store
//!   is materialised on disk via [`crate::io::ensure::ensure_work_units_file`],
//!   matching `queryDependencyStats` calling `ensureWorkUnitsFile(cwd)`.
//! * **Malformed JSON escalates**: parse failure surfaces as
//!   `FspecCoreError::ParseJson { file: "work-units.json", .. }` whose Display
//!   contains the canonical substring `"Failed to parse work-units.json"`.
//! * **Text path is silent**: the TS CLI registers `--format <format>`
//!   defaulting to `"text"` and prints output only when `format === 'json'`.
//!   We mirror that by returning the empty string for any format other than
//!   `"json"`. The CLI bridge prints whatever this function returns; the
//!   dispatcher always invokes with `format: "json"`.
//! * **JSON field order**: declaration order, NOT alphabetical. The struct
//!   below preserves that via Serde's struct-field walk; routing through
//!   `serde_json::json!` would alphabetize via `BTreeMap`.
//! * **Integer-valued average**: `1.0` must serialize as `1` (not `1.0`),
//!   matching `JSON.stringify(1)` in JS where `1.0 === 1`. We use a custom
//!   serializer that demotes whole-number `f64` to `i64`.

use std::path::Path;

use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::WorkUnit;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryDependencyStatsArgs {
    /// `"text"` (default — silent) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shape — DECLARATION ORDER MATTERS (TS-parity field walk)
// ─────────────────────────────────────────────────────────────────────────

/// In-memory shape returned by [`compute_stats`]. Mirrors the TS
/// `QueryDependencyStatsResult` interface at
/// `src/commands/query-dependency-stats.ts:54-65`.
///
/// `#[derive(Serialize)]` with explicit `#[serde(rename_all = "camelCase")]`
/// preserves the **declaration order** of the JSON fields, matching the TS
/// `JSON.stringify(result, null, 2)` output where object-literal insertion
/// order is honoured.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryDependencyStatsResult {
    total_blocks: usize,
    total_blocked_by: usize,
    total_depends_on: usize,
    total_relates_to: usize,
    work_units_with_dependencies: usize,
    work_units_with_blockers: usize,
    work_units_blocking_others: usize,
    work_units_with_soft_dependencies: usize,
    /// Custom serializer demotes whole-number `f64` → `i64` so `1.0`
    /// renders as `1` (JS-parity for `JSON.stringify(1)`).
    #[serde(serialize_with = "serialize_average_dependencies")]
    average_dependencies_per_unit: f64,
    max_dependency_chain_depth: usize,
}

/// Serialize an `f64` as a JSON integer when it is whole and finite, and as
/// the natural decimal representation otherwise. Matches JS `JSON.stringify`
/// where `1.0 === 1` and prints `1` (no decimal point), while `0.5` prints
/// `0.5`.
fn serialize_average_dependencies<S>(v: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if v.is_finite() && v.fract() == 0.0 {
        // Truncate-cast is safe here: TS implementation bounds the value
        // through `Math.round(x * 100) / 100` then we only enter this arm
        // when the fractional part is exactly zero. The total-deps numerator
        // is a usize sum and the unit count is bounded by IndexMap capacity,
        // so the result fits comfortably in i64.
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryDependencyStatsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-dependency-stats",
            reason: format!("failed to parse args: {e}"),
        })?;

    // TS-parity: ensureWorkUnitsFile auto-creates the canonical empty store
    // when missing, and ESCALATES malformed JSON. Both invariants are
    // covered by the dispatcher test suite.
    let data = ensure_work_units_file(project_root)?;
    let result = compute_stats(data.work_units.values());

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "query-dependency-stats",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // TS CLI prints nothing for any format other than `"json"`. We mirror
        // that contract by returning the empty string; the CLI bridge prints
        // whatever we return.
        _ => Ok(String::new()),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Aggregation
// ─────────────────────────────────────────────────────────────────────────

fn compute_stats<'a, I>(work_units_iter: I) -> QueryDependencyStatsResult
where
    I: IntoIterator<Item = &'a WorkUnit>,
{
    // Materialise the iterator so we can do a single linear pass AND a DFS
    // over the same set. Cloning is cheap (it's references to existing
    // structs owned by `WorkUnitsData`).
    let work_units: Vec<&WorkUnit> = work_units_iter.into_iter().collect();

    let mut total_blocks = 0_usize;
    let mut total_blocked_by = 0_usize;
    let mut total_depends_on = 0_usize;
    let mut total_relates_to = 0_usize;
    let mut work_units_with_dependencies = 0_usize;
    let mut work_units_with_blockers = 0_usize;
    let mut work_units_blocking_others = 0_usize;
    let mut work_units_with_soft_dependencies = 0_usize;

    for wu in &work_units {
        let mut has_dependencies = false;

        // Mirror the TS `workUnit.blocks?.length` short-circuit: a missing
        // OR empty array is a no-op; otherwise we bump both the total
        // count AND the per-array population indicator.
        let blocks_len = array_len(wu, "blocks");
        if blocks_len > 0 {
            total_blocks += blocks_len;
            work_units_blocking_others += 1;
            has_dependencies = true;
        }

        let blocked_by_len = array_len(wu, "blockedBy");
        if blocked_by_len > 0 {
            total_blocked_by += blocked_by_len;
            work_units_with_blockers += 1;
            has_dependencies = true;
        }

        let depends_on_len = array_len(wu, "dependsOn");
        if depends_on_len > 0 {
            total_depends_on += depends_on_len;
            work_units_with_soft_dependencies += 1;
            has_dependencies = true;
        }

        let relates_to_len = array_len(wu, "relatesTo");
        if relates_to_len > 0 {
            total_relates_to += relates_to_len;
            has_dependencies = true;
        }

        if has_dependencies {
            work_units_with_dependencies += 1;
        }
    }

    let total_dependencies =
        total_blocks + total_blocked_by + total_depends_on + total_relates_to;
    let avg_raw = if !work_units.is_empty() {
        total_dependencies as f64 / work_units.len() as f64
    } else {
        0.0
    };
    // TS: `Math.round(x * 100) / 100` — round half away from zero.
    let average_dependencies_per_unit = round_half_away_from_zero(avg_raw * 100.0) / 100.0;

    let max_dependency_chain_depth = calculate_max_chain_depth(&work_units);

    QueryDependencyStatsResult {
        total_blocks,
        total_blocked_by,
        total_depends_on,
        total_relates_to,
        work_units_with_dependencies,
        work_units_with_blockers,
        work_units_blocking_others,
        work_units_with_soft_dependencies,
        average_dependencies_per_unit,
        max_dependency_chain_depth,
    }
}

/// Read the length of an array-typed extra field on a [`WorkUnit`]. Returns
/// `0` when the field is missing OR present-but-not-an-array (defensive parity
/// with the TS `?.length` short-circuit which treats `undefined.length` as
/// falsy).
fn array_len(wu: &WorkUnit, field: &str) -> usize {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr.len(),
        _ => 0,
    }
}

/// Read the entries of an array-typed extra field on a [`WorkUnit`] as a
/// slice of `Value`s. Returns the empty slice when missing OR non-array.
fn array_entries<'a>(wu: &'a WorkUnit, field: &str) -> &'a [Value] {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr.as_slice(),
        _ => &[],
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Chain-depth DFS (over `blocks` adjacency only)
// ─────────────────────────────────────────────────────────────────────────

fn calculate_max_chain_depth(work_units: &[&WorkUnit]) -> usize {
    let mut max_depth = 0_usize;
    for wu in work_units {
        let depth =
            calculate_depth(work_units, &wu.id, &mut std::collections::HashSet::new());
        if depth > max_depth {
            max_depth = depth;
        }
    }
    max_depth
}

/// DFS returning the longest `blocks`-chain depth starting at `node_id`.
///
/// Mirrors `calculateDepth` in TS:
/// * `visited` is per-branch (cloned across recursive descents)
/// * a cycle on the current node returns `0` (the call site still adds `+1`
///   because the SOURCE unit owns a non-empty `blocks` array)
/// * a missing target unit returns `0` (same `+1` contribution at the source)
/// * the `+1` is added at the source ONLY when its `blocks` is non-empty
fn calculate_depth(
    work_units: &[&WorkUnit],
    node_id: &str,
    visited: &mut std::collections::HashSet<String>,
) -> usize {
    if visited.contains(node_id) {
        return 0;
    }
    visited.insert(node_id.to_string());

    let wu = match work_units.iter().find(|w| w.id == node_id) {
        Some(w) => *w,
        None => return 0,
    };

    let mut max_child_depth = 0_usize;
    let blocks = array_entries(wu, "blocks");
    for entry in blocks {
        if let Some(child_id) = entry.as_str() {
            // Per-branch visited set: clone so siblings don't pollute each
            // other's traversal — TS `new Set(visited)` semantic.
            let mut branch_visited = visited.clone();
            let child_depth = calculate_depth(work_units, child_id, &mut branch_visited);
            if child_depth > max_child_depth {
                max_child_depth = child_depth;
            }
        }
    }

    let plus_one = if blocks.is_empty() { 0 } else { 1 };
    max_child_depth + plus_one
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Round-half-away-from-zero. `Math.round` in JS rounds half-up for positive
/// values and half-toward-positive-infinity for negative ones. Our inputs
/// (sums of usizes scaled by 100) are always non-negative, so half-up via
/// `(x + 0.5).floor()` is equivalent and avoids an `if`.
fn round_half_away_from_zero(x: f64) -> f64 {
    (x + 0.5).floor()
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests for pure helpers
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, deps: serde_json::Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": "t",
            "status": "backlog",
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

    #[test]
    fn args_parse_with_defaults() {
        let a: QueryDependencyStatsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: QueryDependencyStatsArgs =
            serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn round_half_away_from_zero_is_half_up_for_non_negative() {
        assert_eq!(round_half_away_from_zero(0.5), 1.0);
        assert_eq!(round_half_away_from_zero(0.49), 0.0);
        assert_eq!(round_half_away_from_zero(1.5), 2.0);
        assert_eq!(round_half_away_from_zero(0.0), 0.0);
    }

    #[test]
    fn array_len_returns_zero_for_missing_or_non_array() {
        let wu = make_wu("X", json!({}));
        assert_eq!(array_len(&wu, "blocks"), 0);
        let wu = make_wu("X", json!({"blocks": "not-an-array"}));
        assert_eq!(array_len(&wu, "blocks"), 0);
        let wu = make_wu("X", json!({"blocks": ["A", "B"]}));
        assert_eq!(array_len(&wu, "blocks"), 2);
    }

    #[test]
    fn compute_stats_empty_returns_all_zero() {
        let result = compute_stats(std::iter::empty());
        let s = serde_json::to_string(&result).unwrap();
        assert!(s.contains("\"totalBlocks\":0"));
        assert!(s.contains("\"averageDependenciesPerUnit\":0"));
        assert!(s.contains("\"maxDependencyChainDepth\":0"));
    }

    #[test]
    fn compute_stats_self_cycle_yields_depth_one() {
        let wu = make_wu("A", json!({"blocks": ["A"]}));
        let result = compute_stats(std::iter::once(&wu));
        assert_eq!(result.total_blocks, 1);
        assert_eq!(result.max_dependency_chain_depth, 1);
    }

    #[test]
    fn compute_stats_dangling_ref_still_contributes_plus_one() {
        let wu = make_wu("A", json!({"blocks": ["MISSING"]}));
        let result = compute_stats(std::iter::once(&wu));
        assert_eq!(result.max_dependency_chain_depth, 1);
    }

    #[test]
    fn compute_stats_linear_three_chain_yields_depth_two() {
        let a = make_wu("A", json!({"blocks": ["B"]}));
        let b = make_wu("B", json!({"blocks": ["C"]}));
        let c = make_wu("C", json!({}));
        let result = compute_stats([&a, &b, &c].iter().copied());
        assert_eq!(result.max_dependency_chain_depth, 2);
        assert_eq!(result.total_blocks, 2);
    }

    #[test]
    fn integer_average_serializes_as_integer() {
        let a = make_wu("A", json!({"blocks": ["B", "C"]}));
        let b = make_wu("B", json!({"blockedBy": ["A"]}));
        let c = make_wu("C", json!({}));
        let result = compute_stats([&a, &b, &c].iter().copied());
        // 3 deps / 3 units = 1.0 → must render as `1`, not `1.0`.
        let s = serde_json::to_string_pretty(&result).unwrap();
        assert!(
            s.contains("\"averageDependenciesPerUnit\": 1,"),
            "expected integer-formatted line; got:\n{s}"
        );
        assert!(!s.contains("averageDependenciesPerUnit\": 1.0"));
    }

    #[test]
    fn decimal_average_serializes_as_decimal() {
        let a = make_wu("A", json!({"blocks": ["X"]}));
        let b = make_wu("B", json!({}));
        let result = compute_stats([&a, &b].iter().copied());
        let s = serde_json::to_string_pretty(&result).unwrap();
        assert!(
            s.contains("\"averageDependenciesPerUnit\": 0.5,"),
            "expected decimal-formatted line; got:\n{s}"
        );
    }

    #[test]
    fn field_order_is_declaration_order() {
        let a = make_wu("A", json!({"blocks": ["X"]}));
        let result = compute_stats(std::iter::once(&a));
        let s = serde_json::to_string_pretty(&result).unwrap();
        let expected = [
            "\"totalBlocks\"",
            "\"totalBlockedBy\"",
            "\"totalDependsOn\"",
            "\"totalRelatesTo\"",
            "\"workUnitsWithDependencies\"",
            "\"workUnitsWithBlockers\"",
            "\"workUnitsBlockingOthers\"",
            "\"workUnitsWithSoftDependencies\"",
            "\"averageDependenciesPerUnit\"",
            "\"maxDependencyChainDepth\"",
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
