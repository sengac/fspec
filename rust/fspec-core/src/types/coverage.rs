//! Sidecar coverage-file types — shared across `show-coverage`,
//! `link-coverage`, `audit-coverage`, `generate-coverage`, and
//! `unlink-coverage` command ports.
//!
//! Mirrors `src/utils/coverage-file.ts` (the TypeScript source of truth):
//!
//! ```ignore
//! interface CoverageFile {
//!   scenarios: CoverageScenario[];
//!   stats: CoverageStats;
//! }
//! interface CoverageScenario { name: string; testMappings: TestMapping[]; }
//! interface TestMapping { file: string; lines: string; implMappings: ImplMapping[]; }
//! interface ImplMapping { file: string; lines: number[] | string; }
//! interface CoverageStats {
//!   totalScenarios: number; coveredScenarios: number; coveragePercent: number;
//!   testFiles: string[]; implFiles: string[]; totalLinesCovered: number;
//! }
//! ```
//!
//! Every struct uses `#[serde(rename_all = "camelCase")]` for TS-parity
//! field naming, and `#[serde(flatten)] extra` to preserve unknown fields
//! during round-trip parsing (so commands that mutate the file in-place
//! never strip caller-added metadata).
//!
//! The `serde_json` feature `preserve_order` is enabled workspace-wide
//! (`rust/Cargo.toml`), so `#[derive(Serialize)]` honours field
//! declaration order on the way back out — critical for the byte-parity
//! tests in `rust/fspec-core/tests/show_coverage.rs`.

use serde::{Deserialize, Serialize};

/// Sidecar `<feature>.feature.coverage` file body.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageFile {
    /// Per-scenario coverage entries. Optional in legacy files; the show-
    /// coverage command synthesizes an empty `Vec` when absent.
    #[serde(default)]
    pub scenarios: Vec<CoverageScenario>,
    /// Aggregated stats block. Legacy coverage files omit this entirely —
    /// callers (e.g. `show_coverage`) recompute via [`calculate_stats`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<CoverageStats>,
    /// Any unknown top-level fields are preserved here so commands that
    /// re-serialise the file (link-coverage / audit-coverage --fix) don't
    /// drop caller-added metadata.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One scenario's coverage state — a name plus zero or more test mappings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageScenario {
    pub name: String,
    #[serde(default)]
    pub test_mappings: Vec<TestMapping>,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A single test → implementation mapping. `lines` is a Gherkin-step-range
/// style string (e.g. `"45-62"`). Optional — some legacy fixtures use
/// `testFunction` instead of explicit line ranges.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestMapping {
    pub file: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lines: String,
    #[serde(default)]
    pub impl_mappings: Vec<ImplMapping>,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One implementation-file mapping. `lines` is intentionally untagged —
/// the TS shape allows EITHER a `number[]` of explicit line numbers OR a
/// `"N-M"` string range, and both forms appear in real-world fixtures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplMapping {
    pub file: String,
    pub lines: ImplLines,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Implementation-mapping `lines` field — either an explicit array of line
/// numbers OR a string range like `"1-149"`. Mirrors the TS union type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImplLines {
    /// Explicit list of covered line numbers — `[10, 11, 12, 23, 24]`.
    Array(Vec<i64>),
    /// `"N-M"` range OR a single-line `"42"`.
    String(String),
}

impl Default for ImplLines {
    fn default() -> Self {
        ImplLines::Array(Vec::new())
    }
}

/// Aggregated stats block. Field DECLARATION ORDER matters for serialised
/// output — keep `totalScenarios → coveredScenarios → coveragePercent →
/// testFiles → implFiles → totalLinesCovered` because `preserve_order` is
/// enabled and the show-coverage byte-parity tests assert this order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageStats {
    pub total_scenarios: u64,
    pub covered_scenarios: u64,
    pub coverage_percent: i64,
    #[serde(default)]
    pub test_files: Vec<String>,
    #[serde(default)]
    pub impl_files: Vec<String>,
    #[serde(default)]
    pub total_lines_covered: u64,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Compute stats from a scenarios array — mirrors the TS
/// `calculateStats(coverage)` helper at `src/commands/show-coverage.ts:155`.
///
/// Dedups test/impl files in insertion order, uses Math.round semantics
/// (half-up) for `coverage_percent`, and sets `total_lines_covered = 0`
/// (legacy files don't carry it; the markdown renderer computes its own
/// line totals from ranges).
pub fn calculate_stats(scenarios: &[CoverageScenario]) -> CoverageStats {
    let total_scenarios = scenarios.len() as u64;
    let covered_scenarios = scenarios
        .iter()
        .filter(|s| !s.test_mappings.is_empty())
        .count() as u64;

    let mut test_files: Vec<String> = Vec::new();
    let mut impl_files: Vec<String> = Vec::new();

    for scenario in scenarios {
        for tm in &scenario.test_mappings {
            if !test_files.iter().any(|f| f == &tm.file) {
                test_files.push(tm.file.clone());
            }
            for im in &tm.impl_mappings {
                if !impl_files.iter().any(|f| f == &im.file) {
                    impl_files.push(im.file.clone());
                }
            }
        }
    }

    let coverage_percent = if total_scenarios == 0 {
        0
    } else {
        // Math.round semantics: half-up. `f64::round` is half-away-from-zero
        // which agrees with JS Math.round for non-negative inputs.
        ((covered_scenarios as f64) / (total_scenarios as f64) * 100.0).round() as i64
    };

    CoverageStats {
        total_scenarios,
        covered_scenarios,
        coverage_percent,
        test_files,
        impl_files,
        total_lines_covered: 0,
        extra: serde_json::Map::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn calculate_stats_empty() {
        let s = calculate_stats(&[]);
        assert_eq!(s.total_scenarios, 0);
        assert_eq!(s.covered_scenarios, 0);
        assert_eq!(s.coverage_percent, 0);
    }

    #[test]
    fn calculate_stats_one_covered_one_uncovered() {
        let scenarios = vec![
            CoverageScenario {
                name: "A".into(),
                test_mappings: vec![TestMapping {
                    file: "t1.ts".into(),
                    lines: "1-10".into(),
                    impl_mappings: vec![ImplMapping {
                        file: "i1.ts".into(),
                        lines: ImplLines::String("1-5".into()),
                        extra: Default::default(),
                    }],
                    extra: Default::default(),
                }],
                extra: Default::default(),
            },
            CoverageScenario {
                name: "B".into(),
                test_mappings: vec![],
                extra: Default::default(),
            },
        ];
        let s = calculate_stats(&scenarios);
        assert_eq!(s.total_scenarios, 2);
        assert_eq!(s.covered_scenarios, 1);
        assert_eq!(s.coverage_percent, 50);
        assert_eq!(s.test_files, vec!["t1.ts"]);
        assert_eq!(s.impl_files, vec!["i1.ts"]);
    }

    #[test]
    fn calculate_stats_dedup_files() {
        let scenarios = vec![
            CoverageScenario {
                name: "A".into(),
                test_mappings: vec![TestMapping {
                    file: "t.ts".into(),
                    lines: "1-10".into(),
                    impl_mappings: vec![ImplMapping {
                        file: "i.ts".into(),
                        lines: ImplLines::String("1-5".into()),
                        extra: Default::default(),
                    }],
                    extra: Default::default(),
                }],
                extra: Default::default(),
            },
            CoverageScenario {
                name: "B".into(),
                test_mappings: vec![TestMapping {
                    file: "t.ts".into(),
                    lines: "11-20".into(),
                    impl_mappings: vec![ImplMapping {
                        file: "i.ts".into(),
                        lines: ImplLines::String("6-10".into()),
                        extra: Default::default(),
                    }],
                    extra: Default::default(),
                }],
                extra: Default::default(),
            },
        ];
        let s = calculate_stats(&scenarios);
        assert_eq!(s.test_files.len(), 1);
        assert_eq!(s.impl_files.len(), 1);
    }

    #[test]
    fn impl_lines_round_trip_array() {
        let j = r#"{"file":"a.ts","lines":[1,2,3]}"#;
        let m: ImplMapping = serde_json::from_str(j).unwrap();
        match &m.lines {
            ImplLines::Array(v) => assert_eq!(v, &[1, 2, 3]),
            _ => panic!("expected Array"),
        }
        let back = serde_json::to_string(&m).unwrap();
        assert!(back.contains("[1,2,3]"));
    }

    #[test]
    fn impl_lines_round_trip_string() {
        let j = r#"{"file":"a.ts","lines":"1-149"}"#;
        let m: ImplMapping = serde_json::from_str(j).unwrap();
        match &m.lines {
            ImplLines::String(s) => assert_eq!(s, "1-149"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn extra_fields_preserved() {
        let j = r#"{"scenarios":[],"customField":"keepme"}"#;
        let c: CoverageFile = serde_json::from_str(j).unwrap();
        assert_eq!(
            c.extra.get("customField").and_then(|v| v.as_str()),
            Some("keepme")
        );
    }
}
