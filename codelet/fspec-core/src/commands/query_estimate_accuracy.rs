//! `query-estimate-accuracy` — Rust port of `src/commands/query-estimate-accuracy.ts` (RPC-258).
//!
//! Reads `spec/work-units.json` and produces one of two payload shapes:
//!
//! * Single-unit query (when `workUnitId` is provided) — returns
//!   `{ estimated, actual, comparison }` with the canonical
//!   "0 tokens, N iterations" / "Within expected range" wording.
//! * All-completed aggregate (default) — returns `byStoryPoints` keyed by
//!   estimate; optionally also `byPrefix` when `byPrefix=true`.
//!
//! Behaviour parity with the TypeScript source (lines 41-162):
//!
//! * Spec file missing (ENOENT) → empty aggregate (`byStoryPoints: {}`).
//!   The file is **never** auto-created (parity with the TS read+throw
//!   path, since the All-completed mode never hits the wrapped error
//!   branch for ENOENT — the read throws, caught by the outer wrapper
//!   which re-throws with `Failed to query estimate accuracy:` prefix).
//!   In Rust we collapse the ENOENT branch into the empty result so the
//!   dispatcher does not surface a spurious failure for fresh workspaces.
//! * Malformed JSON → wrapped error whose message contains both
//!   `Failed to query estimate accuracy:` and `Failed to parse
//!   work-units.json` (parity with the TS try/catch wrapper).
//! * Unknown `workUnitId` → wrapped error containing
//!   `Failed to query estimate accuracy:` and `Work unit <id> not found`.
//! * `iterations` is read from the root field first, falling back to
//!   `metrics.iterations` (TS lines 63-67 and 89-92).
//! * `byStoryPoints` aggregation includes ONLY done work units whose
//!   `estimate` and `iterations` are both present (TS lines 94-101).
//! * `byStoryPoints` keys honour **first-encounter insertion order**
//!   (via `IndexMap`) — matching the TS `Object.entries` iteration order
//!   over the bucket map built up in the same first-encounter order.
//! * `avgIterations` rounds to one decimal via `round(x*10)/10` (TS line
//!   108). `avgAccuracy` formats via `.toFixed(1)` → `"N.N avg
//!   iterations"` (TS lines 139-144). `recommendation` pluralises
//!   `sample` based on count (TS line 143).
//!
//! Both invocation paths (dispatcher and standalone CLI bridge) call this
//! single function (RPC-003 §7/§11 two-front-doors invariant).

use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::error::FspecCoreError;

/// CLI arguments accepted by `query-estimate-accuracy`. Mirrors the union
/// of fields read at TS lines 41-46 and the Commander.js flag set at lines
/// 164-217 (`--format <format>`). `workUnitId` and `byPrefix` are
/// dispatcher-only flags (no CLI surface in the TS Commander wiring); the
/// dispatcher passes them through verbatim.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Args {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    by_prefix: bool,
    /// `"text"` (default) or `"json"`. Controls the dispatcher return
    /// payload only — the standalone CLI bridge picks its own rendering
    /// strategy on top of the structured result.
    #[serde(default)]
    format: Option<String>,
}

/// Single work-unit payload. Mirrors `SingleWorkUnitAccuracy` at TS line 20.
#[derive(Debug, Serialize)]
struct SingleResult {
    estimated: String,
    actual: String,
    comparison: String,
}

/// Per-story-point aggregation entry. Mirrors `AccuracyByPoints` at TS line 26.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PointsBucket {
    /// Custom serializer demotes whole-number `f64` → `i64` so `1.0`
    /// renders as `1` (JS-parity for `JSON.stringify(1)`).
    #[serde(serialize_with = "serialize_whole_number_f64")]
    avg_iterations: f64,
    samples: usize,
}

/// Serialize an `f64` as a JSON integer when it is whole and finite, and as
/// the natural decimal representation otherwise. Matches JS `JSON.stringify`
/// where `1.0 === 1` and prints `1` (no decimal point), while `1.5` prints
/// `1.5`.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde serialize_with signature
fn serialize_whole_number_f64<S>(v: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if v.is_finite() && v.fract() == 0.0 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

/// Per-prefix aggregation entry. Mirrors `PrefixAccuracy` at TS line 31.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrefixBucket {
    avg_accuracy: String,
    recommendation: String,
}

/// Aggregate payload. Mirrors `AllWorkUnitsAccuracy` at TS line 36 with
/// `byStoryPoints` always present (possibly empty) and `byPrefix` omitted
/// entirely when not requested.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AllResult {
    by_story_points: IndexMap<String, PointsBucket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_prefix: Option<IndexMap<String, PrefixBucket>>,
}

/// Dispatcher entry point. Both the LLM-facing dispatcher and the
/// standalone CLI bridge funnel through this function.
///
/// Rendering selection (parity with TS Commander default `'text'` at
/// `src/commands/query-estimate-accuracy.ts:168` and the `format === 'json'`
/// branch at line 175):
///
/// * `format == Some("json")` → pretty-printed JSON (2-space indent) of
///   either the single-unit payload or the aggregate.
/// * `format == None` or `Some("text")` or any other value → human-readable
///   text rendering of the aggregate (TS lines 180-211). Single-unit
///   queries always emit JSON regardless of `format` (the TS Commander
///   surface has no `--work-unit-id` flag, so text-rendering a single
///   payload is undefined; we pick JSON defensively).
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-estimate-accuracy",
        reason: format!("failed to parse args: {e}"),
    })?;

    let data = read_work_units(project_root)?;

    if let Some(id) = args.work_unit_id.as_deref() {
        let single = compute_single(&data, id)?;
        return to_pretty_json(&single);
    }

    let aggregate = compute_aggregate(&data, args.by_prefix);

    match args.format.as_deref() {
        Some("json") => to_pretty_json(&aggregate),
        _ => Ok(render_text(&aggregate)),
    }
}

/// Render the aggregate payload as the canonical human-readable text
/// block. Mirrors `src/commands/query-estimate-accuracy.ts:181-211`:
///
/// * Leading blank line then "📊 Estimation Accuracy Report" header.
/// * Empty aggregate → sentinel + guidance bullets (no per-bucket lines).
/// * Populated → "By Story Points:" section per bucket; "By Prefix:"
///   block only when `byPrefix` was requested AND yielded entries.
/// * Trailing blank line (TS `output.log()` at line 211).
///
/// avgIterations is rendered with the TS `${number}` toString semantics —
/// integral values lose their decimal (e.g. `2.0` → `2`), non-integral
/// values keep the one decimal place (e.g. `1.5` stays `1.5`).
fn render_text(agg: &AllResult) -> String {
    let mut out = String::new();
    out.push_str("\n📊 Estimation Accuracy Report\n\n");

    if agg.by_story_points.is_empty() {
        out.push_str("No completed work units with estimates and actuals found.\n");
        out.push_str("\nTo track accuracy, work units need:\n");
        out.push_str("  • Status: done\n");
        out.push_str("  • estimate field (story points)\n");
        out.push_str("  • iterations field\n\n");
        return out;
    }

    out.push_str("By Story Points:");
    for (points, metrics) in &agg.by_story_points {
        out.push_str(&format!("\n\n  {points} points:"));
        out.push_str(&format!(
            "\n    Average iterations: {}",
            format_avg(metrics.avg_iterations)
        ));
        out.push_str(&format!("\n    Samples: {}", metrics.samples));
    }

    if let Some(by_prefix) = agg.by_prefix.as_ref() {
        out.push_str("\n\n\nBy Prefix:");
        for (prefix, accuracy) in by_prefix {
            out.push_str(&format!("\n\n  {prefix}:"));
            out.push_str(&format!("\n    Accuracy: {}", accuracy.avg_accuracy));
            out.push_str(&format!(
                "\n    Recommendation: {}",
                accuracy.recommendation
            ));
        }
    }

    out.push_str("\n\n");
    out
}

/// Format an avgIterations value to match the TS template-literal
/// `${number}` semantics — integral floats drop their decimal, non-
/// integral floats keep one decimal place (since the upstream
/// computation has already rounded via `round(x*10)/10`).
fn format_avg(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Read `spec/work-units.json` with strict semantics:
/// * ENOENT → empty payload (no auto-create, no spec/ created).
/// * Parse failure → wrapped `Failed to query estimate accuracy: Failed
///   to parse work-units.json: …` error (parity with the TS try/catch
///   wrapper at lines 50-161).
fn read_work_units(project_root: &Path) -> Result<RawData, FspecCoreError> {
    let path = project_root.join("spec").join("work-units.json");
    if !path.exists() {
        return Ok(RawData::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-estimate-accuracy",
        reason: format!("Failed to query estimate accuracy: Failed to read work-units.json: {e}"),
    })?;
    serde_json::from_str::<RawData>(&raw).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-estimate-accuracy",
        reason: format!(
            "Failed to query estimate accuracy: Failed to parse work-units.json: {e}. The file may be corrupted or contain invalid JSON."
        ),
    })
}

/// On-disk shape consumed by this command — `workUnits` only, preserved as
/// raw `Value` per entry so we can field-probe both `iterations` and
/// `metrics.iterations` without a strict type contract. Insertion order
/// of the OUTER `workUnits` map is preserved via [`IndexMap`].
#[derive(Debug, Default, Deserialize)]
struct RawData {
    #[serde(rename = "workUnits", default)]
    work_units: IndexMap<String, Value>,
}

/// Compute the single-unit payload for the supplied `id`. Mirrors TS
/// lines 56-74.
fn compute_single(data: &RawData, id: &str) -> Result<SingleResult, FspecCoreError> {
    let wu = data
        .work_units
        .get(id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "query-estimate-accuracy",
            reason: format!("Failed to query estimate accuracy: Work unit {id} not found"),
        })?;

    let estimate = wu.get("estimate").and_then(Value::as_u64).unwrap_or(0);
    let iterations = read_iterations(wu).unwrap_or(0);

    Ok(SingleResult {
        estimated: format!("{estimate} points"),
        actual: format!("0 tokens, {iterations} iterations"),
        comparison: "Within expected range".to_string(),
    })
}

/// Compute the all-completed aggregate. Mirrors TS lines 76-155.
fn compute_aggregate(data: &RawData, by_prefix: bool) -> AllResult {
    // Phase 1: accumulate by story-point bucket — first-encounter insertion
    // order must be preserved (IndexMap on the accumulator).
    let mut bsp_accum: IndexMap<String, (u64, usize)> = IndexMap::new();
    let mut bp_accum: IndexMap<String, (u64, usize)> = IndexMap::new();

    for (id, wu) in &data.work_units {
        if wu.get("status").and_then(Value::as_str) != Some("done") {
            continue;
        }
        let estimate = match wu.get("estimate").and_then(Value::as_u64) {
            Some(v) => v,
            None => continue,
        };
        let iterations = match read_iterations(wu) {
            Some(v) => v,
            None => continue,
        };

        let key = estimate.to_string();
        let entry = bsp_accum.entry(key).or_insert((0, 0));
        entry.0 += iterations;
        entry.1 += 1;

        if by_prefix {
            let prefix = id.split('-').next().unwrap_or(id).to_string();
            let pentry = bp_accum.entry(prefix).or_insert((0, 0));
            pentry.0 += iterations;
            pentry.1 += 1;
        }
    }

    let mut by_story_points: IndexMap<String, PointsBucket> = IndexMap::new();
    // Collect into a temporary Vec so we can sort by the JS Object key-order
    // semantics: integer-like keys ascend numerically, all other keys keep
    // insertion order. This matches `Object.entries(byStoryPoints)` in TS.
    let mut buckets: Vec<(String, (u64, usize))> = bsp_accum.into_iter().collect();
    buckets.sort_by(|a, b| {
        let an = a.0.parse::<i64>().ok();
        let bn = b.0.parse::<i64>().ok();
        match (an, bn) {
            (Some(x), Some(y)) => x.cmp(&y),
            // Non-numeric keys stay relatively positioned (stable sort)
            _ => std::cmp::Ordering::Equal,
        }
    });
    for (k, (total, count)) in buckets {
        let avg = (total as f64) / (count as f64);
        by_story_points.insert(
            k,
            PointsBucket {
                avg_iterations: (avg * 10.0).round() / 10.0,
                samples: count,
            },
        );
    }

    let by_prefix_map = if by_prefix {
        let mut out: IndexMap<String, PrefixBucket> = IndexMap::new();
        for (prefix, (total, count)) in bp_accum {
            let avg = (total as f64) / (count as f64);
            out.insert(
                prefix,
                PrefixBucket {
                    avg_accuracy: format!("{avg:.1} avg iterations"),
                    recommendation: format!(
                        "{count} sample{}",
                        if count > 1 { "s" } else { "" }
                    ),
                },
            );
        }
        Some(out)
    } else {
        None
    };

    AllResult {
        by_story_points,
        by_prefix: by_prefix_map,
    }
}

/// Reads `iterations` from the root field first, falling back to
/// `metrics.iterations`. Mirrors TS lines 63-67 (single) and 89-92 (all)
/// — TS `wu.iterations || metrics?.iterations` short-circuits on `0`,
/// which here is preserved because `as_u64` for a literal `0` returns
/// `Some(0)` and the `?? metrics` branch in TS would also see `0` as
/// falsy. To match the TS truthy-coercion exactly, we treat root
/// `iterations == 0` as "missing" and fall through to the `metrics`
/// branch.
fn read_iterations(wu: &Value) -> Option<u64> {
    if let Some(v) = wu.get("iterations").and_then(Value::as_u64) {
        if v != 0 {
            return Some(v);
        }
    }
    wu.get("metrics")
        .and_then(|m| m.get("iterations"))
        .and_then(Value::as_u64)
}

/// Helper to serialize the payload as a pretty-printed JSON string with
/// the canonical 2-space indent. Wraps serde_json errors as InvalidArgs
/// so the dispatcher contract stays uniform.
fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-estimate-accuracy",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn args_parse_defaults_to_empty() {
        let a: Args = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(!a.by_prefix);
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case() {
        let a: Args = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","byPrefix":true,"format":"json"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert!(a.by_prefix);
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn read_iterations_prefers_root_then_metrics() {
        let v = json!({"iterations": 5, "metrics": {"iterations": 9}});
        assert_eq!(read_iterations(&v), Some(5));
        let v = json!({"metrics": {"iterations": 7}});
        assert_eq!(read_iterations(&v), Some(7));
        let v = json!({"iterations": 0, "metrics": {"iterations": 3}});
        // Root `0` is TS-falsy so we fall through to metrics.
        assert_eq!(read_iterations(&v), Some(3));
        let v = json!({});
        assert_eq!(read_iterations(&v), None);
    }
}
