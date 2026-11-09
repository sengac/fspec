//! Per-feature JSON mode for `show-coverage`.

use super::*;

// JSON serialization for single-file mode. Declaration order matters —
// `serde_json` is built with `preserve_order` workspace-wide.
#[derive(Serialize)]
pub(super) struct SingleFileEnriched<'a> {
    #[serde(rename = "fileName")]
    file_name: &'a str,
    scenarios: Vec<EnrichedScenario<'a>>,
    stats: &'a CoverageStats,
    warnings: Option<&'a [String]>,
}

#[derive(Serialize)]
pub(super) struct EnrichedScenario<'a> {
    name: &'a str,
    #[serde(rename = "testMappings")]
    test_mappings: &'a [crate::types::coverage::TestMapping],
    #[serde(rename = "coverageStatus")]
    coverage_status: &'static str,
    // COV-056: emitted only when true so untagged features stay byte-identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    deprecated: Option<bool>,
}

#[allow(clippy::expect_used)] // stats invariant: caller synthesizes before render
pub(super) fn render_single_json(
    coverage: &CoverageFile,
    file_name: &str,
    warnings: &[String],
    deprecated: &std::collections::HashSet<String>,
) -> String {
    let enriched_scenarios: Vec<EnrichedScenario> = coverage
        .scenarios
        .iter()
        .map(|s| {
            let is_dep = deprecated.contains(&s.name);
            EnrichedScenario {
                name: &s.name,
                test_mappings: &s.test_mappings,
                coverage_status: coverage_status(s).json_str(),
                deprecated: is_dep.then_some(true),
            }
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
