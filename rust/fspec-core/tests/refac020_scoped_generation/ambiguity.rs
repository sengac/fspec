use crate::support::*;
use serde_json::json;

#[test]
fn ambiguous_same_name_scenarios_block_selected_generation_without_writes() -> TestResult {
    // @step Given selected metadata with duplicate scenario names and a newly declared scenario
    let temporary = fixture()?;
    let mut original = load(temporary.path())?;
    let mut duplicate = original["scenarios"][0].clone();
    duplicate["unknownScenario"]["duplicateReceipt"] = json!([9, 6, 9]);
    duplicate["testMappings"][0]["lineage"] = json!("second-same-name-scenario");
    original["scenarios"].as_array_mut().ok_or("scenario array")?.push(duplicate);
    save(temporary.path(), &original)?;
    add_new_scenario(temporary.path())?;
    let before = tree(temporary.path())?;

    // @step When selected generation is previewed and applied to the ambiguous metadata
    let preview = request(temporary.path(), "generate-coverage",
        json!({"featureName": "thing", "dryRun": true}));
    let after_preview = tree(temporary.path())?;
    let applied = request(temporary.path(), "generate-coverage", json!({"featureName": "thing"}));

    // @step Then both operations block without merging duplicate scenarios or adding partial metadata
    assert_eq!(after_preview, before);
    assert!(!applied.success, "generation merged or chose an ambiguous scenario: {applied:?}");
    assert_eq!(load(temporary.path())?, original);
    let mut after = tree(temporary.path())?;
    after.remove(&format!("{SIDECAR}.lock"));
    assert_eq!(after, before);
    let preview = successful_json(preview)?;
    assert_eq!(preview["success"], false);
    assert_eq!(preview["plan"]["status"], "blocked");
    assert!(!preview["plan"]["blockedReasons"].as_array().ok_or("blocked reasons")?.is_empty());
    Ok(())
}
