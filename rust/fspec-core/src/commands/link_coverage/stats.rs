//! BUG-608: the one stats definition for coverage files. Every fspec command
//! that writes a coverage file sets its stats with [`update_stats`], and
//! `fspec audit` compares the stored stats with [`computed_stats`].
//!
//! Feature: spec/features/coverage-stats-match-their-scenarios.feature

use serde_json::{json, Map, Value};

/// The stats block of `coverage`, computed from its scenarios array:
/// `totalScenarios`, `coveredScenarios` (scenarios with a test mapping),
/// `coveragePercent` (rounded half up), `testFiles` and `implFiles`
/// (first-seen order) and `totalLinesCovered` = test line ranges (`"N-M"` →
/// M-N+1) + implementation line counts ([`impl_lines_len`]).
pub(crate) fn computed_stats(coverage: &Value) -> Map<String, Value> {
    let mut test_files: Vec<String> = Vec::new();
    let mut impl_files: Vec<String> = Vec::new();
    let mut total_lines: i64 = 0;
    let mut covered_scenarios: i64 = 0;
    let empty = Vec::new();
    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    for scenario in scenarios {
        let test_mappings = scenario
            .get("testMappings")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        if !test_mappings.is_empty() {
            covered_scenarios += 1;
        }
        for tm in test_mappings {
            if let Some(file) = tm.get("file").and_then(Value::as_str) {
                if !test_files.iter().any(|f| f == file) {
                    test_files.push(file.to_string());
                }
            }
            if let Some((start, end)) = tm
                .get("lines")
                .and_then(Value::as_str)
                .and_then(|lines| lines.split_once('-'))
            {
                if let (Ok(s), Ok(e)) = (start.parse::<i64>(), end.parse::<i64>()) {
                    total_lines += e - s + 1;
                }
            }
            for im in tm
                .get("implMappings")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
            {
                if let Some(file) = im.get("file").and_then(Value::as_str) {
                    if !impl_files.iter().any(|f| f == file) {
                        impl_files.push(file.to_string());
                    }
                }
                total_lines += impl_lines_len(im.get("lines"));
            }
        }
    }
    let total_scenarios = scenarios.len() as i64;
    let coverage_percent = if total_scenarios > 0 {
        ((covered_scenarios as f64) / (total_scenarios as f64) * 100.0).round() as i64
    } else {
        0
    };
    let mut stats = Map::new();
    stats.insert("totalScenarios".to_string(), json!(total_scenarios));
    stats.insert("coveredScenarios".to_string(), json!(covered_scenarios));
    stats.insert("coveragePercent".to_string(), json!(coverage_percent));
    stats.insert("testFiles".to_string(), json!(test_files));
    stats.insert("implFiles".to_string(), json!(impl_files));
    stats.insert("totalLinesCovered".to_string(), json!(total_lines));
    stats
}

/// Set the `stats` block of `coverage` to [`computed_stats`], creating it
/// when missing and keeping any other members it holds.
pub(crate) fn update_stats(coverage: &mut Value) {
    let computed = computed_stats(coverage);
    let Some(top) = coverage.as_object_mut() else {
        return;
    };
    let stats = top
        .entry("stats".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !stats.is_object() {
        *stats = Value::Object(Map::new());
    }
    if let Some(stats) = stats.as_object_mut() {
        stats.extend(computed);
    }
}

/// `implMapping.lines.length` parity: array length for `number[]`, char count
/// for the `"N-M"` string form, 0 otherwise.
pub(crate) fn impl_lines_len(lines: Option<&Value>) -> i64 {
    match lines {
        Some(Value::Array(a)) => a.len() as i64,
        Some(Value::String(s)) => s.chars().count() as i64,
        _ => 0,
    }
}
