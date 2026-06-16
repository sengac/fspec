//! `generate-summary-report` — Rust port of
//! `src/commands/generate-summary-report.ts` (RPC-235).
//!
//! Reads `spec/work-units.json` (NOT auto-creating it: a missing or malformed
//! file is a hard error, matching the TS `readFile` + `try/catch` that wraps
//! the failure as `Failed to generate summary report: <message>`), tallies
//! work units by status, sums story points, computes velocity (completed
//! work), then writes a markdown or JSON report to disk.
//!
//! Output path resolution mirrors TS:
//!   - `--output <file>` (relative to project_root) when supplied;
//!   - otherwise `spec/summary-report.md` (markdown) or
//!     `spec/summary-report.json` (json).
//!
//! On success the function returns the message string
//! `✓ Report generated: <outputFile>` (where `<outputFile>` is the
//! *unresolved* path, exactly as the TS `result.outputFile`).
//!
//! Two-front-doors invariant: the CLI bridge and the LLM dispatcher both call
//! this `run(args_json, project_root)` function.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;

/// CLI / dispatcher arguments accepted by `generate-summary-report`. Field
/// names mirror the TS Commander options: `--format` and `--output`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GenerateSummaryReportArgs {
    format: Option<String>,
    output: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateSummaryReportArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "generate-summary-report",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Read spec/work-units.json verbatim (TS: readFile + JSON.parse, wrapped
    // in a try/catch that re-throws as "Failed to generate summary report").
    let work_units_path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&work_units_path).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "generate-summary-report",
            reason: format!("Failed to generate summary report: {e}"),
        }
    })?;
    let data: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
        command: "generate-summary-report",
        reason: format!("Failed to generate summary report: {e}"),
    })?;

    // Collect work units (Object.values(data.workUnits)). A missing or
    // non-object workUnits is treated as empty (TS would throw on
    // Object.values(undefined), but for parity we keep robust empty handling
    // since the dispatcher contract returns a hard error only on read/parse).
    let work_units: Vec<&Value> = match data.get("workUnits") {
        Some(Value::Object(map)) => map.values().collect(),
        _ => Vec::new(),
    };

    // Count by status (preserving insertion order via a Map). TS uses an
    // object literal keyed by status; first-seen order is preserved.
    let mut by_status: Map<String, Value> = Map::new();
    for wu in &work_units {
        let status = wu
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let entry = by_status
            .entry(status)
            .or_insert_with(|| Value::from(0u64));
        let next = entry.as_u64().unwrap_or(0) + 1;
        *entry = Value::from(next);
    }

    // Sum story points across all work units. TS sums with `+` over
    // `wu.estimate || 0`, so a fractional estimate (e.g. 2.5) contributes its
    // real value; we sum in f64 and re-narrow to an integer Value when whole
    // (RPC-235 parity fix — the earlier port dropped non-integer estimates).
    let total_story_points: f64 = work_units.iter().map(|wu| estimate_of(wu)).sum();

    // Velocity: completed work (status === "done").
    let completed: Vec<&&Value> = work_units
        .iter()
        .filter(|wu| wu.get("status").and_then(Value::as_str) == Some("done"))
        .collect();
    let completed_points: f64 = completed.iter().map(|wu| estimate_of(wu)).sum();
    let completed_work_units = completed.len() as u64;

    // Assemble the report object in the canonical TS field order:
    // totalWorkUnits, byStatus, totalStoryPoints, velocity.
    let mut velocity = Map::new();
    velocity.insert(
        "completedPoints".to_string(),
        number_value(completed_points),
    );
    velocity.insert(
        "completedWorkUnits".to_string(),
        Value::from(completed_work_units),
    );

    let mut report = Map::new();
    report.insert(
        "totalWorkUnits".to_string(),
        Value::from(work_units.len() as u64),
    );
    report.insert("byStatus".to_string(), Value::Object(by_status.clone()));
    report.insert(
        "totalStoryPoints".to_string(),
        number_value(total_story_points),
    );
    report.insert("velocity".to_string(), Value::Object(velocity));

    // Determine output path (TS: format default "markdown").
    let format = args.format.as_deref().unwrap_or("markdown");
    let extension = if format == "json" { "json" } else { "md" };
    let default_output = format!("spec/summary-report.{extension}");
    let output_path = args.output.unwrap_or(default_output);

    // Generate content.
    let content = if format == "json" {
        serde_json::to_string_pretty(&Value::Object(report.clone()))
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        generate_markdown_report(&report)
    };

    // Write to the resolved (relative-to-project_root) path.
    let full_output = resolve_output(project_root, &output_path);
    std::fs::write(&full_output, content).map_err(|source| FspecCoreError::Io {
        command: "generate-summary-report",
        source,
    })?;

    Ok(format!("✓ Report generated: {output_path}"))
}

/// Read a work unit's `estimate` field as a number, defaulting to 0 (mirrors
/// TS `wu.estimate || 0`). Non-numeric estimates (the JS `number + string`
/// coercion quirk) collapse to 0 rather than being string-concatenated.
fn estimate_of(wu: &Value) -> f64 {
    match wu.get("estimate") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Convert a summed f64 to a JSON number Value matching JS `JSON.stringify`
/// semantics: whole values render without a decimal point (`5`, not `5.0`),
/// fractional values keep their decimals (`2.5`).
fn number_value(sum: f64) -> Value {
    if sum.is_finite() && sum.fract() == 0.0 && (0.0..=(u64::MAX as f64)).contains(&sum) {
        Value::from(sum as u64)
    } else {
        Value::from(sum)
    }
}

/// Render a numeric report field the way a JS template literal would
/// (`${value}`): integers without a decimal point, fractions with them.
fn js_number_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_u64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                format!("{f}")
            } else {
                "0".to_string()
            }
        }
        _ => "0".to_string(),
    }
}

/// Render the markdown report (mirrors TS `generateMarkdownReport`).
fn generate_markdown_report(report: &Map<String, Value>) -> String {
    let total_work_units = report
        .get("totalWorkUnits")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_story_points = js_number_str(report.get("totalStoryPoints"));

    let mut md = String::from("# Project Summary Report\n\n");
    md.push_str(&format!("**Total Work Units:** {total_work_units}\n\n"));
    md.push_str(&format!("**Total Story Points:** {total_story_points}\n\n"));
    md.push_str("## Breakdown by Status\n\n");

    if let Some(Value::Object(by_status)) = report.get("byStatus") {
        for (status, count) in by_status {
            let c = count.as_u64().unwrap_or(0);
            md.push_str(&format!("- **{status}:** {c}\n"));
        }
    }

    md.push_str("\n## Velocity Metrics\n\n");
    if let Some(Value::Object(velocity)) = report.get("velocity") {
        let cwu = velocity
            .get("completedWorkUnits")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let cp = js_number_str(velocity.get("completedPoints"));
        md.push_str(&format!("- **Completed Work Units:** {cwu}\n"));
        md.push_str(&format!("- **Completed Story Points:** {cp}\n"));
    }

    md
}

/// Resolve `file` against `project_root` when it is a relative path.
fn resolve_output(project_root: &Path, file: &str) -> PathBuf {
    let p = Path::new(file);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        project_root.join(p)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case_and_defaults() {
        let a: GenerateSummaryReportArgs =
            serde_json::from_str(r#"{"format":"json","output":"out.json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
        assert_eq!(a.output.as_deref(), Some("out.json"));

        let b: GenerateSummaryReportArgs = serde_json::from_str("{}").unwrap();
        assert!(b.format.is_none());
        assert!(b.output.is_none());
    }

    #[test]
    fn markdown_report_renders_sections() {
        let mut velocity = Map::new();
        velocity.insert("completedPoints".to_string(), Value::from(8u64));
        velocity.insert("completedWorkUnits".to_string(), Value::from(2u64));
        let mut by_status = Map::new();
        by_status.insert("done".to_string(), Value::from(2u64));
        let mut report = Map::new();
        report.insert("totalWorkUnits".to_string(), Value::from(3u64));
        report.insert("byStatus".to_string(), Value::Object(by_status));
        report.insert("totalStoryPoints".to_string(), Value::from(10u64));
        report.insert("velocity".to_string(), Value::Object(velocity));

        let md = generate_markdown_report(&report);
        assert!(md.contains("# Project Summary Report"));
        assert!(md.contains("**Total Work Units:** 3"));
        assert!(md.contains("**Total Story Points:** 10"));
        assert!(md.contains("- **done:** 2"));
        assert!(md.contains("- **Completed Work Units:** 2"));
        assert!(md.contains("- **Completed Story Points:** 8"));
    }
}
