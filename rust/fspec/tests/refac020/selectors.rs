use serde_json::json;

use crate::support::{json_output, native_json, success, Workspace, SCENARIO, SIDECAR};
use crate::TestResult;

#[test]
fn machine_mapping_readback_is_explicit_and_leaves_legacy_json_unchanged() -> TestResult {
    // @step Given a real temporary feature with duplicate mapping rows and unknown metadata
    let workspace = Workspace::new()?;
    let before = workspace.snapshot()?;
    let native_legacy = workspace.dispatch("show-coverage", json!({"featureName": "thing", "format": "json"}));
    assert!(native_legacy.success, "{:?}", native_legacy.error);
    // @step When show coverage is invoked with mapping JSON and separately with legacy format JSON
    let readback = json_output(&workspace.cli(&["show-coverage", "thing", "--mapping-json"])?)?;
    let legacy = workspace.cli(&["show-coverage", "thing", "--format", "json"])?;
    success(&legacy);
    // @step Then explicit readback supplies ordered indices and hashes while legacy output and sidecar bytes stay unchanged
    assert_eq!(legacy.stdout, format!("{}\n", native_legacy.data).as_bytes());
    assert_eq!(readback, native_json(workspace.dispatch("show-coverage", json!({"featureName": "thing", "mappingJson": true})))?);
    assert_eq!(readback["fileName"], "thing.feature");
    assert_eq!(readback["scenarios"][0]["name"], SCENARIO);
    assert_eq!(readback["scenarios"][0]["scenarioHash"], workspace.hash()?);
    let rows = workspace.sidecar()?["scenarios"][0]["testMappings"].as_array().ok_or("fixture rows missing")?.clone();
    let expected = rows.into_iter().enumerate().map(|(index, mapping)| json!({"testMappingIndex": index, "mapping": mapping})).collect::<Vec<_>>();
    assert_eq!(readback["scenarios"][0]["testMappings"], json!(expected));
    for format in ["json", "text"] {
        assert_eq!(json_output(&workspace.cli(&["show-coverage", "thing", "--mapping-json", "--format", format])?)?, readback);
    }
    let missing_feature = workspace.cli(&["show-coverage", "--mapping-json"])?;
    assert!(!missing_feature.status.success(), "mapping readback must not fall back to project-wide mode");
    assert!(missing_feature.stdout.is_empty());
    assert_eq!(workspace.snapshot()?, before);
    Ok(())
}

#[test]
fn selected_linking_reaches_the_same_shared_core_mutation_as_native_dispatch() -> TestResult {
    // @step Given identical temporary projects with validated test steps and duplicate coverage rows
    let cli = Workspace::new()?;
    let native = Workspace::new()?;
    let original = cli.sidecar()?;
    let repeated = original["scenarios"][0]["testMappings"][1]["implMappings"].as_array()
        .ok_or("fixture implementations missing")?;
    assert_eq!(repeated.len(), 2);
    assert_eq!(repeated[0], repeated[1], "fixture must retain repeated complete implementation objects");
    let readback = json_output(&cli.cli(&["show-coverage", "thing", "--mapping-json"])?)?;
    let hash = readback["scenarios"][0]["scenarioHash"].as_str().ok_or("readback hash missing")?;
    assert_eq!(hash, cli.hash()?);
    // @step When CLI and native dispatch add an implementation to the selected second row with its current hash
    let output = cli.cli(&["link-coverage", "thing", "--scenario", SCENARIO, "--test-file", "tests/thing.rs",
        "--impl-file", "src/second.rs", "--impl-lines", "1-3", "--test-mapping-index", "1", "--expected-scenario-hash", hash])?;
    success(&output);
    let result = native_json(native.dispatch("link-coverage", json!({"featureName": "thing", "scenario": SCENARIO,
        "testFile": "tests/thing.rs", "implFile": "src/second.rs", "implLines": "1-3",
        "testMappingIndex": 1, "expectedScenarioHash": hash})))?;
    // @step Then complete sidecar results agree and the first row and unrelated metadata retain their lineage
    let actual = cli.sidecar()?;
    assert_eq!(actual, native.sidecar()?);
    let mut expected = original["scenarios"][0].clone();
    expected["testMappings"][1]["implMappings"].as_array_mut().ok_or("fixture implementations missing")?
        .push(json!({"file": "src/second.rs", "lines": [1, 2, 3]}));
    assert_eq!(actual["scenarios"][0], expected);
    assert_eq!(actual["scenarios"][0]["testMappings"][1]["implMappings"].as_array()
        .ok_or("selected implementations missing")?.len(), 3);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    assert_eq!(result["scenarioHash"], cli.hash()?);
    assert_eq!(String::from_utf8(output.stdout)?, format!("{}\n", result["message"].as_str().ok_or("mutation message missing")?));
    Ok(())
}

#[test]
fn selected_unlinking_removes_one_occurrence_through_the_shared_core() -> TestResult {
    // @step Given identical temporary sidecars containing duplicate mapping occurrences
    let cli = Workspace::new()?;
    let native = Workspace::new()?;
    let original = cli.sidecar()?;
    let hash = cli.hash()?;
    // @step When CLI and native dispatch unlink exactly the indexed row with its current hash
    let output = cli.cli(&["unlink-coverage", "thing", "--scenario", SCENARIO, "--test-file", "tests/thing.rs",
        "--test-mapping-index", "1", "--expected-scenario-hash", &hash])?;
    success(&output);
    let result = native_json(native.dispatch("unlink-coverage", json!({"featureName": "thing", "scenario": SCENARIO,
        "testFile": "tests/thing.rs", "testMappingIndex": 1, "expectedScenarioHash": hash})))?;
    // @step Then complete sidecars agree and all retained mapping multiplicity and unknown metadata remain intact
    let actual = cli.sidecar()?;
    let mut expected = original["scenarios"][0].clone();
    expected["testMappings"].as_array_mut().ok_or("fixture rows missing")?.remove(1);
    assert_eq!(actual, native.sidecar()?);
    assert_eq!(actual["scenarios"][0], expected);
    assert_eq!(actual["scenarios"][0]["testMappings"][0], actual["scenarios"][0]["testMappings"][1]);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    assert_eq!(actual["stats"]["unknownStats"], original["stats"]["unknownStats"]);
    assert_eq!(result["scenarioHash"], cli.hash()?);
    assert_eq!(String::from_utf8(output.stdout)?, format!("{}\n", result["message"].as_str().ok_or("mutation message missing")?));
    Ok(())
}

#[test]
fn invalid_selectors_and_ordinary_validation_failures_never_write_coverage() -> TestResult {
    // @step Given a temporary coverage project and its exact original sidecar bytes
    for command in ["link-coverage", "unlink-coverage"] {
        for failure in ["stale", "malformed", "index", "file", "unpaired-index", "unpaired-hash", "incompatible", "missing-file", "steps"] {
            let workspace = Workspace::new()?;
            let original = workspace.bytes(SIDECAR)?;
            let hash = workspace.hash()?;
            let stale = "0".repeat(64);
            let mut arguments = vec![command, "thing", "--scenario", SCENARIO, "--test-file",
                if failure == "file" { "tests/other.rs" } else { "tests/thing.rs" }];
            if command == "link-coverage" {
                arguments.extend(["--impl-file", if failure == "missing-file" { "src/missing.rs" } else { "src/second.rs" }, "--impl-lines", "1-3"]);
            } else if matches!(failure, "missing-file" | "steps") {
                continue;
            }
            if failure != "unpaired-hash" {
                arguments.extend(["--test-mapping-index", if failure == "index" { "999" } else { "1" }]);
            }
            if failure != "unpaired-index" {
                arguments.extend(["--expected-scenario-hash", match failure {
                    "stale" => &stale, "malformed" => "not-a-hash", _ => &hash,
                }]);
            }
            if failure == "incompatible" {
                if command == "link-coverage" { arguments.extend(["--test-lines", "1-4"]); }
                else { arguments.push("--all"); }
            }
            if failure == "steps" { workspace.write("tests/thing.rs", "fn missing_steps() {}\n")?; }
            // @step When the CLI receives stale malformed missing mismatched or out-of-range selectors or invalid files and test steps
            let output = workspace.cli(&arguments)?;
            let error = String::from_utf8(output.stderr)?;
            // @step Then each invocation reports the appropriate failure and every original sidecar byte remains unchanged
            assert!(!output.status.success(), "{command} {failure} unexpectedly succeeded");
            assert!(!error.contains("unknown option"), "selector validation never reached core for {command} {failure}: {error}");
            assert!(error.contains("Error:"), "{command} {failure}: {error}");
            if failure == "steps" { assert!(error.to_lowercase().contains("step"), "{error}"); }
            assert!(output.stdout.is_empty(), "failed mutation emitted success output");
            assert_eq!(workspace.bytes(SIDECAR)?, original, "{command} {failure}");
        }
        for invalid in ["-1", "1.5"] {
            let workspace = Workspace::new()?;
            let original = workspace.bytes(SIDECAR)?;
            let hash = workspace.hash()?;
            let index = format!("--test-mapping-index={invalid}");
            let mut arguments = vec![command, "thing", "--scenario", SCENARIO,
                "--test-file", "tests/thing.rs", &index, "--expected-scenario-hash", &hash];
            if command == "link-coverage" {
                arguments.extend(["--impl-file", "src/second.rs", "--impl-lines", "1-3"]);
            }
            let output = workspace.cli(&arguments)?;
            let error = String::from_utf8(output.stderr)?;
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(error.contains("test-mapping-index") && !error.contains("unknown option"), "index typing not enforced: {error}");
            assert_eq!(workspace.bytes(SIDECAR)?, original);
        }
    }
    Ok(())
}
