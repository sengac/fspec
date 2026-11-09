//! REFAC-033: the compaction behavioural test target is split into small files.
//!
//! Feature: spec/features/compaction-behavioural-test-split.feature

use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_compaction_behavioural_test_files_stay_under_300_lines() -> TestResult {
    // @step Given the cmpct045 test target and its fragment directory
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = vec![tests.join("cmpct045_generate_compaction_behavioral.rs")];
    for entry in std::fs::read_dir(tests.join("cmpct045"))? {
        files.push(entry?.path());
    }

    // @step When the line count of each file is measured
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(oversized.is_empty(), "oversized files: {oversized:?}");
    Ok(())
}
