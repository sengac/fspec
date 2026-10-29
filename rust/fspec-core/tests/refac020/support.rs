use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const SCENARIO: &str = "Thing does the thing";
pub const SIDECAR: &str = "spec/features/thing.feature.coverage";
pub const TEST_FILE: &str = "tests/thing.rs";
pub const FEATURE: &str = "Feature: Thing\n  Scenario: Thing does the thing\n    Given a thing exists\n    When the thing changes\n    Then the change is visible\n";
pub const TEST_SOURCE: &str = "#[test]\nfn thing() {\n    // @step Given a thing exists\n    // @step When the thing changes\n    // @step Then the change is visible\n    assert_eq!(2 + 2, 4);\n}\n";

pub fn write(root: &Path, relative: &str, bytes: impl AsRef<[u8]>) -> TestResult {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

pub fn save(root: &Path, coverage: &Value) -> TestResult {
    write(root, SIDECAR, serde_json::to_vec_pretty(coverage)?)
}

pub fn load(root: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(SIDECAR))?)?)
}

pub fn mapping(lineage: &str) -> Value {
    json!({
        "file": TEST_FILE, "lines": "1-7", "lineage": lineage,
        "unknownRow": {"ordered": ["b", "a", "b"]},
        "implMappings": [{"file": "src/first.rs", "lines": [1, 1, 2],
            "unknownImpl": {"receipt": [3, 2, 3]}}]
    })
}

pub fn coverage() -> Value {
    json!({
        "unknownTop": {"receipt": ["second", "first", "second"]},
        "scenarios": [{"name": SCENARIO,
            "unknownScenario": {"z": 2, "a": [4, 1, 4]},
            "testMappings": [mapping("first"), mapping("second")]}],
        "stats": {"totalScenarios": 1, "coveredScenarios": 1,
            "coveragePercent": 100, "testFiles": [TEST_FILE],
            "implFiles": ["src/first.rs"], "totalLinesCovered": 3,
            "unknownStats": {"receipt": [8, 6, 8]}}
    })
}

pub fn fixture() -> TestResult<tempfile::TempDir> {
    let temporary = setup_temp_directory()?;
    write(temporary.path(), "spec/features/thing.feature", FEATURE)?;
    write(temporary.path(), TEST_FILE, TEST_SOURCE)?;
    for name in ["first", "second", "third"] {
        write(temporary.path(), &format!("src/{name}.rs"), "fn thing() {\n}\n")?;
    }
    save(temporary.path(), &coverage())?;
    Ok(temporary)
}

pub fn request(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

pub fn successful_json(result: DispatchResult) -> TestResult<Value> {
    assert!(result.success, "command failed: {result:?}");
    let parsed = serde_json::from_str(&result.data);
    assert!(parsed.is_ok(), "expected the approved machine JSON response, received: {}", result.data);
    Ok(parsed?)
}

pub fn indexed_link(root: &Path, index: usize, implementation: &str) -> TestResult<Value> {
    Ok(json!({"featureName": "thing", "scenario": SCENARIO,
        "testFile": TEST_FILE, "implFile": implementation, "implLines": "1,2",
        "testMappingIndex": index,
        "expectedScenarioHash": scenario_hash(&load(root)?["scenarios"][0])?}))
}

pub fn scenario_hash(scenario: &Value) -> TestResult<String> {
    Ok(format!("{:x}", Sha256::digest(canonical_text(scenario)?.as_bytes())))
}

fn canonical_text(value: &Value) -> TestResult<String> {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            let mut members = Vec::new();
            for key in keys {
                members.push(format!("{}:{}", serde_json::to_string(key)?, canonical_text(&object[key])?));
            }
            Ok(format!("{{{}}}", members.join(",")))
        }
        Value::Array(array) => {
            let members = array.iter().map(canonical_text).collect::<TestResult<Vec<_>>>()?;
            Ok(format!("[{}]", members.join(",")))
        }
        _ => Ok(serde_json::to_string(value)?),
    }
}

pub fn tree(root: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<String, Vec<u8>>) -> TestResult {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let relative = path.strip_prefix(root)?.to_string_lossy().into_owned();
            if path.is_dir() {
                result.insert(format!("{relative}/"), Vec::new());
                visit(root, &path, result)?;
            } else {
                result.insert(relative, fs::read(path)?);
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result)?;
    Ok(result)
}

pub fn add_new_scenario(root: &Path) -> TestResult {
    write(root, "spec/features/thing.feature", format!("{FEATURE}\n  Scenario: New thing\n    Given another thing exists\n"))
}
