//! `query-example-mapping-stats` — Rust port of
//! `src/commands/query-example-mapping-stats.ts` (RPC-260).
//!
//! Aggregates Example Mapping statistics across work units from
//! `spec/work-units.json`. Returns per-work-unit `completenessScore` plus
//! aggregate counts and averages of rules / examples / questions / assumptions.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## TS-parity rules
//!
//! * **Auto-create** of `spec/work-units.json` via [`ensure_work_units_file`]
//!   (parity with TS `ensureWorkUnitsFile(cwd)` at line 62).
//! * **Malformed JSON escalates** as `FspecCoreError::ParseJson` containing
//!   `"Failed to parse work-units.json"`.
//! * **`workUnitId` filter** — narrows to a single unit; missing id surfaces
//!   `"Work unit '<id>' does not exist"` (TS line 70).
//! * **`hasQuestions` filter** — `true` keeps units whose questions[] is
//!   non-empty; `false` keeps units whose questions[] is empty/missing.
//! * **`questionsFor` filter** — keeps units whose any question text
//!   contains `@<person>` substring (TS line 85: `mention = '@${person}'`).
//! * **`completenessScore`** — 33 (rules non-empty) + 34 (examples non-empty)
//!   + 33 (questions empty). All-three → 100. Examples-only → 67.
//!     Rules-only → 66.
//! * **JSON field order** matches TS struct-literal declaration:
//!   `workUnits, workUnitsWithRules, ..., avgAssumptionsPerWorkUnit`.
//! * **Text path is silent** — TS CLI prints nothing when `format !== 'json'`.

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
struct QueryExampleMappingStatsArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    has_questions: Option<bool>,
    #[serde(default)]
    questions_for: Option<String>,
    /// `"text"` (default — silent) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shapes — DECLARATION ORDER MATTERS (TS-parity field walk)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExampleMappingStats {
    work_unit_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    status: String,
    rules: usize,
    examples: usize,
    questions: usize,
    assumptions: usize,
    completeness_score: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryExampleMappingStatsResult {
    work_units: Vec<ExampleMappingStats>,
    work_units_with_rules: usize,
    work_units_with_examples: usize,
    work_units_with_questions: usize,
    work_units_with_assumptions: usize,
    /// Custom serializer demotes whole-number `f64` → `i64` so `1.0`
    /// renders as `1` (JS-parity for `JSON.stringify(1)`).
    #[serde(serialize_with = "serialize_avg")]
    avg_rules_per_work_unit: f64,
    #[serde(serialize_with = "serialize_avg")]
    avg_examples_per_work_unit: f64,
    #[serde(serialize_with = "serialize_avg")]
    avg_questions_per_work_unit: f64,
    #[serde(serialize_with = "serialize_avg")]
    avg_assumptions_per_work_unit: f64,
}

/// Serialize an `f64` as a JSON integer when it is whole and finite, and as
/// the natural decimal representation otherwise. Matches JS `JSON.stringify`
/// where `1.0 === 1` and prints `1` (no decimal point).
#[allow(clippy::trivially_copy_pass_by_ref)] // serde serialize_with signature
fn serialize_avg<S>(v: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if v.is_finite() && v.fract() == 0.0 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryExampleMappingStatsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-example-mapping-stats",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = ensure_work_units_file(project_root)?;
    let mut units: Vec<&WorkUnit> = data.work_units.values().collect();

    // Filter by specific work unit (TS lines 67-72).
    if let Some(ref id) = args.work_unit_id {
        units.retain(|wu| &wu.id == id);
        if units.is_empty() {
            return Err(FspecCoreError::InvalidArgs {
                command: "query-example-mapping-stats",
                reason: format!("Work unit '{id}' does not exist"),
            });
        }
    }

    // Filter by hasQuestions (TS lines 75-81).
    if let Some(has) = args.has_questions {
        if has {
            units.retain(|wu| array_len(wu, "questions") > 0);
        } else {
            units.retain(|wu| array_len(wu, "questions") == 0);
        }
    }

    // Filter by questionsFor person (TS lines 84-92).
    if let Some(ref person) = args.questions_for {
        let mention = format!("@{person}");
        units.retain(|wu| {
            array_string_entries(wu, "questions")
                .iter()
                .any(|q| q.contains(&mention))
        });
    }

    let result = compute_result(&units);

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "query-example-mapping-stats",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // TS CLI prints nothing for any format other than `"json"`.
        _ => Ok(String::new()),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Aggregation
// ─────────────────────────────────────────────────────────────────────────

fn compute_result(units: &[&WorkUnit]) -> QueryExampleMappingStatsResult {
    let stats: Vec<ExampleMappingStats> = units
        .iter()
        .map(|wu| {
            let rules = array_len(wu, "rules");
            let examples = array_len(wu, "examples");
            let questions = array_len(wu, "questions");
            let assumptions = array_len(wu, "assumptions");
            ExampleMappingStats {
                work_unit_id: wu.id.clone(),
                title: Some(wu.title.clone()),
                status: wu.status.as_str().to_string(),
                rules,
                examples,
                questions,
                assumptions,
                completeness_score: calculate_completeness_score(rules, examples, questions),
            }
        })
        .collect();

    let work_units_with_rules = units.iter().filter(|wu| array_len(wu, "rules") > 0).count();
    let work_units_with_examples =
        units.iter().filter(|wu| array_len(wu, "examples") > 0).count();
    let work_units_with_questions =
        units.iter().filter(|wu| array_len(wu, "questions") > 0).count();
    let work_units_with_assumptions = units
        .iter()
        .filter(|wu| array_len(wu, "assumptions") > 0)
        .count();

    let total_rules: usize = units.iter().map(|wu| array_len(wu, "rules")).sum();
    let total_examples: usize = units.iter().map(|wu| array_len(wu, "examples")).sum();
    let total_questions: usize = units.iter().map(|wu| array_len(wu, "questions")).sum();
    let total_assumptions: usize = units.iter().map(|wu| array_len(wu, "assumptions")).sum();

    let n = units.len();
    let avg = |total: usize| -> f64 {
        if n > 0 {
            total as f64 / n as f64
        } else {
            0.0
        }
    };

    QueryExampleMappingStatsResult {
        work_units: stats,
        work_units_with_rules,
        work_units_with_examples,
        work_units_with_questions,
        work_units_with_assumptions,
        avg_rules_per_work_unit: avg(total_rules),
        avg_examples_per_work_unit: avg(total_examples),
        avg_questions_per_work_unit: avg(total_questions),
        avg_assumptions_per_work_unit: avg(total_assumptions),
    }
}

/// TS `calculateCompletenessScore` (lines 37-54).
fn calculate_completeness_score(rules: usize, examples: usize, questions: usize) -> u32 {
    let mut score = 0_u32;
    if rules > 0 {
        score += 33;
    }
    if examples > 0 {
        score += 34;
    }
    if questions == 0 {
        score += 33;
    }
    score
}

/// Read the length of an array-typed extra field on a [`WorkUnit`]. Returns
/// `0` when the field is missing OR present-but-not-an-array.
fn array_len(wu: &WorkUnit, field: &str) -> usize {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr.len(),
        _ => 0,
    }
}

/// Collect all string entries of an array-typed extra field on a [`WorkUnit`].
/// Returns an empty vector when missing OR non-array.
fn array_string_entries<'a>(wu: &'a WorkUnit, field: &str) -> Vec<&'a str> {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, extra: serde_json::Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": "t",
            "status": "backlog",
            "createdAt": "x",
            "updatedAt": "x"
        });
        if let serde_json::Value::Object(ext) = extra {
            if let serde_json::Value::Object(ref mut base) = v {
                for (k, val) in ext {
                    base.insert(k, val);
                }
            }
        }
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn completeness_score_all_three_is_100() {
        assert_eq!(calculate_completeness_score(2, 1, 0), 100);
    }

    #[test]
    fn completeness_score_rules_only_is_66() {
        assert_eq!(calculate_completeness_score(1, 0, 0), 66);
    }

    #[test]
    fn completeness_score_examples_only_is_67() {
        assert_eq!(calculate_completeness_score(0, 1, 0), 67);
    }

    #[test]
    fn completeness_score_only_questions_is_0() {
        assert_eq!(calculate_completeness_score(0, 0, 1), 0);
    }

    #[test]
    fn array_len_returns_zero_for_missing_or_non_array() {
        let wu = make_wu("X", json!({}));
        assert_eq!(array_len(&wu, "rules"), 0);
        let wu = make_wu("X", json!({"rules": "not-array"}));
        assert_eq!(array_len(&wu, "rules"), 0);
        let wu = make_wu("X", json!({"rules": ["r0","r1"]}));
        assert_eq!(array_len(&wu, "rules"), 2);
    }

    #[test]
    fn integer_average_serializes_as_integer() {
        let a = make_wu("A", json!({"rules": ["r0","r1"]}));
        let units: Vec<&WorkUnit> = vec![&a];
        let result = compute_result(&units);
        let s = serde_json::to_string_pretty(&result).unwrap();
        assert!(s.contains("\"avgRulesPerWorkUnit\": 2,"), "got:\n{s}");
        assert!(!s.contains("avgRulesPerWorkUnit\": 2.0"));
    }

    #[test]
    fn decimal_average_serializes_as_decimal() {
        let a = make_wu("A", json!({"rules": ["r0"]}));
        let b = make_wu("B", json!({}));
        let units: Vec<&WorkUnit> = vec![&a, &b];
        let result = compute_result(&units);
        let s = serde_json::to_string_pretty(&result).unwrap();
        assert!(s.contains("\"avgRulesPerWorkUnit\": 0.5,"), "got:\n{s}");
    }

    #[test]
    fn field_order_is_declaration_order() {
        let a = make_wu("A", json!({"rules": ["r0"]}));
        let units: Vec<&WorkUnit> = vec![&a];
        let result = compute_result(&units);
        let s = serde_json::to_string_pretty(&result).unwrap();
        let expected = [
            "\"workUnits\"",
            "\"workUnitsWithRules\"",
            "\"workUnitsWithExamples\"",
            "\"workUnitsWithQuestions\"",
            "\"workUnitsWithAssumptions\"",
            "\"avgRulesPerWorkUnit\"",
            "\"avgExamplesPerWorkUnit\"",
            "\"avgQuestionsPerWorkUnit\"",
            "\"avgAssumptionsPerWorkUnit\"",
        ];
        let mut positions = Vec::new();
        for f in &expected {
            positions.push(s.find(f).unwrap_or_else(|| panic!("missing {f}\n{s}")));
        }
        for w in positions.windows(2) {
            assert!(w[0] < w[1], "field order violated: {positions:?}");
        }
    }

    #[test]
    fn array_string_entries_returns_only_strings() {
        let wu = make_wu(
            "A",
            json!({"questions": ["@alice cache?", 42, "@bob review"]}),
        );
        let entries = array_string_entries(&wu, "questions");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "@alice cache?");
        assert_eq!(entries[1], "@bob review");
    }
}
