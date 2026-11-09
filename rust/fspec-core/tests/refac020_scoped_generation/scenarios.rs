use crate::support::*;
use serde_json::json;

#[test]
fn selected_generation_does_not_touch_unrelated_sidecars_or_missing_files() -> TestResult {
    // @step Given one changed feature and unrelated drifted or missing sidecars
    let temporary = fixture()?;
    add_new_scenario(temporary.path())?;
    write(
        temporary.path(),
        "spec/features/unrelated.feature",
        "Feature: Other\n  Scenario: Renamed\n    Given something exists\n",
    )?;
    let unrelated = "spec/features/unrelated.feature.coverage";
    let retained_bytes = b"{\n \"scenarios\": [{\"name\":\"Old mapped name\",\"testMappings\":[{\"file\":\"old.rs\",\"lines\":\"1-2\",\"implMappings\":[]}]}],\"unknown\": [2,1,2]\n}\n";
    write(temporary.path(), unrelated, retained_bytes)?;
    write(
        temporary.path(),
        "spec/features/missing.feature",
        "Feature: Missing\n  Scenario: Still absent\n    Given no sidecar exists\n",
    )?;
    let before = tree(temporary.path())?;

    // @step When only the changed feature is generated
    let result = request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing"}),
    );

    // @step Then only its sidecar gains the newly declared scenario
    assert!(result.success, "selected generation: {result:?}");
    assert_eq!(
        std::fs::read(temporary.path().join(unrelated))?,
        retained_bytes
    );
    assert!(!temporary
        .path()
        .join("spec/features/missing.feature.coverage")
        .exists());
    let actual = load(temporary.path())?;
    assert_eq!(
        actual["scenarios"][1],
        json!({"name": "New thing", "testMappings": []})
    );
    let mut after = tree(temporary.path())?;
    let mut expected_inventory = before;
    expected_inventory.remove(SIDECAR);
    expected_inventory.remove(SPEC_LOCK);
    after.remove(SIDECAR);
    after.remove(SPEC_LOCK);
    assert_eq!(
        after, expected_inventory,
        "only the selected sidecar and the spec write lock may change or appear"
    );
    Ok(())
}

#[test]
fn invalid_ambiguous_or_missing_feature_never_falls_back_to_global_generation() -> TestResult {
    // @step Given unrelated features and ambiguous nested feature basenames
    let temporary = fixture()?;
    for directory in ["first", "second"] {
        write(
            temporary.path(),
            &format!("spec/features/{directory}/duplicate.feature"),
            FEATURE,
        )?;
    }
    write(temporary.path(), "outside.feature", FEATURE)?;
    let before = tree(temporary.path())?;

    // @step When invalid ambiguous or absent selected names are requested
    // @step Then generation fails without global fallback or unrelated writes
    for name in [
        "not-present",
        "duplicate",
        "",
        "../outside",
        "../../outside.feature",
        "/outside.feature",
    ] {
        for dry_run in [true, false] {
            let result = request(
                temporary.path(),
                "generate-coverage",
                json!({"featureName": name, "dryRun": dry_run}),
            );
            assert!(
                !result.success,
                "{name:?} unexpectedly generated: {result:?}"
            );
            assert_eq!(
                tree(temporary.path())?,
                before,
                "invalid selected name {name:?}"
            );
        }
    }
    let nested = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "first/duplicate.feature"}),
    ))?;
    assert_eq!(nested["plan"]["status"], "create");
    assert!(temporary
        .path()
        .join("spec/features/first/duplicate.feature.coverage")
        .is_file());
    assert!(!temporary
        .path()
        .join("spec/features/second/duplicate.feature.coverage")
        .exists());
    let mut nested_inventory = tree(temporary.path())?;
    nested_inventory.remove("spec/features/first/duplicate.feature.coverage");
    nested_inventory.remove(SPEC_LOCK);
    assert_eq!(
        nested_inventory, before,
        "exact nested selection must not create any other file or directory"
    );
    Ok(())
}

#[test]
fn selected_generation_preserves_unknown_fields_order_and_duplicate_lineage() -> TestResult {
    // @step Given selected metadata with unknown fields ordered duplicates and outlines
    let temporary = fixture()?;
    let feature = format!("{FEATURE}\n  Scenario Outline: Repeated thing\n    Given item <item> exists\n    Examples:\n      | item |\n      | one  |\n      | two  |\n\n  Scenario: New thing\n    Given another thing exists\n");
    let mut original = load(temporary.path())?;
    let first = original["scenarios"][0].clone();
    let mut outline = first.clone();
    outline["name"] = json!("Repeated thing");
    let duplicate = outline["testMappings"][1].clone();
    outline["testMappings"]
        .as_array_mut()
        .ok_or("mapping array")?
        .push(duplicate);
    let repeated_impl = outline["testMappings"][0]["implMappings"][0].clone();
    outline["testMappings"][0]["implMappings"]
        .as_array_mut()
        .ok_or("implementation array")?
        .push(repeated_impl);
    original["scenarios"] = json!([outline, first]);
    original["stats"]["totalScenarios"] = json!(2);
    save(temporary.path(), &original)?;
    write(temporary.path(), "spec/features/thing.feature", feature)?;

    // @step When the selected feature gains newly declared scenarios
    let response = request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing.feature"}),
    );
    assert!(response.success, "selected generation failed: {response:?}");

    // @step Then retained metadata remains lossless and new scenario names follow feature order
    let actual = load(temporary.path())?;
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(
        actual["stats"]["unknownStats"],
        original["stats"]["unknownStats"]
    );
    assert_eq!(actual["scenarios"][0], original["scenarios"][0]);
    assert_eq!(actual["scenarios"][1], original["scenarios"][1]);
    assert_eq!(
        actual["scenarios"]
            .as_array()
            .ok_or("scenario array")?
            .len(),
        3
    );
    assert_eq!(
        actual["scenarios"][2],
        json!({"name": "New thing", "testMappings": []})
    );
    // BUG-563: Rule blocks are rejected, so no Rule scenario appears here.
    assert_eq!(actual["stats"]["totalScenarios"], 3);
    let response = successful_json(response)?;
    assert_eq!(response["plan"]["addedScenarios"], json!(["New thing"]));
    assert_eq!(response["plan"]["removedScenarios"], json!([]));
    Ok(())
}

#[test]
fn mapped_scenario_removal_blocks_all_selected_changes() -> TestResult {
    // @step Given a selected feature with new scenarios and complete or incomplete obsolete mapped lineage
    // @step When generation is previewed and applied
    // @step Then the entire plan is blocked without partial additions or lineage removal
    for lineage in [json!([mapping("first"), mapping("second")]), json!([{}])] {
        let temporary = fixture()?;
        let mut original = load(temporary.path())?;
        let mut obsolete = original["scenarios"][0].clone();
        obsolete["name"] = json!("Obsolete mapped thing");
        obsolete["testMappings"] = lineage.clone();
        original["scenarios"]
            .as_array_mut()
            .ok_or("scenario array")?
            .push(obsolete);
        save(temporary.path(), &original)?;
        add_new_scenario(temporary.path())?;
        let before = tree(temporary.path())?;
        let preview = request(
            temporary.path(),
            "generate-coverage",
            json!({"featureName": "thing", "dryRun": true}),
        );
        assert_eq!(
            tree(temporary.path())?,
            before,
            "preview changed lineage {lineage}"
        );
        let applied = request(
            temporary.path(),
            "generate-coverage",
            json!({"featureName": "thing"}),
        );
        assert!(
            !applied.success,
            "complete or incomplete lineage must block: {lineage}: {applied:?}"
        );
        assert_eq!(load(temporary.path())?, original);
        let mut after = tree(temporary.path())?;
        after.remove(SPEC_LOCK);
        assert_eq!(after, before, "apply changed lineage {lineage}");
        let preview = successful_json(preview)?;
        assert_eq!(preview["success"], false);
        assert_eq!(preview["plan"]["status"], "blocked");
        assert_eq!(preview["plan"]["addedScenarios"], json!(["New thing"]));
        assert_eq!(
            preview["plan"]["removedScenarios"],
            json!(["Obsolete mapped thing"])
        );
        assert!(!preview["plan"]["blockedReasons"]
            .as_array()
            .ok_or("blocked reasons")?
            .is_empty());
    }
    Ok(())
}

#[test]
fn invalid_or_unreadable_existing_metadata_is_never_recreated() -> TestResult {
    // @step Given invalid malformed or unreadable selected metadata
    // @step When generation is previewed and applied
    // @step Then the blocked metadata is never recreated and unrelated bytes remain unchanged
    for invalid in [
        "{bad JSON",
        "[]",
        "{\"scenarios\":{}}",
        "{\"scenarios\":[true]}",
        "{\"scenarios\":[{\"name\":\"Thing does the thing\",\"testMappings\":{}}]}",
        "directory",
    ] {
        let temporary = fixture()?;
        add_new_scenario(temporary.path())?;
        if invalid == "directory" {
            std::fs::remove_file(temporary.path().join(SIDECAR))?;
            std::fs::create_dir(temporary.path().join(SIDECAR))?;
            write(
                temporary.path(),
                &format!("{SIDECAR}/lineage-receipt"),
                "preserve me",
            )?;
        } else {
            write(temporary.path(), SIDECAR, invalid)?;
        }
        let before = tree(temporary.path())?;
        let preview = request(
            temporary.path(),
            "generate-coverage",
            json!({"featureName": "thing", "dryRun": true}),
        );
        assert_eq!(tree(temporary.path())?, before, "preview changed {invalid}");
        let result = request(
            temporary.path(),
            "generate-coverage",
            json!({"featureName": "thing"}),
        );
        assert!(!result.success, "must not recreate {invalid}: {result:?}");
        let mut after = tree(temporary.path())?;
        after.remove(SPEC_LOCK);
        assert_eq!(after, before, "apply changed {invalid}");
        let preview = successful_json(preview)?;
        assert_eq!(preview["plan"]["status"], "blocked", "{invalid}");
        assert_eq!(preview["success"], false, "{invalid}");
        assert!(!preview["plan"]["blockedReasons"]
            .as_array()
            .ok_or("blocked reasons")?
            .is_empty());
    }
    Ok(())
}

#[test]
fn dry_run_and_apply_report_the_same_actual_selected_plan() -> TestResult {
    // @step Given an unchanged selected-generation input with additions and empty removals
    let temporary = fixture()?;
    add_new_scenario(temporary.path())?;
    let mut original = load(temporary.path())?;
    original["scenarios"]
        .as_array_mut()
        .ok_or("scenarios")?
        .push(json!({"name": "Obsolete empty thing", "testMappings": []}));
    save(temporary.path(), &original)?;
    let before = tree(temporary.path())?;

    // @step When preview is followed by apply and a no-op apply
    let preview = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing", "dryRun": true}),
    ))?;
    let after_preview = tree(temporary.path())?;
    let applied = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing"}),
    ))?;
    let committed = tree(temporary.path())?;
    let no_op = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing"}),
    ))?;

    // @step Then preview and apply report the same deltas and preview and no-op preserve bytes
    assert_eq!(preview["success"], true);
    assert_eq!(preview["dryRun"], true);
    assert_eq!(applied["dryRun"], false);
    assert_eq!(preview["plan"], applied["plan"]);
    assert_eq!(preview["plan"]["status"], "update");
    assert_eq!(preview["plan"]["addedScenarios"], json!(["New thing"]));
    assert_eq!(
        preview["plan"]["removedScenarios"],
        json!(["Obsolete empty thing"])
    );
    assert_eq!(preview["plan"]["blockedReasons"], json!([]));
    assert_eq!(after_preview, before);
    assert_eq!(no_op["plan"]["status"], "skip");
    assert_eq!(tree(temporary.path())?, committed);
    std::fs::remove_file(temporary.path().join(SIDECAR))?;
    let missing_before = tree(temporary.path())?;
    let create_preview = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing", "dryRun": true}),
    ))?;
    assert_eq!(tree(temporary.path())?, missing_before);
    let create = successful_json(request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing"}),
    ))?;
    assert_eq!(create_preview["plan"], create["plan"]);
    assert_eq!(create["plan"]["status"], "create");
    assert_eq!(
        load(temporary.path())?["scenarios"]
            .as_array()
            .ok_or("scenario array")?
            .len(),
        2
    );
    Ok(())
}

#[test]
fn ambiguous_same_name_scenarios_block_selected_generation_without_writes() -> TestResult {
    // @step Given selected metadata with duplicate scenario names and a newly declared scenario
    let temporary = fixture()?;
    let mut original = load(temporary.path())?;
    let mut duplicate = original["scenarios"][0].clone();
    duplicate["unknownScenario"]["duplicateReceipt"] = json!([9, 6, 9]);
    duplicate["testMappings"][0]["lineage"] = json!("second-same-name-scenario");
    original["scenarios"]
        .as_array_mut()
        .ok_or("scenario array")?
        .push(duplicate);
    save(temporary.path(), &original)?;
    add_new_scenario(temporary.path())?;
    let before = tree(temporary.path())?;

    // @step When selected generation is previewed and applied to the ambiguous metadata
    let preview = request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing", "dryRun": true}),
    );
    let after_preview = tree(temporary.path())?;
    let applied = request(
        temporary.path(),
        "generate-coverage",
        json!({"featureName": "thing"}),
    );

    // @step Then both operations block without merging duplicate scenarios or adding partial metadata
    assert_eq!(after_preview, before);
    assert!(
        !applied.success,
        "generation merged or chose an ambiguous scenario: {applied:?}"
    );
    assert_eq!(load(temporary.path())?, original);
    let mut after = tree(temporary.path())?;
    after.remove(SPEC_LOCK);
    assert_eq!(after, before);
    let preview = successful_json(preview)?;
    assert_eq!(preview["success"], false);
    assert_eq!(preview["plan"]["status"], "blocked");
    assert!(!preview["plan"]["blockedReasons"]
        .as_array()
        .ok_or("blocked reasons")?
        .is_empty());
    Ok(())
}
