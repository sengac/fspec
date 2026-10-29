use crate::support::*;
use serde_json::json;

#[test]
fn selected_obsolete_duplicate_is_removed_without_losing_other_occurrences() -> TestResult {
    // @step Given identical duplicate rows and retained implementation arrays
    let temporary = fixture()?;
    let mut original = load(temporary.path())?;
    let mut second = mapping("obsolete");
    second["implMappings"].as_array_mut().ok_or("implementation array")?
        .push(json!({"file": "src/second.rs", "lines": [2, 1, 2], "unknown": true}));
    original["scenarios"][0]["testMappings"] = json!([mapping("keep"), second.clone(), second]);
    let selected = original["scenarios"][0].clone();
    let unrelated = json!({"name": "Unrelated retained thing",
        "unknownScenario": {"receipt": [9, 7, 9]},
        "testMappings": [mapping("unrelated-scenario")]});
    original["scenarios"] = json!([unrelated, selected]);
    original["stats"]["totalScenarios"] = json!(2);
    original["stats"]["coveredScenarios"] = json!(2);
    save(temporary.path(), &original)?;
    let mut expected = original["scenarios"].clone();
    expected[1]["testMappings"].as_array_mut().ok_or("mapping array")?.remove(1);

    // @step When one obsolete occurrence is explicitly unlinked with its fresh hash
    let removed = successful_json(request(temporary.path(), "unlink-coverage", json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "testMappingIndex": 1, "expectedScenarioHash": scenario_hash(&original["scenarios"][1])?})))?;

    // @step Then every other mapping occurrence and unknown member is retained
    let actual = load(temporary.path())?;
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(removed["scenarioHash"], scenario_hash(&expected[1])?);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    let next_hash = scenario_hash(&expected[1])?;
    expected[1]["testMappings"][1]["implMappings"].as_array_mut().ok_or("implementation array")?.remove(0);
    let implementation_removed = request(temporary.path(), "unlink-coverage", json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "testMappingIndex": 1, "expectedScenarioHash": next_hash,
        "implFile": "src/first.rs"}));
    assert!(implementation_removed.success, "indexed implementation unlink: {implementation_removed:?}");
    let final_coverage = load(temporary.path())?;
    assert_eq!(final_coverage["scenarios"], expected);
    assert_eq!(final_coverage["unknownTop"], original["unknownTop"]);
    assert_eq!(final_coverage["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    Ok(())
}
