use crate::support::*;
use serde_json::json;

#[test]
fn selected_link_retains_file_and_story_step_validation() -> TestResult {
    // @step Given a fresh selector for a story coverage row
    let temporary = fixture()?;
    let original = std::fs::read(temporary.path().join(SIDECAR))?;

    // @step When missing files or tests without matching step comments are linked
    let missing = indexed_link(temporary.path(), 1, "src/missing.rs")?;
    let missing_result = request(temporary.path(), "link-coverage", missing);
    assert!(!missing_result.success, "missing implementation unexpectedly linked");
    let missing_error = missing_result.error.ok_or("missing-file diagnostic")?;
    assert!(missing_error.contains("src/missing.rs"), "{missing_error}");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    write(temporary.path(), TEST_FILE, "#[test]\nfn unrelated() {}\n")?;
    let mismatch = indexed_link(temporary.path(), 1, "src/second.rs")?;
    let mismatch_result = request(temporary.path(), "link-coverage", mismatch.clone());

    // @step Then normal validation rejects the request without writes
    assert!(!mismatch_result.success, "missing story steps unexpectedly linked");
    let diagnostic = mismatch_result.error.ok_or("step mismatch diagnostic")?;
    assert!(diagnostic.to_lowercase().contains("step"), "{diagnostic}");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    let mut skipped = mismatch;
    skipped["skipStepValidation"] = json!(true);
    let skipped_result = request(temporary.path(), "link-coverage", skipped);
    assert!(!skipped_result.success, "index cannot bypass story validation");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    Ok(())
}
