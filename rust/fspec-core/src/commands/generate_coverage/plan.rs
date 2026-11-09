//! Lossless selected-generation planning against raw sidecar JSON.
//!
//! Kept scenarios are carried as untouched `Value`s (unknown members, ordered
//! duplicate rows and implementation arrays survive); unknown top-level and
//! stats members are retained. Mapped removals, duplicate names and any
//! invalid or unreadable metadata block the whole plan.

use std::collections::BTreeSet;
use std::io::ErrorKind;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlanStatus {
    Create,
    Update,
    Skip,
    Blocked,
}

impl PlanStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            PlanStatus::Create => "create",
            PlanStatus::Update => "update",
            PlanStatus::Skip => "skip",
            PlanStatus::Blocked => "blocked",
        }
    }
}

pub(super) struct Plan {
    pub status: PlanStatus,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub blocked_reasons: Vec<String>,
    /// Complete sidecar to write for `Create` / `Update`.
    pub next: Option<Value>,
}

impl Plan {
    fn blocked(reason: String) -> Self {
        Self {
            status: PlanStatus::Blocked,
            added: Vec::new(),
            removed: Vec::new(),
            blocked_reasons: vec![reason],
            next: None,
        }
    }
}

pub(super) fn plan(feature_path: &Path, coverage_path: &Path) -> Result<Plan, FspecCoreError> {
    // BUG-563: a Rule block would hide its scenarios.
    crate::io::gherkin::reject_rule_blocks(feature_path, "generate-coverage")?;
    let names = match feature_scenarios(feature_path) {
        Ok(names) => names,
        Err(reason) => return Ok(Plan::blocked(reason)),
    };
    let raw = match std::fs::metadata(coverage_path) {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            let scenarios: Vec<Value> = names.iter().map(|name| new_scenario(name)).collect();
            let mut next = json!({ "scenarios": scenarios, "stats": {} });
            set_counts(&mut next);
            return Ok(Plan {
                status: PlanStatus::Create,
                added: names,
                removed: Vec::new(),
                blocked_reasons: Vec::new(),
                next: Some(next),
            });
        }
        Err(error) => return Ok(Plan::blocked(format!("sidecar is unreadable: {error}"))),
        Ok(metadata) if !metadata.is_file() => {
            return Ok(Plan::blocked("sidecar is not a regular file".to_string()))
        }
        Ok(_) => match std::fs::read_to_string(coverage_path) {
            Ok(raw) => raw,
            Err(error) => return Ok(Plan::blocked(format!("sidecar is unreadable: {error}"))),
        },
    };
    let existing: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => return Ok(Plan::blocked(format!("sidecar is invalid JSON: {error}"))),
    };
    let existing_names = match sidecar_names(&existing) {
        Ok(names) => names,
        Err(reason) => return Ok(Plan::blocked(reason)),
    };
    let wanted: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let present: BTreeSet<&str> = existing_names
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let added: Vec<String> = names
        .iter()
        .filter(|name| !present.contains(name.as_str()))
        .cloned()
        .collect();
    let removed: Vec<String> = existing_names
        .iter()
        .filter(|(name, _)| !wanted.contains(name.as_str()))
        .map(|(name, _)| name.clone())
        .collect();
    let blocked_reasons: Vec<String> = existing_names
        .iter()
        .filter(|(name, mapped)| *mapped && !wanted.contains(name.as_str()))
        .map(|(name, _)| {
            format!("scenario \"{name}\" has mapped coverage lineage and is no longer declared")
        })
        .collect();
    let (status, next) = if !blocked_reasons.is_empty() {
        (PlanStatus::Blocked, None)
    } else if added.is_empty() && removed.is_empty() {
        (PlanStatus::Skip, None)
    } else {
        let mut next = existing;
        if let Some(scenarios) = next.get_mut("scenarios").and_then(Value::as_array_mut) {
            scenarios.retain(|scenario| {
                scenario
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| wanted.contains(name))
            });
            scenarios.extend(added.iter().map(|name| new_scenario(name)));
        }
        set_counts(&mut next);
        (PlanStatus::Update, Some(next))
    };
    Ok(Plan {
        status,
        added,
        removed,
        blocked_reasons,
        next,
    })
}

fn new_scenario(name: &str) -> Value {
    json!({ "name": name, "testMappings": [] })
}

/// Declared scenario names in feature order; outlines count once. Duplicate
/// names make selection ambiguous. (Rule blocks are rejected before this,
/// BUG-563.)
fn feature_scenarios(feature_path: &Path) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(feature_path)
        .map_err(|e| format!("feature file is unreadable: {e}"))?;
    let feature = parse_feature_lenient(&content)
        .map_err(|e| format!("feature file cannot be parsed: {e}"))?;
    let mut positioned: Vec<(usize, String)> = feature
        .scenarios
        .iter()
        .chain(feature.rules.iter().flat_map(|rule| rule.scenarios.iter()))
        .map(|scenario| (scenario.position.line, scenario.name.clone()))
        .collect();
    positioned.sort_by_key(|(line, _)| *line);
    let names: Vec<String> = positioned.into_iter().map(|(_, name)| name).collect();
    let mut seen = BTreeSet::new();
    if let Some(duplicate) = names.iter().find(|name| !seen.insert(name.as_str())) {
        return Err(format!(
            "feature declares duplicate scenario name \"{duplicate}\""
        ));
    }
    Ok(names)
}

/// `(name, has mapped lineage)` per sidecar scenario after strict shape
/// validation; duplicate names are ambiguous and block.
fn sidecar_names(existing: &Value) -> Result<Vec<(String, bool)>, String> {
    let scenarios = existing
        .as_object()
        .ok_or("sidecar is not a JSON object")?
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or("sidecar has no scenarios array")?;
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let name = scenario
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("sidecar scenario {index} has no string name"))?;
        let mappings = scenario
            .get("testMappings")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("sidecar scenario \"{name}\" has no testMappings array"))?;
        if !seen.insert(name.to_string()) {
            return Err(format!(
                "sidecar contains ambiguous duplicate scenario name \"{name}\""
            ));
        }
        names.push((name.to_string(), !mappings.is_empty()));
    }
    Ok(names)
}

/// BUG-608: recompute the stats with the one stats definition, retaining
/// every other stats member.
fn set_counts(coverage: &mut Value) {
    crate::commands::link_coverage::stats::update_stats(coverage);
}
