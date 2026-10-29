use crate::support::*;
use serde_json::json;

#[test]
fn selected_generation_does_not_touch_unrelated_sidecars_or_missing_files() -> TestResult {
    // @step Given one changed feature and unrelated drifted or missing sidecars
    let temporary = fixture()?;
    add_new_scenario(temporary.path())?;
    write(temporary.path(), "spec/features/unrelated.feature", "Feature: Other\n  Scenario: Renamed\n    Given something exists\n")?;
    let unrelated = "spec/features/unrelated.feature.coverage";
    let retained_bytes = b"{\n \"scenarios\": [{\"name\":\"Old mapped name\",\"testMappings\":[{\"file\":\"old.rs\",\"lines\":\"1-2\",\"implMappings\":[]}]}],\"unknown\": [2,1,2]\n}\n";
    write(temporary.path(), unrelated, retained_bytes)?;
    write(temporary.path(), "spec/features/missing.feature", "Feature: Missing\n  Scenario: Still absent\n    Given no sidecar exists\n")?;
    let before = tree(temporary.path())?;

    // @step When only the changed feature is generated
    let result = request(temporary.path(), "generate-coverage", json!({"featureName": "thing"}));

    // @step Then only its sidecar gains the newly declared scenario
    assert!(result.success, "selected generation: {result:?}");
    assert_eq!(std::fs::read(temporary.path().join(unrelated))?, retained_bytes);
    assert!(!temporary.path().join("spec/features/missing.feature.coverage").exists());
    let actual = load(temporary.path())?;
    assert_eq!(actual["scenarios"][1], json!({"name": "New thing", "testMappings": []}));
    let mut after = tree(temporary.path())?;
    let mut expected_inventory = before;
    expected_inventory.remove(SIDECAR);
    expected_inventory.remove(&format!("{SIDECAR}.lock"));
    after.remove(SIDECAR);
    after.remove(&format!("{SIDECAR}.lock"));
    assert_eq!(after, expected_inventory, "only the selected sidecar and its one known persistent lock may change or appear");
    Ok(())
}

#[test]
fn invalid_ambiguous_or_missing_feature_never_falls_back_to_global_generation() -> TestResult {
    // @step Given unrelated features and ambiguous nested feature basenames
    let temporary = fixture()?;
    for directory in ["first", "second"] {
        write(temporary.path(), &format!("spec/features/{directory}/duplicate.feature"), FEATURE)?;
    }
    write(temporary.path(), "outside.feature", FEATURE)?;
    let before = tree(temporary.path())?;

    // @step When invalid ambiguous or absent selected names are requested
    // @step Then generation fails without global fallback or unrelated writes
    for name in ["not-present", "duplicate", "", "../outside", "../../outside.feature", "/outside.feature"] {
        for dry_run in [true, false] {
            let result = request(temporary.path(), "generate-coverage", json!({"featureName": name, "dryRun": dry_run}));
            assert!(!result.success, "{name:?} unexpectedly generated: {result:?}");
            assert_eq!(tree(temporary.path())?, before, "invalid selected name {name:?}");
        }
    }
    let nested = successful_json(request(temporary.path(), "generate-coverage",
        json!({"featureName": "first/duplicate.feature"})))?;
    assert_eq!(nested["plan"]["status"], "create");
    assert!(temporary.path().join("spec/features/first/duplicate.feature.coverage").is_file());
    assert!(!temporary.path().join("spec/features/second/duplicate.feature.coverage").exists());
    let mut nested_inventory = tree(temporary.path())?;
    nested_inventory.remove("spec/features/first/duplicate.feature.coverage");
    nested_inventory.remove("spec/features/first/duplicate.feature.coverage.lock");
    assert_eq!(nested_inventory, before, "exact nested selection must not create any other file or directory");
    Ok(())
}
