//! Project-wide mode for `show-coverage`.

use super::*;

// ─────────────────────────── Project-wide mode ───────────────────────────

pub(super) struct LoadedFeature {
    file_name: String,
    coverage: CoverageFile,
    warnings: Vec<String>,
}

pub(super) fn show_project_wide(
    project_root: &Path,
    format: OutputFormat,
) -> Result<String, FspecCoreError> {
    let features_dir = crate::io::spec_paths::features_dir_of(project_root);
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
        // BUG-563: a Rule block would hide its scenarios.
        crate::io::gherkin::reject_rule_blocks(&path.with_file_name(&file_name), "show-coverage")?;
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
pub(super) struct Aggregated {
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
pub(super) struct ProjectWidePayload<'a> {
    aggregated: &'a Aggregated,
    features: Vec<ProjectFeatureEntry<'a>>,
}

#[derive(Serialize)]
pub(super) struct ProjectFeatureEntry<'a> {
    #[serde(rename = "fileName")]
    file_name: &'a str,
    coverage: ProjectFeatureCoverage<'a>,
}

#[derive(Serialize)]
pub(super) struct ProjectFeatureCoverage<'a> {
    scenarios: &'a [CoverageScenario],
    stats: &'a CoverageStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<&'a [String]>,
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
pub(super) fn render_project_json(aggregated: &Aggregated, loaded: &[LoadedFeature]) -> String {
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
pub(super) fn render_project_markdown(aggregated: &Aggregated, loaded: &[LoadedFeature]) -> String {
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
