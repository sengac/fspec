//! BUG-608: the coverage area's stats check. A coverage file's stats block
//! is a deterministic function of its scenarios array (link-coverage's
//! [`computed_stats`], the one stats definition every writer uses), so a
//! stored value that differs is reported and `--fix` rewrites the stats
//! block. A file without a stats block is not reported.
//!
//! Feature: spec/features/coverage-stats-match-their-scenarios.feature

use serde_json::Value;

use super::fix::Repair;
use super::{Area, Problem};
use crate::commands::link_coverage::stats::computed_stats;

/// The stats problem of the coverage file `coverage` (at `coverage_file`)
/// of feature `name`, or `None` when its stored stats are what its
/// scenarios give.
pub(super) fn problem(name: &str, coverage_file: &str, coverage: &Value) -> Option<Problem> {
    // A file without a stats block stores nothing to be wrong; show-coverage
    // computes its stats when it shows it.
    let stored = coverage.get("stats")?;
    let empty = serde_json::Map::new();
    let stored = stored.as_object().unwrap_or(&empty);
    let differences: Vec<String> = computed_stats(coverage)
        .iter()
        .filter(|(field, value)| stored.get(*field) != Some(*value))
        .map(|(field, value)| {
            let was = stored
                .get(field)
                .map_or_else(|| "absent".to_string(), Value::to_string);
            format!("{field} {was} -> {value}")
        })
        .collect();
    if differences.is_empty() {
        return None;
    }
    let fact = format!(
        "the stats do not match the scenarios: {}",
        differences.join(", ")
    );
    Some(
        Problem::new(Area::Coverage, "stats-mismatch", name, fact)
            .at(coverage_file)
            .mechanical(Repair::Stats),
    )
}
