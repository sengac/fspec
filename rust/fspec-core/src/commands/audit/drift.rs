//! BUG-375: drift detection for `fspec audit`'s coverage area (AUDIT-003) —
//! a test mapping whose line range no longer holds an @step comment for every
//! step of its scenario.

use std::path::Path;

use std::collections::HashMap;

use super::source::Source;
use crate::commands::link_coverage::step::{missing_steps, parsed_scenario_steps};
use crate::error::FspecCoreError;
use crate::types::coverage::CoverageFile;

/// A test mapping that no longer covers its scenario's steps.
pub(crate) struct Drift {
    /// The scenario's mapped ranges, `file:lines` joined by ", ".
    pub(crate) mappings: String,
    pub(crate) scenario: String,
    /// AUDIT-003: `unreadable-range`, `outside-file`, `drifted` or
    /// `stale-entry` (the scenario is not in the feature), and the fact
    /// without the location.
    pub(crate) code: &'static str,
    pub(crate) fact: String,
    /// The first mapped test file.
    pub(crate) file: String,
}

/// Every scenario of `coverage` (for the feature whose text is
/// `feature_content`) whose mapped test ranges together lack an @step comment
/// for one of its steps. Mappings whose test file is missing are left to the
/// existence check.
pub(crate) fn drifted_mappings(
    project_root: &Path,
    source: &Source,
    feature_content: &str,
    coverage: &CoverageFile,
) -> Result<Vec<Drift>, FspecCoreError> {
    let feature = crate::io::gherkin::parse_feature_lenient(feature_content).ok();
    let mut test_files: HashMap<&str, String> = HashMap::new();
    let mut drifts = Vec::new();
    for scenario in &coverage.scenarios {
        // A scenario's steps may be spread over several mapped ranges; the
        // ranges together must hold every step.
        let mut mapped = Vec::new();
        let mut first_file = None;
        let mut text = String::new();
        for mapping in &scenario.test_mappings {
            if !source.exists(project_root, &mapping.file) {
                continue; // the existence check reports it
            }
            let reference = format!("{}:{}", mapping.file, mapping.lines);
            let Some(ranges) = line_ranges(&mapping.lines) else {
                let fact = format!("unreadable line range '{}'", mapping.lines);
                drifts.push(Drift {
                    mappings: reference,
                    scenario: scenario.name.clone(),
                    code: "unreadable-range",
                    fact,
                    file: mapping.file.clone(),
                });
                continue;
            };
            let test = match test_files.get(mapping.file.as_str()) {
                Some(test) => test,
                None => {
                    let test = source
                        .read(project_root, &mapping.file)?
                        .unwrap_or_default();
                    test_files.entry(mapping.file.as_str()).or_insert(test)
                }
            };
            // BUG-525: like link-coverage, a range must end inside the file.
            let length = test.lines().count();
            if ranges.iter().any(|(_, end)| *end > length) {
                let fact = format!("lies outside the file ({length} lines)");
                drifts.push(Drift {
                    mappings: reference.clone(),
                    scenario: scenario.name.clone(),
                    code: "outside-file",
                    fact,
                    file: mapping.file.clone(),
                });
            }
            for (start, end) in ranges {
                for line in test.lines().skip(start - 1).take(end + 1 - start) {
                    text.push_str(line);
                    text.push('\n');
                }
            }
            mapped.push(reference);
            first_file.get_or_insert_with(|| mapping.file.clone());
        }
        if mapped.is_empty() {
            continue;
        }
        let drift = |code: &'static str, problem: String| Drift {
            mappings: mapped.join(", "),
            scenario: scenario.name.clone(),
            code,
            fact: problem,
            file: first_file.clone().unwrap_or_default(),
        };
        match feature
            .as_ref()
            .and_then(|feature| parsed_scenario_steps(feature, &scenario.name))
        {
            None => drifts.push(drift(
                "stale-entry",
                "the scenario is not in the feature file".to_string(),
            )),
            Some(steps) => {
                if let Some(step) = missing_steps(&steps, &text).first() {
                    drifts.push(drift(
                        "drifted",
                        format!("no @step comment for '{step}' in the mapped lines"),
                    ));
                }
            }
        }
    }
    Ok(drifts)
}

/// BUG-501: a comma-separated list of [`line_range`]s (`"10-20,30-40"`,
/// `"45,46,47"`), the forms link-coverage accepts; `None` when any part is
/// unreadable.
pub(crate) fn line_ranges(lines: &str) -> Option<Vec<(usize, usize)>> {
    lines.split(',').map(line_range).collect()
}

/// `"a-b"` or `"a"` (1-based, inclusive).
pub(crate) fn line_range(lines: &str) -> Option<(usize, usize)> {
    let (start, end) = match lines.split_once('-') {
        Some((start, end)) => (start.trim().parse().ok()?, end.trim().parse().ok()?),
        None => {
            let line = lines.trim().parse().ok()?;
            (line, line)
        }
    };
    (start >= 1 && end >= start).then_some((start, end))
}
