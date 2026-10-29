use crate::support::*;
use serde_json::json;

#[test]
fn selected_second_duplicate_gains_complete_two_file_union() -> TestResult {
    // @step Given two same-file mapping rows with distinct retained lineage
    let temporary = fixture()?;
    let original = load(temporary.path())?;
    let args = indexed_link(temporary.path(), 1, "src/second.rs")?;

    // @step When the second row gains another physical implementation using its fresh hash
    let result = successful_json(request(temporary.path(), "link-coverage", args))?;

    // @step Then only the selected row gains the complete two file union
    let actual = load(temporary.path())?;
    let mut expected = original["scenarios"].clone();
    expected[0]["testMappings"][1]["implMappings"].as_array_mut()
        .ok_or("implementation array")?.push(json!({"file": "src/second.rs", "lines": [1, 2]}));
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    assert_eq!(result["scenarioHash"], scenario_hash(&actual["scenarios"][0])?);
    assert_ne!(result["scenarioHash"], scenario_hash(&original["scenarios"][0])?);
    Ok(())
}

#[test]
fn unselected_mutation_retains_legacy_duplicate_selection_behavior() -> TestResult {
    // @step Given two same-file mapping rows with distinct retained lineage
    let temporary = fixture()?;
    let original = load(temporary.path())?;

    // @step When implementation linking and unlinking omit the new selector
    let linked = request(temporary.path(), "link-coverage", json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "implFile": "src/second.rs", "implLines": "1,2"}));
    assert!(linked.success, "legacy link failed: {linked:?}");
    let intermediate = load(temporary.path())?;
    let unlinked = request(temporary.path(), "unlink-coverage", json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "implFile": "src/second.rs"}));

    // @step Then the legacy first matching row behavior is unchanged
    assert!(unlinked.success, "legacy unlink failed: {unlinked:?}");
    assert_eq!(intermediate["scenarios"][0]["testMappings"][0]["implMappings"][1],
        json!({"file": "src/second.rs", "lines": [1, 2]}));
    assert_eq!(intermediate["scenarios"][0]["testMappings"][1], original["scenarios"][0]["testMappings"][1]);
    assert_eq!(load(temporary.path())?["scenarios"], original["scenarios"]);
    Ok(())
}
