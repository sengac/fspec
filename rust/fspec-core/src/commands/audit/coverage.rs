//! The coverage area of `fspec audit` (AUDIT-003): the BUG-485/557 sync
//! check, missing files, the BUG-375/501/509/525 test-range checks, the
//! BUG-515 implementation-range check, BUG-357 shared test files and
//! uncovered scenarios; BUG-608: stats that do not match the scenarios.
//!
//! COV-078: a drifted, out-of-range or missing test mapping whose scenario's
//! exact @step block is found once is repaired by `--fix`; otherwise the
//! problem names how often the block occurs. A coverage entry whose scenario
//! is not in the feature is removed by `--fix`.
//!
//! AUDIT-004: a problem whose repair command holds a `<placeholder>`, or
//! whose exact @step block is not found exactly once, is guided: it names
//! `fspec audit --start --feature <f>`, whose session step carries the
//! repair command.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::fix::Repair;
use super::relink::{relink, Relink};
use super::{Area, Problem, Sidecars};
use crate::error::FspecCoreError;
use crate::types::coverage::CoverageFile;

/// One feature as the coverage area sees it.
pub(super) struct FeatureCoverage<'a> {
    pub(super) project_root: &'a Path,
    pub(super) name: &'a str,
    pub(super) feature_file: &'a str,
    pub(super) content: &'a str,
    pub(super) feature: &'a gherkin::Feature,
    /// The coverage file's text, when there is one.
    pub(super) sidecar: Option<&'a str>,
    /// The coverage file parsed, when it parses.
    pub(super) coverage: Option<&'a CoverageFile>,
}

/// Push the coverage problems of `target` onto `problems`.
pub(super) fn problems(
    target: &FeatureCoverage<'_>,
    sidecars: &Sidecars,
    problems: &mut Vec<Problem>,
) -> Result<(), FspecCoreError> {
    let name = target.name;
    let coverage_file = format!("{}.coverage", target.feature_file);
    let problem = |code: &'static str, fact: String| Problem::new(Area::Coverage, code, name, fact);
    let deprecated =
        crate::types::coverage::deprecated_scenario_names(target.content).unwrap_or_default();
    let live_scenarios = || {
        target
            .feature
            .scenarios
            .iter()
            .map(|scenario| scenario.name.as_str())
            .filter(|scenario| !deprecated.contains(*scenario))
    };

    // BUG-563: a Rule block hides its scenarios from coverage, so the
    // feature's coverage is not checked until the block is gone.
    let rule_blocks = crate::io::gherkin::rule_block_errors(target.content);
    if !rule_blocks.is_empty() {
        for (line, message) in rule_blocks {
            problems.push(
                problem("rule-block", message)
                    .at(&format!("{}:{line}", target.feature_file))
                    .fix(format!("fspec validate {}", target.feature_file)),
            );
        }
        return Ok(());
    }

    let Some(raw) = target.sidecar else {
        // No coverage file yet: every live scenario is uncovered.
        for scenario in live_scenarios() {
            problems.push(
                problem(
                    "uncovered",
                    "the scenario has no test mappings and no coverage file".to_string(),
                )
                .scenario(scenario)
                .fix(format!("fspec generate-coverage --feature-name {name}")),
            );
        }
        return Ok(());
    };

    // BUG-639: a file unreadable only for its stats block (e.g. counts
    // where lists belong) is repaired by the stats fix, not by hand.
    let unreadable_fix = if stats_only_unreadable(raw) {
        format!("fspec audit --fix --feature {name}")
    } else {
        format!("fspec validate {}", target.feature_file)
    };
    // BUG-485/557: the coverage file lists exactly the feature's scenarios,
    // each mapping once.
    let sync = crate::commands::validate::coverage_drift::sync_problems(
        &coverage_file,
        raw,
        target.feature,
    );
    for sync_problem in &sync {
        let fix = match sync_problem.code {
            "duplicate-mapping" => format!(
                "fspec link-coverage {name} --scenario \"{}\" --test-file <file> --test-lines <lines> --replace",
                sync_problem.scenario.as_deref().unwrap_or_default()
            ),
            "unreadable-coverage" => unreadable_fix.clone(),
            _ => format!("fspec generate-coverage --feature-name {name}"),
        };
        let mut found = problem(sync_problem.code, sync_problem.message.clone())
            .at(&coverage_file)
            .fix(fix);
        if sync_problem.code == "duplicate-mapping" {
            found = found.guided();
        }
        if let Some(scenario) = &sync_problem.scenario {
            found = found.scenario(scenario);
            if sync_problem.code == "stale-entry" {
                found = found.mechanical(Repair::StaleEntry {
                    scenario: scenario.clone(),
                });
            }
        }
        problems.push(found);
    }
    // BUG-608: the stats block is the one its scenarios give.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        problems.extend(super::stats::problem(name, &coverage_file, &value));
    }
    let Some(coverage) = target.coverage else {
        if !sync.iter().any(|p| p.code == "unreadable-coverage") {
            problems.push(
                problem(
                    "unreadable-coverage",
                    format!(
                        "{coverage_file} does not match the coverage format: {}",
                        serde_json::from_str::<CoverageFile>(raw)
                            .err()
                            .map(|error| error.to_string())
                            .unwrap_or_default()
                    ),
                )
                .at(&coverage_file)
                .fix(unreadable_fix),
            );
        }
        return Ok(());
    };

    // COV-078: the exact @step block search, once per scenario.
    let mut relinks: BTreeMap<&str, Relink> = BTreeMap::new();
    let mut relinked = |scenario_name: &str, found: Problem| -> Result<Problem, FspecCoreError> {
        let Some(scenario) = coverage
            .scenarios
            .iter()
            .find(|scenario| scenario.name == scenario_name)
        else {
            return Ok(found);
        };
        let decided = match relinks.get(scenario.name.as_str()) {
            Some(decided) => decided.clone(),
            None => {
                let decided = relink(
                    target.project_root,
                    name,
                    target.feature,
                    scenario,
                    sidecars,
                )?;
                relinks.insert(scenario.name.as_str(), decided.clone());
                decided
            }
        };
        Ok(match decided {
            Relink::Once(repair) => found.mechanical(repair),
            // AUDIT-004: which block, if any, to map needs judgment.
            Relink::NotOnce(fact) => {
                let mut found = found.guided();
                found.fact = format!("{}; {fact}", found.fact);
                found
            }
            Relink::Unchanged => found,
        })
    };

    // Missing test and implementation files.
    for scenario in &coverage.scenarios {
        for mapping in &scenario.test_mappings {
            if !sidecars.source.exists(target.project_root, &mapping.file) {
                let found = problem(
                    "missing-test-file",
                    "the test file does not exist".to_string(),
                )
                .scenario(&scenario.name)
                .at(&format!("{}:{}", mapping.file, mapping.lines))
                .fix(format!(
                    "fspec unlink-coverage {name} --scenario \"{}\" --test-file {}",
                    scenario.name, mapping.file
                ));
                problems.push(relinked(&scenario.name, found)?);
            }
            for implementation in &mapping.impl_mappings {
                if !sidecars
                    .source
                    .exists(target.project_root, &implementation.file)
                {
                    problems.push(
                        problem(
                            "missing-impl-file",
                            "the implementation file does not exist".to_string(),
                        )
                        .scenario(&scenario.name)
                        .at(&implementation.file)
                        .fix(format!(
                            "fspec unlink-coverage {name} --scenario \"{}\" --test-file {} --impl-file {}",
                            scenario.name, mapping.file, implementation.file
                        )),
                    );
                }
            }
        }
    }

    // BUG-375/501/509/525: each scenario's mapped test lines hold an exact
    // @step comment for every step, inside the file.
    let source = &sidecars.source;
    let drifts =
        super::drift::drifted_mappings(target.project_root, source, target.content, coverage)?;
    for drift in drifts
        .into_iter()
        .filter(|drift| drift.code != "stale-entry")
    {
        let found = problem(drift.code, drift.fact)
            .scenario(&drift.scenario)
            .at(&drift.mappings)
            .fix(format!(
                "fspec link-coverage {name} --scenario \"{}\" --test-file {} --test-lines <lines> --replace",
                drift.scenario, drift.file
            ))
            .guided();
        problems.push(relinked(&drift.scenario, found)?);
    }

    // BUG-515: implementation lines lie inside their file.
    for found in super::impl_ranges::impl_range_problems(
        target.project_root,
        source,
        &coverage_file,
        coverage,
        &sidecars.history,
    )? {
        let mut reported = problem(found.code, found.fact)
            .scenario(&found.scenario)
            .at(&found.location)
            .fix(format!(
                "fspec link-coverage {name} --scenario \"{}\" --test-file {} --impl-file {} --impl-lines <lines>",
                found.scenario, found.test_file, found.impl_file
            ))
            .guided();
        // COV-079: a mapping whose code only moved, or whose text can be
        // recorded exactly, is repaired by --fix.
        if let Some(change) = found.repair {
            reported = reported.mechanical(super::fix::Repair::Impl {
                scenario: found.scenario.clone(),
                test_file: found.test_file.clone(),
                test_lines: found.test_lines.clone(),
                impl_file: found.impl_file.clone(),
                lines: found.lines.clone(),
                change,
            });
        }
        problems.push(reported);
    }

    // BUG-357: one test file maps to one feature.
    let test_files: BTreeSet<&str> = coverage
        .scenarios
        .iter()
        .flat_map(|scenario| &scenario.test_mappings)
        .map(|mapping| mapping.file.as_str())
        .collect();
    for file in test_files {
        let others = sidecars.other_features(file, name);
        if others.is_empty() {
            continue;
        }
        let others: Vec<String> = others
            .iter()
            .map(|other| format!("spec/features/{other}.feature"))
            .collect();
        problems.push(
            problem(
                "shared-test-file",
                format!("also mapped by {}", others.join(", ")),
            )
            .at(file)
            .fix(format!(
                "fspec unlink-coverage {name} --scenario \"<scenario>\" --test-file {file}"
            ))
            .guided(),
        );
    }

    // Uncovered live scenarios (a scenario missing from the coverage file is
    // reported by the sync check above).
    for scenario in live_scenarios() {
        let uncovered = coverage
            .scenarios
            .iter()
            .find(|entry| entry.name == scenario)
            .is_some_and(|entry| entry.test_mappings.is_empty());
        if uncovered {
            problems.push(
                problem("uncovered", "the scenario has no test mappings".to_string())
                    .scenario(scenario)
                    .fix(format!(
                        "fspec link-coverage {name} --scenario \"{scenario}\" --test-file <file> --test-lines <lines>"
                    ))
                    .guided(),
            );
        }
    }
    Ok(())
}

/// BUG-639: `raw` does not match the coverage format, but would once its
/// stats block were the one its scenarios give.
fn stats_only_unreadable(raw: &str) -> bool {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    if value.get("stats").is_none() || serde_json::from_value::<CoverageFile>(value.clone()).is_ok()
    {
        return false;
    }
    let stats = crate::commands::link_coverage::stats::computed_stats(&value);
    value["stats"] = serde_json::Value::Object(stats);
    serde_json::from_value::<CoverageFile>(value).is_ok()
}
