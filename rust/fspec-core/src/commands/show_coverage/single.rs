//! Per-feature markdown mode for `show-coverage`.

use super::*;

// ─────────────────────────── Per-feature mode ───────────────────────────

pub(super) fn show_single_feature(
    project_root: &Path,
    feature_input: &str,
    format: OutputFormat,
) -> Result<String, FspecCoreError> {
    let features_dir = crate::io::spec_paths::features_dir_of(project_root);
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

    // COV-056: read @deprecated scenario names from the sibling .feature file
    // (parse failure → empty set, no markers rendered).
    let feature_path = features_dir.join(file_name.as_str());
    // BUG-563: a Rule block would hide its scenarios.
    crate::io::gherkin::reject_rule_blocks(&feature_path, "show-coverage")?;
    let deprecated: std::collections::HashSet<String> = std::fs::read_to_string(&feature_path)
        .ok()
        .and_then(|content| crate::types::coverage::deprecated_scenario_names(&content))
        .unwrap_or_default();

    match format {
        OutputFormat::Json => Ok(render_single_json(
            &coverage,
            &file_name,
            &warnings,
            &deprecated,
        )),
        OutputFormat::Markdown => Ok(render_single_markdown(
            &coverage,
            &file_name,
            &warnings,
            &deprecated,
        )),
    }
}

pub(super) fn collect_warnings(coverage: &CoverageFile, project_root: &Path) -> Vec<String> {
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

pub(super) struct LineCounts {
    test_lines: i64,
    impl_lines: i64,
    total_lines: i64,
}

pub(super) fn calculate_line_counts(coverage: &CoverageFile) -> LineCounts {
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
pub(super) fn render_single_markdown(
    coverage: &CoverageFile,
    file_name: &str,
    warnings: &[String],
    deprecated: &std::collections::HashSet<String>,
) -> String {
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
        // COV-056: additive DEPRECATED marker (byte-identical output when the
        // scenario is not deprecated — see the untagged byte-parity scenario).
        if deprecated.contains(&scenario.name) {
            lines.push(format!(
                "### {} {} ({}) [DEPRECATED]",
                status.symbol(),
                scenario.name,
                status.label()
            ));
        } else {
            lines.push(format!(
                "### {} {} ({})",
                status.symbol(),
                scenario.name,
                status.label()
            ));
        }

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
