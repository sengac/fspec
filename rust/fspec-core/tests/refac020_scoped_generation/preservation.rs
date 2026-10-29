use crate::support::*;
use serde_json::json;

#[test]
fn selected_generation_preserves_unknown_fields_order_and_duplicate_lineage() -> TestResult {
    // @step Given selected metadata with unknown fields ordered duplicates outlines and rule scenarios
    let temporary = fixture()?;
    let feature = format!("{FEATURE}\n  Scenario Outline: Repeated thing\n    Given item <item> exists\n    Examples:\n      | item |\n      | one  |\n      | two  |\n\n  Scenario: New thing\n    Given another thing exists\n\n  Rule: Related things\n    Scenario: Rule thing\n      Given a rule thing exists\n");
    let mut original = load(temporary.path())?;
    let first = original["scenarios"][0].clone();
    let mut outline = first.clone();
    outline["name"] = json!("Repeated thing");
    let duplicate = outline["testMappings"][1].clone();
    outline["testMappings"].as_array_mut().ok_or("mapping array")?.push(duplicate);
    let repeated_impl = outline["testMappings"][0]["implMappings"][0].clone();
    outline["testMappings"][0]["implMappings"].as_array_mut().ok_or("implementation array")?.push(repeated_impl);
    original["scenarios"] = json!([outline, first]);
    original["stats"]["totalScenarios"] = json!(2);
    save(temporary.path(), &original)?;
    write(temporary.path(), "spec/features/thing.feature", feature)?;

    // @step When the selected feature gains newly declared scenarios
    let response = request(temporary.path(), "generate-coverage", json!({"featureName": "thing.feature"}));
    assert!(response.success, "selected generation failed: {response:?}");

    // @step Then retained metadata remains lossless and new scenario names follow feature order
    let actual = load(temporary.path())?;
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    assert_eq!(actual["scenarios"][0], original["scenarios"][0]);
    assert_eq!(actual["scenarios"][1], original["scenarios"][1]);
    assert_eq!(actual["scenarios"].as_array().ok_or("scenario array")?.len(), 4);
    assert_eq!(actual["scenarios"][2], json!({"name": "New thing", "testMappings": []}));
    assert_eq!(actual["scenarios"][3], json!({"name": "Rule thing", "testMappings": []}));
    assert_eq!(actual["stats"]["totalScenarios"], 4);
    let response = successful_json(response)?;
    assert_eq!(response["plan"]["addedScenarios"], json!(["New thing", "Rule thing"]));
    assert_eq!(response["plan"]["removedScenarios"], json!([]));
    Ok(())
}
