//! CLI-019: read the feature files and select the criteria of `fspec context`.
//!
//! `--tags` is a cucumber tag expression parsed by the gherkin crate's
//! [`TagOperation`] and matched against a scenario's own tags plus those of
//! its Rule and Feature. `--text` (case-insensitive substring) and `--regex`
//! match the scenario name, step text, doc strings and table cells, each on
//! its own. Criteria keep feature-path then line order.

use std::path::Path;

use gherkin::tagexpr::TagOperation;
use gherkin::{Rule, Scenario, Step, Table};
use regex::Regex;

use super::invalid;
use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;
use crate::io::gherkin_format::{feature_header_lines, scenario_lines};

/// The exact filters of one `fspec context` call.
pub(super) struct Filters {
    tags: Option<TagOperation>,
    text: Option<String>,
    regex: Option<Regex>,
}

impl Filters {
    pub(super) fn new(
        tags: Option<&str>,
        text: Option<&str>,
        regex: Option<&str>,
    ) -> Result<Self, FspecCoreError> {
        let tags = tags
            .map(|expr| {
                expr.parse::<TagOperation>()
                    .map_err(|e| invalid(format!("invalid tag expression '{expr}': {e}")))
            })
            .transpose()?;
        let regex = regex
            .map(|pattern| {
                Regex::new(pattern).map_err(|e| invalid(format!("invalid regex '{pattern}': {e}")))
            })
            .transpose()?;
        Ok(Self {
            tags,
            text: text.map(str::to_lowercase),
            regex,
        })
    }

    /// Whether a scenario with the matching tags `tags` (no leading `@`)
    /// passes every given filter.
    fn keeps(&self, scenario: &Scenario, tags: &[String]) -> bool {
        if let Some(op) = &self.tags {
            if !eval(op, tags) {
                return false;
            }
        }
        let texts = texts(scenario);
        if let Some(text) = &self.text {
            if !texts.iter().any(|t| t.to_lowercase().contains(text)) {
                return false;
            }
        }
        if let Some(regex) = &self.regex {
            if !texts.iter().any(|t| regex.is_match(t)) {
                return false;
            }
        }
        true
    }
}

/// Evaluate a parsed tag expression over a scenario's matching tags.
fn eval(op: &TagOperation, tags: &[String]) -> bool {
    match op {
        TagOperation::And(a, b) => eval(a, tags) && eval(b, tags),
        TagOperation::Or(a, b) => eval(a, tags) || eval(b, tags),
        TagOperation::Not(a) => !eval(a, tags),
        TagOperation::Tag(tag) => tags.iter().any(|t| t == tag),
    }
}

/// The texts `--text` and `--regex` look at: the scenario name, then each
/// step's text, doc string and table cells, then the Examples cells.
fn texts(scenario: &Scenario) -> Vec<&str> {
    fn cells(table: &Table) -> impl Iterator<Item = &str> {
        table.rows.iter().flatten().map(String::as_str)
    }
    fn step_texts(step: &Step) -> impl Iterator<Item = &str> {
        std::iter::once(step.value.as_str())
            .chain(step.docstring.as_deref())
            .chain(step.table.iter().flat_map(cells))
    }
    std::iter::once(scenario.name.as_str())
        .chain(scenario.steps.iter().flat_map(step_texts))
        .chain(
            scenario
                .examples
                .iter()
                .filter_map(|e| e.table.as_ref())
                .flat_map(cells),
        )
        .collect()
}

/// One selected feature: its file, name, tags (with `@`) and header lines.
pub(super) struct Feature {
    pub file: String,
    pub name: String,
    pub tags: Vec<String>,
    pub header: Vec<String>,
}

/// One acceptance criterion.
pub(super) struct Criterion {
    /// Index into [`Selection::features`].
    pub feature: usize,
    pub line: usize,
    pub rule: Option<String>,
    pub name: String,
    /// The scenario's own tags, with `@`.
    pub tags: Vec<String>,
    /// The Feature's then the Rule's tags, with `@`.
    pub inherited: Vec<String>,
    pub lines: Vec<String>,
}

/// The features that have selected criteria, and the criteria in order.
pub(super) struct Selection {
    pub features: Vec<Feature>,
    pub criteria: Vec<Criterion>,
}

fn at(tags: &[String]) -> Vec<String> {
    tags.iter().map(|t| format!("@{t}")).collect()
}

/// Whether the feature file `rel` is the one `--feature name` names: its
/// path or file name when `name` ends in `.feature`, else its file stem.
fn is_named(rel: &str, name: &str) -> bool {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    if name.ends_with(".feature") {
        rel == name || file == name
    } else {
        file.strip_suffix(".feature") == Some(name)
    }
}

/// Read `spec/features/**/*.feature` (only the `--feature` one when given)
/// and select the criteria every filter keeps.
pub(super) fn load(
    project_root: &Path,
    feature: Option<&str>,
    filters: &Filters,
) -> Result<Selection, FspecCoreError> {
    if !crate::io::spec_paths::features_dir_of(project_root).is_dir() {
        return Err(FspecCoreError::Message(
            "spec/features directory not found".to_string(),
        ));
    }
    let mut files = glob_feature_files(project_root)?;
    if let Some(name) = feature {
        files.retain(|rel| is_named(rel, name));
        if files.is_empty() {
            return Err(invalid(format!(
                "feature '{name}' does not exist in spec/features/"
            )));
        }
    }
    let mut selection = Selection {
        features: Vec::new(),
        criteria: Vec::new(),
    };
    for rel in files {
        let source = std::fs::read_to_string(project_root.join(&rel)).map_err(|source| {
            FspecCoreError::Io {
                command: "context",
                source,
            }
        })?;
        let parsed = parse_feature_lenient(&source)
            .map_err(|e| FspecCoreError::Message(format!("{rel} does not parse: {e}")))?;
        let index = selection.features.len();
        let before = selection.criteria.len();
        let mut found = scenarios(&parsed)
            .into_iter()
            .filter_map(|(scenario, rule, next)| {
                let mut tags = parsed.tags.clone();
                tags.extend(rule.iter().flat_map(|r| r.tags.iter().cloned()));
                let inherited = at(&tags);
                tags.extend(scenario.tags.iter().cloned());
                filters.keeps(scenario, &tags).then(|| Criterion {
                    feature: index,
                    line: scenario.position.line,
                    rule: rule.map(|r| r.name.clone()),
                    name: scenario.name.clone(),
                    tags: at(&scenario.tags),
                    inherited,
                    lines: scenario_lines(scenario, rule, &source, next),
                })
            })
            .collect::<Vec<_>>();
        found.sort_by_key(|c| c.line);
        selection.criteria.extend(found);
        if selection.criteria.len() > before {
            selection.features.push(Feature {
                file: rel.clone(),
                name: parsed.name.clone(),
                tags: at(&parsed.tags),
                header: feature_header_lines(&parsed, &source),
            });
        }
    }
    Ok(selection)
}

/// Every scenario of a feature with its Rule and the line of the next
/// sibling construct (which bounds a step-less scenario's description).
fn scenarios(feature: &gherkin::Feature) -> Vec<(&Scenario, Option<&Rule>, Option<usize>)> {
    let first_rule = feature.rules.first().map(|r| r.position.line);
    let mut out = Vec::new();
    for (i, scenario) in feature.scenarios.iter().enumerate() {
        let next = feature
            .scenarios
            .get(i + 1)
            .map(|s| s.position.line)
            .or(first_rule);
        out.push((scenario, None, next));
    }
    for (r, rule) in feature.rules.iter().enumerate() {
        let next_rule = feature.rules.get(r + 1).map(|r| r.position.line);
        for (i, scenario) in rule.scenarios.iter().enumerate() {
            let next = rule
                .scenarios
                .get(i + 1)
                .map(|s| s.position.line)
                .or(next_rule);
            out.push((scenario, Some(rule), next));
        }
    }
    out
}
