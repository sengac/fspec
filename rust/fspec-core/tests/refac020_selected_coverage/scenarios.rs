use crate::support::*;
use serde_json::{json, Value};

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
    let machine = successful_json(request(
        temporary.path(),
        "show-coverage",
        json!({"featureName": "thing", "mappingJson": true}),
    ))?;

    // @step Then row indices and the complete canonical scenario hash are exposed without writes
    assert_eq!(machine["fileName"], "thing.feature");
    assert_eq!(
        machine["scenarios"][0]["scenarioHash"],
        scenario_hash(&original["scenarios"][0])?
    );
    assert_eq!(
        machine["scenarios"][0]["testMappings"],
        json!([
            {"testMappingIndex": 0, "mapping": original["scenarios"][0]["testMappings"][0]},
            {"testMappingIndex": 1, "mapping": original["scenarios"][0]["testMappings"][1]}
        ])
    );
    assert_eq!(tree(temporary.path())?, before);
    let mut reordered = original.clone();
    let scenario = reordered["scenarios"][0]
        .as_object_mut()
        .ok_or("scenario object")?;
    let removed = scenario.remove("name").ok_or("scenario name")?;
    scenario.insert("name".to_string(), removed);
    save(temporary.path(), &reordered)?;
    let ordered = successful_json(request(
        temporary.path(),
        "show-coverage",
        json!({"featureName": "thing", "mappingJson": true}),
    ))?;
    assert_eq!(
        machine["scenarios"][0]["scenarioHash"],
        ordered["scenarios"][0]["scenarioHash"]
    );
    reordered["scenarios"][0]["testMappings"]
        .as_array_mut()
        .ok_or("mapping array")?
        .reverse();
    save(temporary.path(), &reordered)?;
    let reversed = successful_json(request(
        temporary.path(),
        "show-coverage",
        json!({"featureName": "thing", "mappingJson": true}),
    ))?;
    assert_ne!(
        machine["scenarios"][0]["scenarioHash"],
        reversed["scenarios"][0]["scenarioHash"]
    );
    reordered["scenarios"][0]["unknownScenario"]["newReceipt"] = json!(9);
    save(temporary.path(), &reordered)?;
    let unknown = successful_json(request(
        temporary.path(),
        "show-coverage",
        json!({"featureName": "thing", "mappingJson": true}),
    ))?;
    assert_ne!(
        unknown["scenarios"][0]["scenarioHash"],
        reversed["scenarios"][0]["scenarioHash"]
    );
    save(temporary.path(), &original)?;
    assert_eq!(
        request(temporary.path(), "show-coverage", legacy_args).data,
        legacy.data
    );
    assert_eq!(tree(temporary.path())?, before);
    let invalid = request(
        temporary.path(),
        "show-coverage",
        json!({"mappingJson": true}),
    );
    assert!(
        !invalid.success,
        "machine view must require a selected feature"
    );
    assert_eq!(tree(temporary.path())?, before);
    Ok(())
}

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
    expected[0]["testMappings"][1]["implMappings"]
        .as_array_mut()
        .ok_or("implementation array")?
        .push(json!({"file": "src/second.rs", "lines": [1, 2], "firstLine": "fn thing() {", "lastLine": "}"}));
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(
        actual["stats"]["unknownStats"],
        original["stats"]["unknownStats"]
    );
    assert_eq!(
        result["scenarioHash"],
        scenario_hash(&actual["scenarios"][0])?
    );
    assert_ne!(
        result["scenarioHash"],
        scenario_hash(&original["scenarios"][0])?
    );
    Ok(())
}

#[test]
fn unselected_mutation_retains_legacy_duplicate_selection_behavior() -> TestResult {
    // @step Given two same-file mapping rows with distinct retained lineage
    let temporary = fixture()?;
    let original = load(temporary.path())?;

    // @step When implementation linking and unlinking omit the new selector
    let linked = request(
        temporary.path(),
        "link-coverage",
        json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "implFile": "src/second.rs", "implLines": "1,2"}),
    );
    assert!(linked.success, "legacy link failed: {linked:?}");
    let intermediate = load(temporary.path())?;
    let unlinked = request(
        temporary.path(),
        "unlink-coverage",
        json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "implFile": "src/second.rs"}),
    );

    // @step Then the legacy first matching row behavior is unchanged
    assert!(unlinked.success, "legacy unlink failed: {unlinked:?}");
    assert_eq!(
        intermediate["scenarios"][0]["testMappings"][0]["implMappings"][1],
        json!({"file": "src/second.rs", "lines": [1, 2], "firstLine": "fn thing() {", "lastLine": "}"})
    );
    assert_eq!(
        intermediate["scenarios"][0]["testMappings"][1],
        original["scenarios"][0]["testMappings"][1]
    );
    assert_eq!(load(temporary.path())?["scenarios"], original["scenarios"]);
    Ok(())
}

#[test]
fn selected_obsolete_duplicate_is_removed_without_losing_other_occurrences() -> TestResult {
    // @step Given identical duplicate rows and retained implementation arrays
    let temporary = fixture()?;
    let mut original = load(temporary.path())?;
    let mut second = mapping("obsolete");
    second["implMappings"]
        .as_array_mut()
        .ok_or("implementation array")?
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
    expected[1]["testMappings"]
        .as_array_mut()
        .ok_or("mapping array")?
        .remove(1);

    // @step When one obsolete occurrence is explicitly unlinked with its fresh hash
    let removed = successful_json(request(
        temporary.path(),
        "unlink-coverage",
        json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "testMappingIndex": 1, "expectedScenarioHash": scenario_hash(&original["scenarios"][1])?}),
    ))?;

    // @step Then every other mapping occurrence and unknown member is retained
    let actual = load(temporary.path())?;
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(removed["scenarioHash"], scenario_hash(&expected[1])?);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(
        actual["stats"]["unknownStats"],
        original["stats"]["unknownStats"]
    );
    let next_hash = scenario_hash(&expected[1])?;
    expected[1]["testMappings"][1]["implMappings"]
        .as_array_mut()
        .ok_or("implementation array")?
        .remove(0);
    let implementation_removed = request(
        temporary.path(),
        "unlink-coverage",
        json!({
        "featureName": "thing", "scenario": SCENARIO, "testFile": TEST_FILE,
        "testMappingIndex": 1, "expectedScenarioHash": next_hash,
        "implFile": "src/first.rs"}),
    );
    assert!(
        implementation_removed.success,
        "indexed implementation unlink: {implementation_removed:?}"
    );
    let final_coverage = load(temporary.path())?;
    assert_eq!(final_coverage["scenarios"], expected);
    assert_eq!(final_coverage["unknownTop"], original["unknownTop"]);
    assert_eq!(
        final_coverage["stats"]["unknownStats"],
        original["stats"]["unknownStats"]
    );
    Ok(())
}

#[test]
fn invalid_or_stale_selector_leaves_exact_sidecar_bytes_unchanged() -> TestResult {
    // @step Given a valid duplicate-row coverage sidecar
    let temporary = fixture()?;
    let valid = indexed_link(temporary.path(), 1, "src/second.rs")?;
    let original = std::fs::read(temporary.path().join(SIDECAR))?;
    let cases = [
        ("stale", "expectedScenarioHash", json!("0".repeat(64))),
        ("malformed", "expectedScenarioHash", json!("not-a-hash")),
        (
            "wrong length",
            "expectedScenarioHash",
            json!("a".repeat(63)),
        ),
        ("wrong file", "testFile", json!("src/first.rs")),
        ("out of bounds", "testMappingIndex", json!(50)),
        ("negative", "testMappingIndex", json!(-1)),
        ("fractional", "testMappingIndex", json!(1.5)),
    ];
    let mut requests: Vec<(&str, Value)> = cases
        .into_iter()
        .map(|(label, key, value)| {
            let mut args = valid.clone();
            args[key] = value;
            (label, args)
        })
        .collect();
    for key in ["expectedScenarioHash", "testMappingIndex"] {
        let mut args = valid.clone();
        args.as_object_mut().ok_or("argument object")?.remove(key);
        requests.push((key, args));
    }
    let mut append = valid.clone();
    append["testLines"] = json!("1-7");
    requests.push(("indexed append", append));
    let mut selected_without_impl = valid.clone();
    selected_without_impl
        .as_object_mut()
        .ok_or("arguments")?
        .remove("implFile");
    requests.push(("invalid indexed mode", selected_without_impl));

    // @step When stale malformed mismatched unpaired or incompatible selectors are submitted
    // @step Then every rejected request leaves exact sidecar bytes unchanged
    for (label, args) in requests {
        let linked = request(temporary.path(), "link-coverage", args.clone());
        assert!(!linked.success, "{label} unexpectedly linked: {linked:?}");
        assert_eq!(
            std::fs::read(temporary.path().join(SIDECAR))?,
            original,
            "{label}"
        );
        if !matches!(label, "indexed append" | "invalid indexed mode") {
            let mut unlink_args = args;
            unlink_args
                .as_object_mut()
                .ok_or("argument object")?
                .remove("implFile");
            let unlinked = request(temporary.path(), "unlink-coverage", unlink_args);
            assert!(
                !unlinked.success,
                "{label} unexpectedly unlinked: {unlinked:?}"
            );
            assert_eq!(
                std::fs::read(temporary.path().join(SIDECAR))?,
                original,
                "{label}"
            );
        }
    }
    let mut all = valid;
    all["all"] = json!(true);
    let result = request(temporary.path(), "unlink-coverage", all);
    assert!(
        !result.success,
        "index plus all must not select a deletion scope"
    );
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    Ok(())
}

#[test]
fn selected_link_retains_file_and_story_step_validation() -> TestResult {
    // @step Given a fresh selector for a story coverage row
    let temporary = fixture()?;
    let original = std::fs::read(temporary.path().join(SIDECAR))?;

    // @step When missing files or tests without matching step comments are linked
    let missing = indexed_link(temporary.path(), 1, "src/missing.rs")?;
    let missing_result = request(temporary.path(), "link-coverage", missing);
    assert!(
        !missing_result.success,
        "missing implementation unexpectedly linked"
    );
    let missing_error = missing_result.error.ok_or("missing-file diagnostic")?;
    assert!(missing_error.contains("src/missing.rs"), "{missing_error}");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    // BUG-452: long enough for the mapped 1-7 range, but without the steps.
    write(
        temporary.path(),
        TEST_FILE,
        "#[test]\nfn unrelated() {}\n\n\n\n\n\n",
    )?;
    let mismatch = indexed_link(temporary.path(), 1, "src/second.rs")?;
    let mismatch_result = request(temporary.path(), "link-coverage", mismatch.clone());

    // @step Then normal validation rejects the request without writes
    assert!(
        !mismatch_result.success,
        "missing story steps unexpectedly linked"
    );
    let diagnostic = mismatch_result.error.ok_or("step mismatch diagnostic")?;
    assert!(diagnostic.to_lowercase().contains("step"), "{diagnostic}");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    let mut skipped = mismatch;
    skipped["skipStepValidation"] = json!(true);
    let skipped_result = request(temporary.path(), "link-coverage", skipped);
    assert!(
        !skipped_result.success,
        "index cannot bypass story validation"
    );
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    Ok(())
}

#[test]
fn ambiguous_same_name_scenarios_reject_indexed_mutations_without_writes() -> TestResult {
    // @step Given two raw coverage scenarios with the same name and distinct metadata
    let temporary = fixture()?;
    let mut ambiguous = load(temporary.path())?;
    let mut duplicate = ambiguous["scenarios"][0].clone();
    duplicate["unknownScenario"]["duplicateReceipt"] = json!([7, 3, 7]);
    duplicate["testMappings"][0]["lineage"] = json!("second-same-name-scenario");
    ambiguous["scenarios"]
        .as_array_mut()
        .ok_or("scenario array")?
        .push(duplicate);
    save(temporary.path(), &ambiguous)?;
    let before = tree(temporary.path())?;
    let args = indexed_link(temporary.path(), 1, "src/second.rs")?;

    // @step When indexed linking and unlinking select that ambiguous name
    // @step Then neither scenario is chosen and exact sidecar bytes remain unchanged
    for command in ["link-coverage", "unlink-coverage"] {
        let mut selected_args = args.clone();
        if command == "unlink-coverage" {
            selected_args
                .as_object_mut()
                .ok_or("argument object")?
                .remove("implFile");
            selected_args
                .as_object_mut()
                .ok_or("argument object")?
                .remove("implLines");
        }
        let result = request(temporary.path(), command, selected_args);
        assert!(
            !result.success,
            "{command} arbitrarily selected a same-name scenario: {result:?}"
        );
        assert_eq!(load(temporary.path())?, ambiguous);
        let mut after = tree(temporary.path())?;
        after.remove(SPEC_LOCK);
        assert_eq!(after, before, "{command} rewrote ambiguous scenario bytes");
    }
    Ok(())
}
