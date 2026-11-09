//! REFAC-034: the GenerateCompaction feature is split into small files.
//!
//! Feature: spec/features/generate-compaction-feature-split.feature

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WIRING_SCENARIOS: [&str; 3] = [
    "Scenario: Compactor sub-agent provider arms match the DeepSearch clone",
    "Scenario: GenerateCompaction tool wired into provider agent builders",
    "Scenario: Handler registration lifecycle mirrors the DeepSearch pattern",
];

fn features_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("agent-loop manifest has no repository root two levels up")?;
    Ok(repo.join("spec").join("features"))
}

#[test]
fn the_generate_compaction_feature_files_stay_under_300_lines() -> TestResult {
    // @step Given the GenerateCompaction tool feature and its wiring feature
    let dir = features_dir()?;
    let tool = std::fs::read_to_string(dir.join("generate-compaction-tool.feature"))?;
    let wiring = std::fs::read_to_string(dir.join("generate-compaction-tool-wiring.feature"))?;

    // @step When the line count of each feature file is measured
    let counts = [
        ("generate-compaction-tool.feature", tool.lines().count()),
        (
            "generate-compaction-tool-wiring.feature",
            wiring.lines().count(),
        ),
    ];

    // @step Then no feature file has 300 or more lines
    let oversized: Vec<_> = counts.iter().filter(|(_, lines)| *lines >= 300).collect();
    assert!(
        oversized.is_empty(),
        "oversized feature files: {oversized:?}"
    );

    // @step And the three wiring scenarios are in the wiring feature and not in the tool feature
    for scenario in WIRING_SCENARIOS {
        assert!(
            wiring.contains(scenario),
            "wiring feature lacks '{scenario}'"
        );
        assert!(
            !tool.contains(scenario),
            "tool feature still has '{scenario}'"
        );
    }
    Ok(())
}
