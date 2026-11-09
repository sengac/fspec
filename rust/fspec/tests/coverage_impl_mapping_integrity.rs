//! Feature: spec/features/coverage-implementation-mapping-integrity.feature

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf()
}

fn line_numbers(lines: &serde_json::Value) -> Vec<usize> {
    let parse = |text: &str| -> Vec<usize> {
        text.split(',')
            .flat_map(|part| {
                let mut bounds = part.trim().splitn(2, '-').filter_map(|n| n.parse().ok());
                match (bounds.next(), bounds.next()) {
                    (Some(start), Some(end)) => (start..=end).collect(),
                    (Some(line), None) => vec![line],
                    _ => Vec::new(),
                }
            })
            .collect()
    };
    match lines {
        serde_json::Value::Array(items) => items
            .iter()
            .flat_map(|item| match item {
                serde_json::Value::Number(n) => {
                    n.as_u64().map(|n| n as usize).into_iter().collect()
                }
                serde_json::Value::String(text) => parse(text),
                _ => Vec::new(),
            })
            .collect(),
        serde_json::Value::String(text) => parse(text),
        _ => Vec::new(),
    }
}

#[test]
fn every_implementation_mapping_points_at_existing_source_lines(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the coverage sidecars under spec/features
    let root = repo_root();
    let features = root.join("spec/features");
    let mut stale = Vec::new();

    // @step When every implementation mapping is resolved against the repository
    for entry in std::fs::read_dir(&features)? {
        let path = entry?.path();
        if !path.to_string_lossy().ends_with(".feature.coverage") {
            continue;
        }
        let coverage: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        let scenarios = coverage["scenarios"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for scenario in scenarios {
            for test in scenario["testMappings"]
                .as_array()
                .cloned()
                .unwrap_or_default()
            {
                for implementation in test["implMappings"].as_array().cloned().unwrap_or_default() {
                    let file = implementation["file"].as_str().unwrap_or_default();
                    // @step Then each mapped file exists
                    // @step And each mapped line is within that file
                    let length = std::fs::read_to_string(root.join(file))
                        .map(|text| text.lines().count())
                        .unwrap_or(0);
                    let last = line_numbers(&implementation["lines"])
                        .into_iter()
                        .max()
                        .unwrap_or(0);
                    if length == 0 || last > length {
                        stale.push(format!(
                            "{}: {} -> {file}:{last} (file has {length} lines)",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            scenario["name"].as_str().unwrap_or_default()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        stale.is_empty(),
        "{} stale implementation mappings, e.g.:\n{}",
        stale.len(),
        stale
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}
