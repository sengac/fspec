//! BUG-485: a feature's coverage file must list exactly the feature's
//! scenarios, so the card that adds or removes scenarios syncs the coverage
//! instead of a later `generate-coverage` changing files it never touched.
//!
//! Detection is exact: the scenario names the Gherkin parser reads (Rule
//! children included, an outline once), counted, against the `name` of each
//! coverage scenario. No step text is read and nothing is compared
//! semantically. A feature without a coverage file yet is not checked.
//! BUG-557: exact duplicate test or implementation mappings are reported too.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

/// One coverage sync problem: a stable `code` (`unreadable-coverage`,
/// `missing-entry`, `stale-entry`, `duplicate-mapping`), the scenario it
/// concerns, and the message `validate` prints. AUDIT-003: `fspec audit`'s
/// coverage area reports these same problems.
pub(crate) struct SyncProblem {
    pub(crate) code: &'static str,
    pub(crate) scenario: Option<String>,
    pub(crate) message: String,
}

impl SyncProblem {
    fn new(code: &'static str, scenario: Option<&str>, message: String) -> Self {
        SyncProblem {
            code,
            scenario: scenario.map(str::to_string),
            message,
        }
    }
}

/// The drift between the parsed `feature` at `feature_path` (project
/// relative) and its coverage file, one message per problem.
pub(super) fn coverage_drift(
    project_root: &Path,
    feature_path: &str,
    feature: &gherkin::Feature,
) -> Vec<String> {
    let coverage_path = format!("{feature_path}.coverage");
    let raw = match std::fs::read_to_string(project_root.join(&coverage_path)) {
        Ok(raw) => raw,
        // A feature gets its coverage file when its coverage is first
        // generated (create-feature and generate-scenarios create none), so a
        // missing file is not drift.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => return vec![format!("{coverage_path} cannot be read: {error}")],
    };
    sync_problems(&coverage_path, &raw, feature)
        .into_iter()
        .map(|problem| problem.message)
        .collect()
}

/// The sync problems between `feature` and the text `raw` of its coverage
/// file at `coverage_path`.
pub(crate) fn sync_problems(
    coverage_path: &str,
    raw: &str,
    feature: &gherkin::Feature,
) -> Vec<SyncProblem> {
    let declared = counts(
        feature
            .scenarios
            .iter()
            .chain(feature.rules.iter().flat_map(|rule| rule.scenarios.iter()))
            .map(|scenario| scenario.name.as_str()),
    );
    let unreadable = |message: String| vec![SyncProblem::new("unreadable-coverage", None, message)];
    let coverage: Value = match serde_json::from_str(raw) {
        Ok(coverage) => coverage,
        Err(error) => return unreadable(format!("{coverage_path} is not valid JSON: {error}")),
    };
    let Some(scenarios) = coverage.get("scenarios").and_then(Value::as_array) else {
        return unreadable(format!("{coverage_path} has no scenarios array"));
    };
    let listed = counts(
        scenarios
            .iter()
            .filter_map(|scenario| scenario.get("name").and_then(Value::as_str)),
    );
    let mut drift = Vec::new();
    for (name, count) in &declared {
        if listed.get(name).copied().unwrap_or(0) < *count {
            drift.push(SyncProblem::new(
                "missing-entry",
                Some(name),
                format!("{coverage_path} is missing the scenario \"{name}\""),
            ));
        }
    }
    for (name, count) in &listed {
        if declared.get(name).copied().unwrap_or(0) < *count {
            drift.push(SyncProblem::new(
                "stale-entry",
                Some(name),
                format!("{coverage_path} lists the scenario \"{name}\" the feature does not have"),
            ));
        }
    }
    drift.extend(duplicate_mappings(coverage_path, scenarios));
    drift
}

/// BUG-557: exact duplicate mappings, a second test mapping of a scenario
/// with the same file and lines text, or a second implementation mapping of
/// a test mapping with the same file and line list. Same-file mappings with
/// different ranges are allowed.
fn duplicate_mappings(coverage_path: &str, scenarios: &[Value]) -> Vec<SyncProblem> {
    let mut duplicates = Vec::new();
    for scenario in scenarios {
        let name = scenario
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let rows = scenario
            .get("testMappings")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut seen_rows: Vec<(&str, &str)> = Vec::new();
        for row in rows {
            let file = row.get("file").and_then(Value::as_str).unwrap_or_default();
            let lines = row.get("lines").and_then(Value::as_str).unwrap_or_default();
            if seen_rows.contains(&(file, lines)) {
                duplicates.push(SyncProblem::new(
                    "duplicate-mapping",
                    Some(name),
                    format!(
                        "{coverage_path} maps {file}:{lines} more than once in scenario \"{name}\""
                    ),
                ));
            } else {
                seen_rows.push((file, lines));
            }
            let impls = row
                .get("implMappings")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let mut seen_impls: Vec<(&str, &Value)> = Vec::new();
            for implementation in impls {
                let impl_file = implementation
                    .get("file")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let impl_lines = implementation.get("lines").unwrap_or(&Value::Null);
                if seen_impls.contains(&(impl_file, impl_lines)) {
                    duplicates.push(SyncProblem::new(
                        "duplicate-mapping",
                        Some(name),
                        format!(
                            "{coverage_path} maps the implementation {impl_file} {} more than once \
                             under {file}:{lines} in scenario \"{name}\"",
                            line_list(impl_lines)
                        ),
                    ));
                } else {
                    seen_impls.push((impl_file, impl_lines));
                }
            }
        }
    }
    duplicates
}

/// `[2, 3]` for a JSON line list.
fn line_list(lines: &Value) -> String {
    let numbers: Vec<String> = lines
        .as_array()
        .map(|lines| lines.iter().map(ToString::to_string).collect())
        .unwrap_or_default();
    format!("[{}]", numbers.join(", "))
}

fn counts<'a>(names: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut counted = BTreeMap::new();
    for name in names {
        *counted.entry(name).or_insert(0) += 1;
    }
    counted
}
