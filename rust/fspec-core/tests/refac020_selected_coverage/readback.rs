use crate::support::*;
use serde_json::json;

#[test]
fn readback_exposes_ordered_rows_and_canonical_scenario_hash() -> TestResult {
    // @step Given duplicate mapping rows with unknown metadata
    let temporary = fixture()?;
    let original = load(temporary.path())?;
    let before = tree(temporary.path())?;
    let legacy_args = json!({"featureName": "thing", "format": "json"});
    let legacy = request(temporary.path(), "show-coverage", legacy_args.clone());
    assert!(legacy.success, "legacy readback fixture: {legacy:?}");

    // @step When explicit machine readback is requested
    let machine = successful_json(request(temporary.path(), "show-coverage",
        json!({"featureName": "thing", "mappingJson": true})))?;

    // @step Then row indices and the complete canonical scenario hash are exposed without writes
    assert_eq!(machine["fileName"], "thing.feature");
    assert_eq!(machine["scenarios"][0]["scenarioHash"], scenario_hash(&original["scenarios"][0])?);
    assert_eq!(machine["scenarios"][0]["testMappings"], json!([
        {"testMappingIndex": 0, "mapping": original["scenarios"][0]["testMappings"][0]},
        {"testMappingIndex": 1, "mapping": original["scenarios"][0]["testMappings"][1]}
    ]));
    assert_eq!(tree(temporary.path())?, before);
    let mut reordered = original.clone();
    let scenario = reordered["scenarios"][0].as_object_mut().ok_or("scenario object")?;
    let removed = scenario.remove("name").ok_or("scenario name")?;
    scenario.insert("name".to_string(), removed);
    save(temporary.path(), &reordered)?;
    let ordered = successful_json(request(temporary.path(), "show-coverage",
        json!({"featureName": "thing", "mappingJson": true})))?;
    assert_eq!(machine["scenarios"][0]["scenarioHash"], ordered["scenarios"][0]["scenarioHash"]);
    reordered["scenarios"][0]["testMappings"].as_array_mut().ok_or("mapping array")?.reverse();
    save(temporary.path(), &reordered)?;
    let reversed = successful_json(request(temporary.path(), "show-coverage",
        json!({"featureName": "thing", "mappingJson": true})))?;
    assert_ne!(machine["scenarios"][0]["scenarioHash"], reversed["scenarios"][0]["scenarioHash"]);
    reordered["scenarios"][0]["unknownScenario"]["newReceipt"] = json!(9);
    save(temporary.path(), &reordered)?;
    let unknown = successful_json(request(temporary.path(), "show-coverage",
        json!({"featureName": "thing", "mappingJson": true})))?;
    assert_ne!(unknown["scenarios"][0]["scenarioHash"], reversed["scenarios"][0]["scenarioHash"]);
    save(temporary.path(), &original)?;
    assert_eq!(request(temporary.path(), "show-coverage", legacy_args).data, legacy.data);
    assert_eq!(tree(temporary.path())?, before);
    let invalid = request(temporary.path(), "show-coverage", json!({"mappingJson": true}));
    assert!(!invalid.success, "machine view must require a selected feature");
    assert_eq!(tree(temporary.path())?, before);
    Ok(())
}

