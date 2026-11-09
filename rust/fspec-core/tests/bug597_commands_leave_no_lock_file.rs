//! BUG-597: an fspec command that writes the board leaves no
//! spec/.fspec-write.lock behind.
//!
//! Feature: spec/features/the-spec-write-lock-is-removed-when-the-write-finishes.feature

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_fspec_command_leaves_no_lock_file() -> TestResult {
    // @step Given a project fspec manages
    let root = tempfile::tempdir()?;
    let spec = root.path().join("spec");
    std::fs::create_dir_all(spec.join("features"))?;
    std::fs::write(
        spec.join("work-units.json"),
        json!({
            "meta": {"version": "0.7.1", "lastUpdated": "2026-11-07T00:00:00.000Z"},
            "workUnits": {},
            "states": {"backlog": [], "specifying": [], "testing": [], "implementing": [],
                       "validating": [], "done": [], "blocked": []}
        })
        .to_string(),
    )?;
    std::fs::write(
        spec.join("prefixes.json"),
        json!({"prefixes": {"LOCK": {"prefix": "LOCK", "description": "lock",
                                     "createdAt": "2026-11-07T00:00:00.000Z"}}})
        .to_string(),
    )?;
    std::fs::write(
        spec.join("foundation.json"),
        json!({
            "version": "2.0.0",
            "project": {"name": "bug597", "vision": "v", "projectType": "cli-tool"},
            "problemSpace": {"primaryProblem": {"title": "t", "description": "d", "impact": "high"}},
            "solutionSpace": {"overview": "o", "capabilities": []},
            "personas": [],
            "architectureDiagrams": []
        })
        .to_string(),
    )?;

    // @step When fspec create-story writes the board
    let result = dispatch_command(DispatchRequest {
        command: "create-story".to_string(),
        args_json: json!({"prefix": "LOCK", "title": "No lock file"}).to_string(),
        project_root: root.path().to_path_buf(),
    });
    assert!(result.success, "create-story failed: {result:?}");
    assert!(root.path().join("spec/work-units.json").is_file());

    // @step Then spec/ holds no .fspec-write.lock
    assert!(!root.path().join("spec/.fspec-write.lock").exists());
    let locks: Vec<String> = std::fs::read_dir(&spec)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".lock"))
        .collect();
    assert!(locks.is_empty(), "lock files left in spec/: {locks:?}");
    Ok(())
}
