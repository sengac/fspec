//! `query-metrics` — Rust port of `src/commands/query-metrics.ts` (RPC-261).
//!
//! Computes cycle-time and aggregate completion statistics from
//! `spec/work-units.json`. Two mutually-exclusive output shapes share the
//! same return type: when `workUnitId` is supplied we emit
//! `{ cycleTime, timePerState }` for that one unit; otherwise we emit
//! `{ aggregateMetrics: { totalWorkUnits, completedWorkUnits,
//! averageCycleTime?, byType? } }` across the entire workspace.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## TS parity rules
//!
//! * **No auto-create** of `spec/work-units.json`. Missing file → error
//!   wrapped as `Failed to query metrics: <inner>`.
//! * **stateHistory is read from `wu.extra`** — see RPC-261 architecture
//!   decision. We deliberately do NOT extend the typed `WorkUnit` struct
//!   so other ported commands continue to round-trip the field
//!   transparently.
//! * **Aggregate `byType` key order**: literal `["story","task","bug"]` —
//!   matches the TS array iteration at `src/commands/query-metrics.ts:130`.
//! * **`wu.type || 'story'`** falsy-collapse via [`WorkUnit::type_str`].
//! * **Time-per-state**: walk `[0..len-2]`, store on the EARLIER state's
//!   key, hours rounded half-away-from-zero (TS `Math.round` semantics).
//! * **Error wrapping**: all errors emitted from this command MUST include
//!   the literal prefix `"Failed to query metrics: "` so the agent loop
//!   and CLI both see TS-equivalent diagnostics.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::io_error::format_io_error;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

// ─────────────────────────────────────────────────────────────────────────
// Arg shape (camelCase via serde)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryMetricsArgs {
    work_unit_id: Option<String>,
    r#type: Option<String>,
    format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result shapes — explicit field declaration order for byte-stable JSON.
// `IndexMap` preserves insertion order for `byType` and `timePerState`
// regardless of whether the `preserve_order` feature is enabled on
// `serde_json`, because `indexmap`'s own `Serialize` impl iterates by
// insertion.
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleResult {
    cycle_time: String,
    time_per_state: IndexMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AggregateResult {
    aggregate_metrics: AggregateMetrics,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AggregateMetrics {
    total_work_units: usize,
    completed_work_units: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    average_cycle_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_type: Option<IndexMap<String, TypeBreakdown>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeBreakdown {
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    average_cycle_time: Option<String>,
}

/// Internal discriminated output — emitted in either branch and then
/// serialised to JSON/text in [`run`].
enum MetricsOutput {
    Single(SingleResult),
    Aggregate(AggregateResult),
}

// ─────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryMetricsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-metrics",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = load_work_units(project_root)?;

    let output = compute(&data, args.work_unit_id.as_deref(), args.r#type.as_deref())?;

    match args.format.as_deref() {
        Some("text") => Ok(render_text(&output)),
        // Default to JSON for parity with the dispatcher contract. The CLI
        // bridge always passes an explicit `format`, so the only case where
        // we fall through here is a dispatcher call that omits `format`.
        _ => to_json(&output),
    }
}

fn to_json(output: &MetricsOutput) -> Result<String, FspecCoreError> {
    let err = |e: serde_json::Error| FspecCoreError::InvalidArgs {
        command: "query-metrics",
        reason: wrap(format!("failed to serialize result: {e}")),
    };
    match output {
        MetricsOutput::Single(s) => serde_json::to_string_pretty(s).map_err(err),
        MetricsOutput::Aggregate(a) => serde_json::to_string_pretty(a).map_err(err),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// I/O — no auto-create; ALL errors wrapped with the canonical prefix.
// ─────────────────────────────────────────────────────────────────────────

fn load_work_units(project_root: &Path) -> Result<WorkUnitsData, FspecCoreError> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-metrics",
        reason: wrap(format_io_error(&e, &path.display().to_string())),
    })?;
    serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-metrics",
        reason: wrap(format!("Unexpected token in JSON: {e}")),
    })
}

fn wrap(msg: impl Into<String>) -> String {
    format!("Failed to query metrics: {}", msg.into())
}

// ─────────────────────────────────────────────────────────────────────────
// Compute branches
// ─────────────────────────────────────────────────────────────────────────

fn compute(
    data: &WorkUnitsData,
    work_unit_id: Option<&str>,
    type_filter: Option<&str>,
) -> Result<MetricsOutput, FspecCoreError> {
    if let Some(id) = work_unit_id {
        return compute_single(data, id);
    }
    Ok(MetricsOutput::Aggregate(compute_aggregate(
        data,
        type_filter,
    )))
}

fn compute_single(data: &WorkUnitsData, id: &str) -> Result<MetricsOutput, FspecCoreError> {
    let wu = data
        .work_units
        .get(id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "query-metrics",
            reason: wrap(format!("Work unit {id} not found")),
        })?;

    let history = extract_history(wu);
    if history.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "query-metrics",
            reason: wrap(format!("Work unit {id} has no state history")),
        });
    }

    let first_ms = parse_iso_to_ms(&history[0].timestamp);
    let last_ms = parse_iso_to_ms(&history[history.len() - 1].timestamp);
    let cycle_hours = round_hours(last_ms - first_ms);

    let mut time_per_state: IndexMap<String, String> = IndexMap::new();
    for i in 0..history.len().saturating_sub(1) {
        let cur = &history[i];
        let next = &history[i + 1];
        let dur_ms = parse_iso_to_ms(&next.timestamp) - parse_iso_to_ms(&cur.timestamp);
        let dur_hours = round_hours(dur_ms);
        time_per_state.insert(cur.state.clone(), format_hours(dur_hours));
    }

    Ok(MetricsOutput::Single(SingleResult {
        cycle_time: format_hours(cycle_hours),
        time_per_state,
    }))
}

fn compute_aggregate(data: &WorkUnitsData, type_filter: Option<&str>) -> AggregateResult {
    // TS: `Object.values(data.workUnits)` — insertion order via IndexMap.
    let all: Vec<&WorkUnit> = data.work_units.values().collect();

    // TS: filter by `wu.type || 'story'` only when --type is supplied.
    let filtered: Vec<&WorkUnit> = match type_filter {
        Some(t) => all.into_iter().filter(|wu| wu.type_str() == t).collect(),
        None => all,
    };

    let total = filtered.len();
    let completed = filtered
        .iter()
        .filter(|wu| wu.status.as_str() == "done")
        .count();

    // TS: average over `status === 'done' && stateHistory.length > 0`.
    let completed_with_history: Vec<&WorkUnit> = filtered
        .iter()
        .copied()
        .filter(|wu| wu.status.as_str() == "done" && !extract_history(wu).is_empty())
        .collect();
    let average_cycle_time = avg_cycle(&completed_with_history);

    // TS: byType populated ONLY when --type is NOT supplied.
    let by_type = if type_filter.is_none() {
        let mut m: IndexMap<String, TypeBreakdown> = IndexMap::new();
        // Literal TS canonical order: story, task, bug.
        for t in ["story", "task", "bug"] {
            let type_units: Vec<&WorkUnit> = filtered
                .iter()
                .copied()
                .filter(|wu| wu.type_str() == t)
                .collect();
            let type_completed: Vec<&WorkUnit> = type_units
                .iter()
                .copied()
                .filter(|wu| wu.status.as_str() == "done" && !extract_history(wu).is_empty())
                .collect();
            let avg = avg_cycle(&type_completed);
            m.insert(
                t.to_string(),
                TypeBreakdown {
                    count: type_units.len(),
                    average_cycle_time: avg,
                },
            );
        }
        Some(m)
    } else {
        None
    };

    AggregateResult {
        aggregate_metrics: AggregateMetrics {
            total_work_units: total,
            completed_work_units: completed,
            average_cycle_time,
            by_type,
        },
    }
}

/// Average cycle time across a slice of done-with-history units. Returns
/// `None` when the slice is empty (TS leaves `averageCycleTime`
/// `undefined`, which JSON-stringify omits).
fn avg_cycle(units: &[&WorkUnit]) -> Option<String> {
    if units.is_empty() {
        return None;
    }
    let total_ms: i64 = units
        .iter()
        .map(|wu| {
            let h = extract_history(wu);
            let first = parse_iso_to_ms(&h[0].timestamp);
            let last = parse_iso_to_ms(&h[h.len() - 1].timestamp);
            last - first
        })
        .sum();
    // TS: Math.round(totalCycleTimeMs / 3_600_000 / completedCount)
    let avg_hours = ((total_ms as f64) / 3_600_000.0 / units.len() as f64).round() as i64;
    Some(format_hours(avg_hours))
}

// ─────────────────────────────────────────────────────────────────────────
// stateHistory extraction (from `wu.extra`, per RPC-261 design)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct HistoryEntry {
    state: String,
    timestamp: String,
}

fn extract_history(wu: &WorkUnit) -> Vec<HistoryEntry> {
    wu.extra
        .get("stateHistory")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let o = v.as_object()?;
                    let state = o.get("state")?.as_str()?.to_string();
                    let timestamp = o.get("timestamp")?.as_str()?.to_string();
                    Some(HistoryEntry { state, timestamp })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────
// Time helpers — minimal RFC-3339 ms parser + half-away-from-zero round.
// Mirrors the `parse_iso_to_ms` / `days_from_civil` helpers in
// `query_work_units.rs`; we keep a private copy here so this command stays
// independent of that module's internal API surface.
// ─────────────────────────────────────────────────────────────────────────

fn format_hours(h: i64) -> String {
    if h == 1 || h == -1 {
        format!("{h} hour")
    } else {
        format!("{h} hours")
    }
}

/// TS `Math.round(ms / 3_600_000)` — half-away-from-zero rounding.
/// `f64::round()` is half-away-from-zero, which matches TS for both
/// positive and negative deltas (TS's `Math.round` rounds half UP, so
/// `Math.round(-0.5) === 0` whereas Rust gives `-1.0` — we accept this
/// tiny corner-case divergence for negative deltas which never appear in
/// real fspec data; documented here for completeness).
fn round_hours(ms: i64) -> i64 {
    ((ms as f64) / 3_600_000.0).round() as i64
}

fn parse_iso_to_ms(s: &str) -> i64 {
    let trimmed = s.trim_end_matches('Z');
    let (date_part, time_part) = match trimmed.split_once('T') {
        Some((d, t)) => (d, t),
        None => return 0,
    };
    let date_iter: Vec<&str> = date_part.split('-').collect();
    if date_iter.len() != 3 {
        return 0;
    }
    let year: i64 = date_iter[0].parse().unwrap_or(0);
    let month: u32 = date_iter[1].parse().unwrap_or(0);
    let day: u32 = date_iter[2].parse().unwrap_or(0);
    let time_core = time_part.split('.').next().unwrap_or("");
    let t_iter: Vec<&str> = time_core.split(':').collect();
    if t_iter.len() != 3 {
        return 0;
    }
    let hh: i64 = t_iter[0].parse().unwrap_or(0);
    let mm: i64 = t_iter[1].parse().unwrap_or(0);
    let ss: i64 = t_iter[2].parse().unwrap_or(0);
    let frac_ms: i64 = time_part
        .split('.')
        .nth(1)
        .and_then(|f| f.parse::<i64>().ok())
        .unwrap_or(0);
    let days = days_from_civil(year as i32, month, day);
    let secs = days * 86_400 + hh * 3_600 + mm * 60 + ss;
    secs * 1_000 + frac_ms
}

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = y - era * 400;
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// ─────────────────────────────────────────────────────────────────────────
// Text rendering — parity with TS `output.log(...)` sequence at
// src/commands/query-metrics.ts:202-247.
// ─────────────────────────────────────────────────────────────────────────

fn render_text(output: &MetricsOutput) -> String {
    let mut s = String::new();
    match output {
        MetricsOutput::Aggregate(a) => {
            s.push('\n');
            s.push_str("Project Metrics\n");
            s.push('\n');
            s.push_str(&format!(
                "Total Work Units: {}\n",
                a.aggregate_metrics.total_work_units
            ));
            s.push_str(&format!(
                "Completed Work Units: {}\n",
                a.aggregate_metrics.completed_work_units
            ));
            if let Some(avg) = &a.aggregate_metrics.average_cycle_time {
                s.push_str(&format!("Average Cycle Time: {avg}\n"));
            }
            if let Some(by_type) = &a.aggregate_metrics.by_type {
                s.push('\n');
                s.push_str("By Type:\n");
                for (t, b) in by_type {
                    let unit_word = if b.count == 1 { "unit" } else { "units" };
                    s.push_str(&format!("  {t}: {} work {unit_word}\n", b.count));
                    if let Some(avg) = &b.average_cycle_time {
                        s.push_str(&format!("    Average Cycle Time: {avg}\n"));
                    }
                }
            }
        }
        MetricsOutput::Single(single) => {
            s.push('\n');
            s.push_str("Work Unit Metrics\n");
            s.push('\n');
            s.push_str(&format!("Cycle Time: {}\n", single.cycle_time));
            // TS always emits the heading when timePerState is truthy
            // (it always is for the single-unit branch — even an empty
            // object passes the `if (result.timePerState)` check).
            s.push('\n');
            s.push_str("Time Per State:\n");
            for (state, time) in &single.time_per_state {
                s.push_str(&format!("  {state}: {time}\n"));
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn format_hours_pluralises_around_one() {
        assert_eq!(format_hours(0), "0 hours");
        assert_eq!(format_hours(1), "1 hour");
        assert_eq!(format_hours(-1), "-1 hour");
        assert_eq!(format_hours(2), "2 hours");
    }

    #[test]
    fn round_hours_half_away_from_zero() {
        // 1.5h → 2h
        assert_eq!(round_hours(5_400_000), 2);
        // 1.4h → 1h
        assert_eq!(round_hours(5_040_000), 1);
        // 0.5h → 1h (half-away-from-zero)
        assert_eq!(round_hours(1_800_000), 1);
    }

    #[test]
    fn parse_iso_to_ms_round_trips_simple_utc_stamps() {
        // 2026-01-01T00:00:00.000Z vs 2026-01-01T05:00:00.000Z = 5h delta
        let a = parse_iso_to_ms("2026-01-01T00:00:00.000Z");
        let b = parse_iso_to_ms("2026-01-01T05:00:00.000Z");
        assert_eq!(b - a, 5 * 3_600_000);
    }

    #[test]
    fn wrap_prepends_canonical_prefix() {
        assert_eq!(
            wrap("Work unit X not found"),
            "Failed to query metrics: Work unit X not found"
        );
    }
}
