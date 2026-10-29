use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::TestResult;

pub const SCENARIO: &str = "Thing";
pub const SIDECAR: &str = "spec/features/thing.feature.coverage";
pub const FEATURE: &str = "@CLI-001\nFeature: Thing\n\n  Scenario: Thing\n    Given a thing exists\n    When the thing is used\n    Then the thing works\n";
pub const TEST: &str = "// @step Given a thing exists\n// @step When the thing is used\n// @step Then the thing works\nfn verifies_thing() {}\n";

pub struct Workspace {
    temporary: tempfile::TempDir,
}

impl Workspace {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let workspace = Self { temporary: codelet_test_helpers::setup_temp_directory()? };
        std::fs::create_dir_all(workspace.path().join(".home"))?;
        workspace.write("spec/features/thing.feature", FEATURE)?;
        workspace.write("tests/thing.rs", TEST)?;
        workspace.write("tests/other.rs", TEST)?;
        workspace.write("src/first.rs", "fn first() {\n    return;\n}\n")?;
        workspace.write("src/second.rs", "fn second() {\n    return;\n}\n")?;
        workspace.write("spec/work-units.json", &json!({"workUnits": {
            "CLI-001": {"id": "CLI-001", "title": "Thing", "type": "story", "status": "testing"}
        }}).to_string())?;
        let implementation = json!({"file": "src/first.rs", "lines": [1, 2, 2],
            "vendorImpl": {"owners": ["keep", "keep"], "evidence": {"build": 7}}});
        let row = json!({"file": "tests/thing.rs", "lines": "1-4", "origin": "first",
            "implMappings": [implementation.clone(), implementation]});
        let mut second = row.clone();
        second["origin"] = json!("second");
        workspace.write(SIDECAR, &serde_json::to_string_pretty(&json!({
            "unknownTop": {"retained": [2, 2, 1]},
            "scenarios": [{"name": SCENARIO, "unknownScenario": {"z": 1, "a": 2},
                "testMappings": [row.clone(), second, row]}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
                "testFiles": ["tests/thing.rs"], "implFiles": ["src/first.rs"],
                "totalLinesCovered": 0, "unknownStats": "retained"}
        }))?)?;
        Ok(workspace)
    }

    pub fn path(&self) -> &Path {
        self.temporary.path()
    }

    pub fn write(&self, relative: &str, body: &str) -> TestResult {
        let path = self.path().join(relative);
        std::fs::create_dir_all(path.parent().ok_or("fixture has no parent")?)?;
        std::fs::write(path, body)?;
        Ok(())
    }

    pub fn bytes(&self, relative: &str) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(self.path().join(relative))
    }

    pub fn sidecar(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&self.bytes(SIDECAR)?)?)
    }

    pub fn hash(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(scenario_hash(&self.sidecar()?["scenarios"][0]))
    }

    pub fn cli(&self, arguments: &[&str]) -> Result<Output, std::io::Error> {
        Command::new(env!("CARGO_BIN_EXE_fspec"))
            .args(arguments)
            .current_dir(self.path())
            .env("HOME", self.path().join(".home"))
            .env("XDG_CONFIG_HOME", self.path().join(".home/config"))
            .env("XDG_DATA_HOME", self.path().join(".home/data"))
            .env("XDG_RUNTIME_DIR", self.path().join(".home/runtime"))
            .env("NO_COLOR", "1")
            .env_remove("CLICOLOR_FORCE")
            .output()
    }

    pub fn dispatch(&self, command: &str, args: Value) -> DispatchResult {
        dispatch_command(DispatchRequest { command: command.into(), args_json: args.to_string(),
            project_root: self.path().to_path_buf() })
    }

    pub fn snapshot(&self) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>, std::io::Error> {
        fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(directory)? {
                let path = entry?.path();
                let relative = path.strip_prefix(root).map_err(std::io::Error::other)?.to_path_buf();
                if path.is_dir() {
                    files.insert(relative, None);
                    visit(root, &path, files)?;
                } else {
                    files.insert(relative, Some(std::fs::read(path)?));
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        visit(self.path(), self.path(), &mut files)?;
        Ok(files)
    }
}

pub fn success(output: &Output) {
    assert!(output.status.success(), "CLI failed: status={:?}, stdout={}, stderr={}",
        output.status.code(), String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "successful CLI stderr: {}", String::from_utf8_lossy(&output.stderr));
}

pub fn json_output(output: &Output) -> Result<Value, serde_json::Error> {
    success(output);
    serde_json::from_slice(&output.stdout)
}

pub fn native_json(result: DispatchResult) -> Result<Value, serde_json::Error> {
    assert!(result.success, "native dispatch failed: {:?}", result.error);
    serde_json::from_str(&result.data)
}

pub fn scenario_hash(value: &Value) -> String {
    fn canonical(value: &Value) -> Value {
        match value {
            Value::Object(object) => {
                let ordered = object.iter().map(|(key, value)| (key.clone(), canonical(value)))
                    .collect::<BTreeMap<_, _>>();
                Value::Object(ordered.into_iter().collect())
            }
            Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
            value => value.clone(),
        }
    }
    hex::encode(Sha256::digest(canonical(value).to_string().as_bytes()))
}
