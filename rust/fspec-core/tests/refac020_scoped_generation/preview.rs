use crate::support::*;
use serde_json::json;

#[test]
fn dry_run_and_apply_report_the_same_actual_selected_plan() -> TestResult {
    // @step Given an unchanged selected-generation input with additions and empty removals
    let temporary = fixture()?;
    add_new_scenario(temporary.path())?;
    let mut original = load(temporary.path())?;
    original["scenarios"].as_array_mut().ok_or("scenarios")?
        .push(json!({"name": "Obsolete empty thing", "testMappings": []}));
    save(temporary.path(), &original)?;
    let before = tree(temporary.path())?;

    // @step When preview is followed by apply and a no-op apply
    let preview = successful_json(request(temporary.path(), "generate-coverage",
        json!({"featureName": "thing", "dryRun": true})))?;
    let after_preview = tree(temporary.path())?;
    let applied = successful_json(request(temporary.path(), "generate-coverage", json!({"featureName": "thing"})))?;
    let committed = tree(temporary.path())?;
    let no_op = successful_json(request(temporary.path(), "generate-coverage", json!({"featureName": "thing"})))?;

    // @step Then preview and apply report the same deltas and preview and no-op preserve bytes
    assert_eq!(preview["success"], true);
    assert_eq!(preview["dryRun"], true);
    assert_eq!(applied["dryRun"], false);
    assert_eq!(preview["plan"], applied["plan"]);
    assert_eq!(preview["plan"]["status"], "update");
    assert_eq!(preview["plan"]["addedScenarios"], json!(["New thing"]));
    assert_eq!(preview["plan"]["removedScenarios"], json!(["Obsolete empty thing"]));
    assert_eq!(preview["plan"]["blockedReasons"], json!([]));
    assert_eq!(after_preview, before);
    assert_eq!(no_op["plan"]["status"], "skip");
    assert_eq!(tree(temporary.path())?, committed);
    std::fs::remove_file(temporary.path().join(SIDECAR))?;
    let missing_before = tree(temporary.path())?;
    let create_preview = successful_json(request(temporary.path(), "generate-coverage",
        json!({"featureName": "thing", "dryRun": true})))?;
    assert_eq!(tree(temporary.path())?, missing_before);
    let create = successful_json(request(temporary.path(), "generate-coverage", json!({"featureName": "thing"})))?;
    assert_eq!(create_preview["plan"], create["plan"]);
    assert_eq!(create["plan"]["status"], "create");
    assert_eq!(load(temporary.path())?["scenarios"].as_array().ok_or("scenario array")?.len(), 2);
    Ok(())
}
