//! `query-estimation-guide` — Rust port of `src/commands/query-estimation-guide.ts` (RPC-259).
//!
//! Returns estimation patterns derived from historical completed work units.
//! Groups by story-point estimate, computes iteration min/max, assigns
//! confidence based on sample size. Both invocation paths (LLM dispatcher AND
//! standalone CLI) call this single function — RPC-003 §7/§11 two-front-doors
//! invariant.
//!
//! ## TS parity rules
//!
//! * **Framing A divergence**: TS source-of-truth at
//!   `src/commands/query-estimation-guide.ts:38` calls `readFile` directly
//!   and errors out on ENOENT. The Rust port uses [`ensure_work_units_file`]
//!   to auto-create an empty canonical store — this is documented as a
//!   Framing A divergence: the canonical empty result `{patterns: []}` is
//!   more useful than an unhelpful ENOENT error.
//! * **Malformed JSON escalates** as `FspecCoreError::ParseJson` with
//!   `file = "work-units.json"`.
//! * **Filter**: only `status === 'done'` AND `estimate` present AND
//!   `iterations !== undefined`.
//! * **Bucket**: group by `estimate`.
//! * **Sort**: ascending by `points`.
//! * **Confidence**: `iterations.length >= 4 → high`, `>= 2 → medium`,
//!   otherwise `low`.
//! * **`expectedIterations`**: `"<min>-<max>"` string.
//! * **JSON field order**: points, expectedIterations, confidence.
//! * **Args**: `workUnitId` is consumed but discarded (TS parity — the
//!   reference implementation has the parameter in its signature but never
//!   uses it for filtering).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::WorkUnit;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryEstimationGuideArgs {
    /// Consumed but discarded (TS parity — used for clap surface only).
    #[serde(default)]
    #[allow(dead_code)]
    work_unit_id: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shape
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EstimationPattern {
    points: u64,
    expected_iterations: String,
    confidence: String,
}

#[derive(Debug, Serialize)]
struct QueryEstimationGuideResult {
    patterns: Vec<EstimationPattern>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryEstimationGuideArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-estimation-guide",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = ensure_work_units_file(project_root)?;
    let result = compute_guide(data.work_units.values());

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "query-estimation-guide",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // TS text path: silent (the CLI prints nothing when format !== 'json').
        _ => Ok(String::new()),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Computation
// ─────────────────────────────────────────────────────────────────────────

fn compute_guide<'a, I>(work_units: I) -> QueryEstimationGuideResult
where
    I: IntoIterator<Item = &'a WorkUnit>,
{
    // BTreeMap gives ascending key iteration order (parity with TS
    // `Object.entries` over integer-string keys — JS engines yield those in
    // ascending numeric order for integer-like keys).
    let mut by_points: BTreeMap<u64, Vec<u64>> = BTreeMap::new();

    for wu in work_units {
        if wu.status.as_str() != "done" {
            continue;
        }
        let estimate = match wu.extra.get("estimate").and_then(serde_json::Value::as_u64) {
            Some(e) => e,
            None => continue,
        };
        let iterations = match wu
            .extra
            .get("iterations")
            .and_then(serde_json::Value::as_u64)
        {
            Some(i) => i,
            None => continue,
        };
        by_points.entry(estimate).or_default().push(iterations);
    }

    let patterns: Vec<EstimationPattern> = by_points
        .into_iter()
        .filter_map(|(points, iters)| {
            // `or_default().push()` above guarantees `iters` is non-empty at
            // construction time. The `filter_map` here is defence-in-depth so
            // we never call `.expect()`/`.unwrap()` on an `Option` even when
            // unreachable.
            let min = *iters.iter().min()?;
            let max = *iters.iter().max()?;
            let confidence = if iters.len() >= 4 {
                "high"
            } else if iters.len() >= 2 {
                "medium"
            } else {
                "low"
            };
            Some(EstimationPattern {
                points,
                expected_iterations: format!("{min}-{max}"),
                confidence: confidence.to_string(),
            })
        })
        .collect();

    QueryEstimationGuideResult { patterns }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, status: &str, extras: serde_json::Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": format!("title {id}"),
            "status": status,
            "createdAt": "x",
            "updatedAt": "x"
        });
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
    fn empty_yields_empty_patterns() {
        let result = compute_guide(std::iter::empty());
        assert_eq!(result.patterns.len(), 0);
    }

    #[test]
    fn ignores_non_done_units() {
        let a = make_wu("A", "backlog", json!({"estimate": 3, "iterations": 1}));
        let b = make_wu("B", "implementing", json!({"estimate": 5, "iterations": 2}));
        let result = compute_guide([&a, &b].iter().copied());
        assert_eq!(result.patterns.len(), 0);
    }

    #[test]
    fn done_without_estimate_skipped() {
        let a = make_wu("A", "done", json!({"iterations": 1}));
        let result = compute_guide(std::iter::once(&a));
        assert_eq!(result.patterns.len(), 0);
    }

    #[test]
    fn done_without_iterations_skipped() {
        let a = make_wu("A", "done", json!({"estimate": 3}));
        let result = compute_guide(std::iter::once(&a));
        assert_eq!(result.patterns.len(), 0);
    }

    #[test]
    fn single_done_yields_low_confidence() {
        let a = make_wu("A", "done", json!({"estimate": 3, "iterations": 1}));
        let result = compute_guide(std::iter::once(&a));
        assert_eq!(result.patterns.len(), 1);
        assert_eq!(result.patterns[0].points, 3);
        assert_eq!(result.patterns[0].expected_iterations, "1-1");
        assert_eq!(result.patterns[0].confidence, "low");
    }

    #[test]
    fn two_done_units_medium_confidence() {
        let a = make_wu("A", "done", json!({"estimate": 3, "iterations": 1}));
        let b = make_wu("B", "done", json!({"estimate": 3, "iterations": 2}));
        let result = compute_guide([&a, &b].iter().copied());
        assert_eq!(result.patterns.len(), 1);
        assert_eq!(result.patterns[0].expected_iterations, "1-2");
        assert_eq!(result.patterns[0].confidence, "medium");
    }

    #[test]
    fn four_done_high_confidence() {
        let units: Vec<WorkUnit> = (1..=4)
            .map(|i| {
                make_wu(
                    &format!("U{i}"),
                    "done",
                    json!({"estimate": 5, "iterations": i}),
                )
            })
            .collect();
        let result = compute_guide(units.iter());
        assert_eq!(result.patterns.len(), 1);
        assert_eq!(result.patterns[0].points, 5);
        assert_eq!(result.patterns[0].expected_iterations, "1-4");
        assert_eq!(result.patterns[0].confidence, "high");
    }

    #[test]
    fn two_buckets_sorted_ascending() {
        let units: Vec<WorkUnit> = vec![
            make_wu("A", "done", json!({"estimate": 5, "iterations": 1})),
            make_wu("B", "done", json!({"estimate": 5, "iterations": 2})),
            make_wu("C", "done", json!({"estimate": 3, "iterations": 1})),
            make_wu("D", "done", json!({"estimate": 3, "iterations": 2})),
        ];
        let result = compute_guide(units.iter());
        assert_eq!(result.patterns.len(), 2);
        assert_eq!(result.patterns[0].points, 3);
        assert_eq!(result.patterns[1].points, 5);
    }

    #[test]
    fn field_declaration_order() {
        let a = make_wu("A", "done", json!({"estimate": 3, "iterations": 1}));
        let result = compute_guide(std::iter::once(&a));
        let s = serde_json::to_string_pretty(&result).unwrap();
        let expected = ["\"points\"", "\"expectedIterations\"", "\"confidence\""];
        let mut positions = Vec::new();
        for f in &expected {
            positions.push(s.find(f).unwrap_or_else(|| panic!("missing {f}\n{s}")));
        }
        for w in positions.windows(2) {
            assert!(w[0] < w[1], "field order violated: {positions:?}");
        }
    }
}
