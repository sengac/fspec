//! BUG-515 — fspec audit checks implementation mapping ranges.
//!
//! Feature: spec/features/audit-coverage-checks-implementation-mapping-ranges.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n";

const TEST_FILE: &str = "fn log_in() {\n    // @step Given a user\n}\n";

/// An implementation mapping of `src/login.rs` (whose line n reads
/// `// impl n`) with `lines`; COV-079: lines inside the file carry their
/// recorded first and last line text.
fn impl_mapping(lines: Value) -> Value {
    let mut mapping = json!({"file": "src/login.rs", "lines": lines});
    let numbers: Vec<i64> = match &mapping["lines"] {
        Value::Array(lines) => lines.iter().filter_map(Value::as_i64).collect(),
        Value::String(range) => range
            .split_once('-')
            .and_then(|(a, b)| Some(vec![a.parse().ok()?, b.parse().ok()?]))
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    if let (Some(first), Some(last)) = (numbers.iter().min(), numbers.iter().max()) {
        if *first >= 1 && *last <= 20 {
            mapping["firstLine"] = json!(format!("// impl {first}"));
            mapping["lastLine"] = json!(format!("// impl {last}"));
        }
    }
    mapping
}

/// `src/login.rs` has 20 lines; the scenario's test mapping carries `lines`.
fn project(lines: Value) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/login.feature", FEATURE)?;
    write("tests/login_test.rs", TEST_FILE)?;
    write(
        "src/login.rs",
        &(1..=20)
            .map(|n| format!("// impl {n}\n"))
            .collect::<String>(),
    )?;
    write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": [
            {"file": "tests/login_test.rs", "lines": "1-3",
             "implMappings": [impl_mapping(lines)]}
        ]}]})
        .to_string(),
    )?;
    Ok(root)
}

fn audit(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["login"], "area": ["coverage"]}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let report: Value = serde_json::from_str(&result.data)?;
    let output = report["output"].as_str().unwrap_or_default().to_string();
    Ok((report["exitCode"].clone(), output))
}

#[test]
fn an_implementation_mapping_past_the_end_of_its_file_is_reported() -> TestResult {
    // @step Given an implementation file of 20 lines mapped to lines 18 to 25
    let root = project(json!([18, 19, 20, 21, 22, 23, 24, 25]))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails saying the mapping lies outside the file of 20 lines
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("src/login.rs:18-25: lies outside the file (20 lines)"),
        "{output}"
    );
    Ok(())
}

#[test]
fn an_implementation_mapping_inside_its_file_passes() -> TestResult {
    // @step Given an implementation file of 20 lines mapped to the range string "5-9"
    let root = project(json!("5-9"))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn an_implementation_mapping_that_lists_no_lines_is_reported() -> TestResult {
    // @step Given an implementation mapping with an empty line list
    let root = project(json!([]))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails saying the mapping lists no lines
    assert_eq!(exit, 1, "{output}");
    assert!(output.contains("src/login.rs: lists no lines"), "{output}");
    Ok(())
}
