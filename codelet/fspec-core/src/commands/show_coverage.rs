//! `show-coverage` — Rust port of `src/commands/show-coverage.ts` (RPC-300).
//!
//! Loads a `.feature.coverage` JSON sidecar (per-feature mode) or every
//! sidecar under `spec/features/` (project-wide mode), enriches each entry
//! with missing-file warnings, synthesizes a `stats` block when the legacy
//! file omits it, and renders either markdown or 2-space JSON.
//!
//! Two-front-doors invariant (RPC-003 §7/§11): the LLM dispatcher AND the
//! standalone clap CLI both call this single `run` function — the CLI
//! bridge contains zero coverage logic.
//!
//! ## Behaviour parity with TypeScript (`src/commands/show-coverage.ts`)
//!
//! * Per-feature mode (positional `featureName` present):
//!   * Resolves `<root>/spec/features/<name>.feature.coverage`; tolerates
//!     a trailing `.feature` on the bare name.
//!   * Missing → `FspecCoreError::Io` whose `source` message starts with
//!     `Coverage file not found: <name>.feature.coverage` and ends with a
//!     create-feature suggestion line.
//!   * Invalid JSON → `FspecCoreError::InvalidArgs` whose reason begins
//!     `Invalid JSON in coverage file: <name>.feature.coverage` and ends
//!     with `Suggestion: Validate the JSON or recreate the file`.
//!   * Markdown sections: title, coverage line, `## Summary` (with line
//!     order `Total Scenarios, Covered, Uncovered, Test Files,
//!     Implementation Files, Test Lines, Implementation Lines, Total
//!     Lines`), optional `## Warnings`, `## Scenarios`, optional
//!     `## ⚠️  Coverage Gaps`.
//!   * JSON: top-level keys `fileName, scenarios, stats, warnings` in
//!     declaration order; each scenario has an appended `coverageStatus`.
//!
//! * Project-wide mode (no `featureName`):
//!   * Reads every `*.feature.coverage` under `spec/features/`.
//!   * Missing dir → `FspecCoreError::Io` with `Features directory not
//!     found: spec/features/` + suggestion.
//!   * Empty dir → `FspecCoreError::Io` with `No coverage files found in
//!     spec/features/` + suggestion.
//!   * Invalid JSON files are silently skipped.
//!   * Markdown: `# Project Coverage Report`, overall coverage,
//!     `## Project Summary`, `## Features Overview` (✅/⚠️/❌ by
//!     percentage band), `## Detailed Coverage by Feature`.
//!   * JSON: top-level `{aggregated: {totalFeatures, totalScenarios,
//!     coveredScenarios, coveragePercent}, features: [{fileName,
//!     coverage}, ...]}`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::types::coverage::{
    calculate_stats, CoverageFile, CoverageScenario, CoverageStats, ImplLines,
};

/// CLI / dispatcher arguments accepted by `show-coverage`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowCoverageArgs {
    /// Feature basename (e.g. `"user-login"`), with or without `.feature`.
    #[serde(default)]
    feature_name: Option<String>,
    /// `"markdown"` (default) or `"json"`. Accept `"text"` as an alias for
    /// `"markdown"` since the TS Commander.js shape used both at various
    /// points.
    #[serde(default)]
    format: Option<String>,
    /// Optional output file path (project-root-relative). When `Some`,
    /// the rendered content is also written to `<project_root>/<output>`
    /// after a successful render — mirrors the parity behaviour of
    /// `commands::show_feature::run`. The CLI bridge prints a confirmation
    /// line ("✓ Coverage report written to <path>") instead of stdout.
    #[serde(default)]
    output: Option<String>,
}

/// Coverage status of a single scenario, per the TS `CoverageStatus` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoverageStatusKind {
    FullyCovered,
    PartiallyCovered,
    Uncovered,
}

impl CoverageStatusKind {
    fn symbol(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "✅",
            CoverageStatusKind::PartiallyCovered => "⚠️",
            CoverageStatusKind::Uncovered => "❌",
        }
    }
    fn label(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "FULLY COVERED",
            CoverageStatusKind::PartiallyCovered => "PARTIALLY COVERED",
            CoverageStatusKind::Uncovered => "UNCOVERED",
        }
    }
    fn json_str(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "fully-covered",
            CoverageStatusKind::PartiallyCovered => "partially-covered",
            CoverageStatusKind::Uncovered => "uncovered",
        }
    }
}

fn coverage_status(scenario: &CoverageScenario) -> CoverageStatusKind {
    if scenario.test_mappings.is_empty() {
        return CoverageStatusKind::Uncovered;
    }
    let has_impl = scenario
        .test_mappings
        .iter()
        .any(|tm| !tm.impl_mappings.is_empty());
    if has_impl {
        CoverageStatusKind::FullyCovered
    } else {
        CoverageStatusKind::PartiallyCovered
    }
}

/// Dispatcher entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowCoverageArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-coverage",
            reason: format!("failed to parse args: {e}"),
        })?;

    let format = args.format.as_deref().unwrap_or("markdown");
    let format = match format {
        "json" => OutputFormat::Json,
        // TS defaults to markdown; "text" used as alias by some callers.
        _ => OutputFormat::Markdown,
    };

    let rendered = match args.feature_name.as_deref() {
        Some(name) => show_single_feature(project_root, name, format),
        None => show_project_wide(project_root, format),
    }?;

    // Optional writeback (parity with show-feature). On success only.
    if let Some(out_rel) = args.output.as_deref() {
        let out_abs = project_root.join(out_rel);
        if let Some(parent) = out_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
                command: "show-coverage",
                source,
            })?;
        }
        std::fs::write(&out_abs, rendered.as_bytes()).map_err(|source| FspecCoreError::Io {
            command: "show-coverage",
            source,
        })?;
    }

    Ok(rendered)
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Markdown,
    Json,
}

// ─────────────────────────── Per-feature mode ───────────────────────────

fn show_single_feature(
    project_root: &Path,
    feature_input: &str,
    format: OutputFormat,
) -> Result<String, FspecCoreError> {
    let features_dir = project_root.join("spec").join("features");
    let stripped = feature_input
        .strip_suffix(".feature")
        .unwrap_or(feature_input);
    let file_name = format!("{stripped}.feature");
    let coverage_path = features_dir.join(format!("{file_name}.coverage"));

    if !coverage_path.exists() {
        let msg = format!(
            "Coverage file not found: {file_name}.coverage\nSuggestion: Run 'fspec create-feature' to create the feature with coverage tracking"
        );
        return Err(FspecCoreError::Io {
            command: "show-coverage",
            source: std::io::Error::new(std::io::ErrorKind::NotFound, msg),
        });
    }

    let content = std::fs::read_to_string(&coverage_path).map_err(|source| FspecCoreError::Io {
        command: "show-coverage",
        source,
    })?;

    let mut coverage: CoverageFile = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            return Err(FspecCoreError::InvalidArgs {
                command: "show-coverage",
                reason: format!(
                    "Invalid JSON in coverage file: {file_name}.coverage\n  Parse error: {e}\nSuggestion: Validate the JSON or recreate the file"
                ),
            });
        }
    };

    if coverage.stats.is_none() {
        coverage.stats = Some(calculate_stats(&coverage.scenarios));
    }

    let warnings = collect_warnings(&coverage, project_root);

    match format {
        OutputFormat::Json => Ok(render_single_json(&coverage, &file_name, &warnings)),
        OutputFormat::Markdown => Ok(render_single_markdown(&coverage, &file_name, &warnings)),
    }
}

fn collect_warnings(coverage: &CoverageFile, project_root: &Path) -> Vec<String> {
    let mut warnings: Vec<String> = Vec::new();
    for scenario in &coverage.scenarios {
        for tm in &scenario.test_mappings {
            if !project_root.join(&tm.file).exists() {
                warnings.push(format!("⚠️  File not found: {}", tm.file));
            }
            for im in &tm.impl_mappings {
                if !project_root.join(&im.file).exists() {
                    warnings.push(format!("⚠️  File not found: {}", im.file));
                }
            }
        }
    }
    warnings
}

struct LineCounts {
    test_lines: i64,
    impl_lines: i64,
    total_lines: i64,
}

fn calculate_line_counts(coverage: &CoverageFile) -> LineCounts {
    let mut test_lines: i64 = 0;
    let mut impl_lines: i64 = 0;
    for scenario in &coverage.scenarios {
        for tm in &scenario.test_mappings {
            let range: Vec<&str> = tm.lines.split('-').collect();
            if range.len() == 2 {
                if let (Ok(s), Ok(e)) = (range[0].parse::<i64>(), range[1].parse::<i64>()) {
                    test_lines += e - s + 1;
                }
            }
            for im in &tm.impl_mappings {
                match &im.lines {
                    ImplLines::Array(v) => {
                        impl_lines += v.len() as i64;
                    }
                    ImplLines::String(s) => {
                        let parts: Vec<&str> = s.split('-').collect();
                        if parts.len() == 2 {
                            if let (Ok(a), Ok(b)) =
                                (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                            {
                                impl_lines += b - a + 1;
                            }
                        }
                    }
                }
            }
        }
    }
    LineCounts {
        test_lines,
        impl_lines,
        total_lines: test_lines + impl_lines,
    }
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
fn render_single_markdown(coverage: &CoverageFile, file_name: &str, warnings: &[String]) -> String {
    let mut lines: Vec<String> = Vec::new();

    let stats = coverage
        .stats
        .as_ref()
        .expect("stats must be synthesized by caller");

    lines.push(format!("# Coverage Report: {file_name}"));
    lines.push(String::new());
    lines.push(format!(
        "**Coverage**: {}% ({}/{} scenarios)",
        stats.coverage_percent, stats.covered_scenarios, stats.total_scenarios
    ));
    lines.push(String::new());

    lines.push("## Summary".to_string());
    lines.push(format!("- Total Scenarios: {}", stats.total_scenarios));
    lines.push(format!("- Covered: {}", stats.covered_scenarios));
    lines.push(format!(
        "- Uncovered: {}",
        stats
            .total_scenarios
            .saturating_sub(stats.covered_scenarios)
    ));
    lines.push(format!("- Test Files: {}", stats.test_files.len()));
    lines.push(format!(
        "- Implementation Files: {}",
        stats.impl_files.len()
    ));

    let lc = calculate_line_counts(coverage);
    lines.push(format!("- Test Lines: {}", lc.test_lines));
    lines.push(format!("- Implementation Lines: {}", lc.impl_lines));
    lines.push(format!("- Total Lines: {}", lc.total_lines));
    lines.push(String::new());

    if !warnings.is_empty() {
        lines.push("## Warnings".to_string());
        for w in warnings {
            lines.push(w.clone());
        }
        lines.push(String::new());
    }

    lines.push("## Scenarios".to_string());
    lines.push(String::new());

    for scenario in &coverage.scenarios {
        let status = coverage_status(scenario);
        lines.push(format!(
            "### {} {} ({})",
            status.symbol(),
            scenario.name,
            status.label()
        ));

        if scenario.test_mappings.is_empty() {
            lines.push("- No test mappings".to_string());
        } else {
            for tm in &scenario.test_mappings {
                lines.push(format!("- **Test**: `{}:{}`", tm.file, tm.lines));
                if tm.impl_mappings.is_empty() {
                    lines.push("- **Implementation**: ⚠️  No implementation mappings".to_string());
                } else {
                    for im in &tm.impl_mappings {
                        let impl_lines_str = match &im.lines {
                            ImplLines::Array(v) => {
                                v.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
                            }
                            ImplLines::String(s) => s.clone(),
                        };
                        lines.push(format!(
                            "- **Implementation**: `{}:{}`",
                            im.file, impl_lines_str
                        ));
                    }
                }
            }
        }
        lines.push(String::new());
    }

    let uncovered: Vec<&CoverageScenario> = coverage
        .scenarios
        .iter()
        .filter(|s| s.test_mappings.is_empty())
        .collect();

    if !uncovered.is_empty() {
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("## ⚠️  Coverage Gaps".to_string());
        lines.push(String::new());
        lines.push("The following scenarios need test coverage:".to_string());
        lines.push(String::new());
        for s in &uncovered {
            lines.push(format!("- {}", s.name));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

// JSON serialization for single-file mode. Declaration order matters —
// `serde_json` is built with `preserve_order` workspace-wide.
#[derive(Serialize)]
struct SingleFileEnriched<'a> {
    #[serde(rename = "fileName")]
    file_name: &'a str,
    scenarios: Vec<EnrichedScenario<'a>>,
    stats: &'a CoverageStats,
    warnings: Option<&'a [String]>,
}

#[derive(Serialize)]
struct EnrichedScenario<'a> {
    name: &'a str,
    #[serde(rename = "testMappings")]
    test_mappings: &'a [crate::types::coverage::TestMapping],
    #[serde(rename = "coverageStatus")]
    coverage_status: &'static str,
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
fn render_single_json(coverage: &CoverageFile, file_name: &str, warnings: &[String]) -> String {
    let enriched_scenarios: Vec<EnrichedScenario> = coverage
        .scenarios
        .iter()
        .map(|s| EnrichedScenario {
            name: &s.name,
            test_mappings: &s.test_mappings,
            coverage_status: coverage_status(s).json_str(),
        })
        .collect();

    let stats = coverage
        .stats
        .as_ref()
        .expect("stats must be synthesized by caller");

    let payload = SingleFileEnriched {
        file_name,
        scenarios: enriched_scenarios,
        stats,
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    };

    serde_json::to_string_pretty(&payload).unwrap_or_default()
}

// ─────────────────────────── Project-wide mode ───────────────────────────

struct LoadedFeature {
    file_name: String,
    coverage: CoverageFile,
    warnings: Vec<String>,
}

fn show_project_wide(project_root: &Path, format: OutputFormat) -> Result<String, FspecCoreError> {
    let features_dir = project_root.join("spec").join("features");
    let entries = match std::fs::read_dir(&features_dir) {
        Ok(e) => e,
        Err(_) => {
            let msg = "Features directory not found: spec/features/\nSuggestion: Run 'fspec create-feature' to create your first feature".to_string();
            return Err(FspecCoreError::Io {
                command: "show-coverage",
                source: std::io::Error::new(std::io::ErrorKind::NotFound, msg),
            });
        }
    };

    let mut coverage_paths: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(s) = path.file_name().and_then(|f| f.to_str()) {
            if s.ends_with(".feature.coverage") {
                coverage_paths.push(path);
            }
        }
    }
    // Stable ordering for deterministic output (mirrors TS readdir ordering on Linux).
    coverage_paths.sort();

    if coverage_paths.is_empty() {
        let msg = "No coverage files found in spec/features/\nSuggestion: Run 'fspec create-feature' to create features with coverage tracking".to_string();
        return Err(FspecCoreError::Io {
            command: "show-coverage",
            source: std::io::Error::new(std::io::ErrorKind::NotFound, msg),
        });
    }

    let mut loaded: Vec<LoadedFeature> = Vec::new();
    for path in coverage_paths {
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut cov) = serde_json::from_str::<CoverageFile>(&body) else {
            continue;
        };
        if cov.stats.is_none() {
            cov.stats = Some(calculate_stats(&cov.scenarios));
        }
        let warnings = collect_warnings(&cov, project_root);
        let raw_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_string();
        let file_name = raw_name.replace(".feature.coverage", ".feature");
        loaded.push(LoadedFeature {
            file_name,
            coverage: cov,
            warnings,
        });
    }

    let total_features = loaded.len() as i64;
    let total_scenarios: i64 = loaded
        .iter()
        .map(|f| {
            f.coverage
                .stats
                .as_ref()
                .map(|s| s.total_scenarios as i64)
                .unwrap_or(0)
        })
        .sum();
    let covered_scenarios: i64 = loaded
        .iter()
        .map(|f| {
            f.coverage
                .stats
                .as_ref()
                .map(|s| s.covered_scenarios as i64)
                .unwrap_or(0)
        })
        .sum();
    let coverage_percent: i64 = if total_scenarios == 0 {
        0
    } else {
        ((covered_scenarios as f64) / (total_scenarios as f64) * 100.0).round() as i64
    };

    let aggregated = Aggregated {
        total_features,
        total_scenarios,
        covered_scenarios,
        coverage_percent,
    };

    match format {
        OutputFormat::Json => Ok(render_project_json(&aggregated, &loaded)),
        OutputFormat::Markdown => Ok(render_project_markdown(&aggregated, &loaded)),
    }
}

#[derive(Serialize)]
struct Aggregated {
    #[serde(rename = "totalFeatures")]
    total_features: i64,
    #[serde(rename = "totalScenarios")]
    total_scenarios: i64,
    #[serde(rename = "coveredScenarios")]
    covered_scenarios: i64,
    #[serde(rename = "coveragePercent")]
    coverage_percent: i64,
}

#[derive(Serialize)]
struct ProjectWidePayload<'a> {
    aggregated: &'a Aggregated,
    features: Vec<ProjectFeatureEntry<'a>>,
}

#[derive(Serialize)]
struct ProjectFeatureEntry<'a> {
    #[serde(rename = "fileName")]
    file_name: &'a str,
    coverage: ProjectFeatureCoverage<'a>,
}

#[derive(Serialize)]
struct ProjectFeatureCoverage<'a> {
    scenarios: &'a [CoverageScenario],
    stats: &'a CoverageStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<&'a [String]>,
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
fn render_project_json(aggregated: &Aggregated, loaded: &[LoadedFeature]) -> String {
    let features: Vec<ProjectFeatureEntry> = loaded
        .iter()
        .map(|lf| ProjectFeatureEntry {
            file_name: &lf.file_name,
            coverage: ProjectFeatureCoverage {
                scenarios: &lf.coverage.scenarios,
                stats: lf
                    .coverage
                    .stats
                    .as_ref()
                    .expect("stats synthesized by caller"),
                warnings: if lf.warnings.is_empty() {
                    None
                } else {
                    Some(&lf.warnings)
                },
            },
        })
        .collect();
    let payload = ProjectWidePayload {
        aggregated,
        features,
    };
    serde_json::to_string_pretty(&payload).unwrap_or_default()
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
fn render_project_markdown(aggregated: &Aggregated, loaded: &[LoadedFeature]) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("# Project Coverage Report".to_string());
    lines.push(String::new());
    lines.push(format!(
        "**Overall Coverage**: {}% ({}/{} scenarios)",
        aggregated.coverage_percent, aggregated.covered_scenarios, aggregated.total_scenarios
    ));
    lines.push(String::new());

    lines.push("## Project Summary".to_string());
    lines.push(format!("- Total Features: {}", aggregated.total_features));
    lines.push(format!("- Total Scenarios: {}", aggregated.total_scenarios));
    lines.push(format!("- Covered: {}", aggregated.covered_scenarios));
    lines.push(format!(
        "- Uncovered: {}",
        aggregated
            .total_scenarios
            .saturating_sub(aggregated.covered_scenarios)
    ));
    lines.push(String::new());

    lines.push("## Features Overview".to_string());
    lines.push(String::new());
    for lf in loaded {
        let stats = lf
            .coverage
            .stats
            .as_ref()
            .expect("stats synthesized by caller");
        let percent = stats.coverage_percent;
        let symbol = if percent == 100 {
            "✅"
        } else if percent >= 50 {
            "⚠️"
        } else {
            "❌"
        };
        lines.push(format!(
            "- {}: {}% ({}/{}) {}",
            lf.file_name, percent, stats.covered_scenarios, stats.total_scenarios, symbol
        ));
    }
    lines.push(String::new());

    lines.push("---".to_string());
    lines.push(String::new());
    lines.push("## Detailed Coverage by Feature".to_string());
    lines.push(String::new());

    for lf in loaded {
        let stats = lf
            .coverage
            .stats
            .as_ref()
            .expect("stats synthesized by caller");
        lines.push(format!("### {}", lf.file_name));
        lines.push(format!(
            "**Coverage**: {}% ({}/{} scenarios)",
            stats.coverage_percent, stats.covered_scenarios, stats.total_scenarios
        ));
        lines.push(String::new());
        for s in &lf.coverage.scenarios {
            let status = coverage_status(s);
            lines.push(format!("- {} {}", status.symbol(), s.name));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ShowCoverageArgs = serde_json::from_str("{}").unwrap();
        assert!(a.feature_name.is_none());
        assert!(a.format.is_none());
        assert!(a.output.is_none());
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ShowCoverageArgs =
            serde_json::from_str(r#"{"featureName":"auth","format":"json"}"#).unwrap();
        assert_eq!(a.feature_name.as_deref(), Some("auth"));
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn coverage_status_uncovered_when_no_test_mappings() {
        let s = CoverageScenario {
            name: "X".into(),
            test_mappings: vec![],
            extra: Default::default(),
        };
        assert_eq!(coverage_status(&s), CoverageStatusKind::Uncovered);
    }
}
