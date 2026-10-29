use serde_json::json;

use crate::support::{json_output, native_json, Workspace, FEATURE, SIDECAR};
use crate::TestResult;

#[test]
fn scoped_generation_previews_and_applies_only_the_named_feature_through_shared_core() -> TestResult {
    // @step Given temporary projects with one selected feature addition and unrelated missing or drifted sidecars
    let cli = Workspace::new()?;
    let native = Workspace::new()?;
    for workspace in [&cli, &native] {
        workspace.write("spec/features/thing.feature", &format!("{FEATURE}\n  Scenario: Added\n    Given a thing exists\n    When the thing is used\n    Then the thing works\n"))?;
        workspace.write("spec/features/missing.feature", FEATURE)?;
        workspace.write("spec/features/drift.feature", "Feature: Drift\n  Scenario: New name\n    Given a different thing\n")?;
        workspace.write("spec/features/drift.feature.coverage", "{\"scenarios\":[{\"name\":\"Old name\",\"testMappings\":[{\"file\":\"original.rs\",\"lines\":\"1-2\"}]}],\"unrelated\":true}\n")?;
    }
    let before = cli.snapshot()?;
    let native_before = native.snapshot()?;
    let original = cli.sidecar()?;
    // @step When CLI and native dispatch preview then apply generation with the explicit feature name
    let preview = json_output(&cli.cli(&["generate-coverage", "--feature-name", "thing", "--dry-run"])?)?;
    let native_preview = native_json(native.dispatch("generate-coverage", json!({"featureName": "thing", "dryRun": true})))?;
    assert_eq!(cli.snapshot()?, before, "preview changed files or directory entries");
    assert_eq!(native.snapshot()?, native_before);
    let applied = json_output(&cli.cli(&["generate-coverage", "--feature-name", "thing"])?)?;
    let native_applied = native_json(native.dispatch("generate-coverage", json!({"featureName": "thing", "dryRun": false})))?;
    // @step Then JSON plans and complete results agree while preview writes nothing and unrelated bytes remain untouched
    assert_eq!(preview, native_preview);
    assert_eq!(applied, native_applied);
    assert_eq!(preview["success"], true);
    assert_eq!(preview["dryRun"], true);
    assert_eq!(applied["dryRun"], false);
    assert_eq!(preview["plan"], applied["plan"]);
    assert_eq!(preview["plan"]["status"], "update");
    assert_eq!(preview["plan"]["addedScenarios"], json!(["Added"]));
    assert_eq!(preview["plan"]["removedScenarios"], json!([]));
    assert_eq!(preview["plan"]["blockedReasons"], json!([]));
    assert_eq!(cli.sidecar()?, native.sidecar()?);
    let actual = cli.sidecar()?;
    assert_eq!(actual["scenarios"][0], original["scenarios"][0]);
    assert_eq!(actual["scenarios"][1], json!({"name": "Added", "testMappings": []}));
    assert_eq!(actual["scenarios"].as_array().ok_or("scenario list missing")?.len(), 2);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    let after = cli.snapshot()?;
    for (path, bytes) in before {
        if path != std::path::Path::new(SIDECAR) {
            assert_eq!(after.get(&path), Some(&bytes), "unrelated path changed: {}", path.display());
        }
    }
    assert!(!cli.path().join("spec/features/missing.feature.coverage").exists());
    Ok(())
}
