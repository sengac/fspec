//! Feature: spec/features/automatic-json-file-initialization.feature
//!
//! `ensure_work_units_file` tests, moved out of ensure.rs's inline tests
//! (COV-071: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::TempDir;

const SEEDED_WORK_UNITS: &str = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Original Title", "type": "story", "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002", "title": "Second", "type": "story", "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": ["AUTH-002"], "testing": [], "implementing": [],
    "validating": [], "done": [], "blocked": []
  }
}
"#;

fn seed(tmp: &TempDir) -> std::path::PathBuf {
    std::fs::create_dir(tmp.path().join("spec")).unwrap();
    let path = tmp.path().join("spec/work-units.json");
    std::fs::write(&path, SEEDED_WORK_UNITS).unwrap();
    path
}

const KANBAN_STATES: [&str; 7] = [
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

#[test]
fn calling_ensure_utilities_multiple_times_is_safe() {
    // @step Given I have spec/work-units.json with existing work units
    let tmp = TempDir::new().unwrap();
    let path = seed(&tmp);

    // @step When ensure_work_units_file is called
    let data = ensure_work_units_file(tmp.path()).unwrap();

    // @step Then it should return the existing data
    let ids: Vec<&str> = data.work_units.keys().map(String::as_str).collect();
    assert_eq!(ids, vec!["AUTH-001", "AUTH-002"]);

    // @step And should not modify the file
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SEEDED_WORK_UNITS);

    // @step And should not lose any data
    let again = ensure_work_units_file(tmp.path()).unwrap();
    assert_eq!(again.work_units.len(), 2);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SEEDED_WORK_UNITS);
}

#[test]
fn ensure_utilities_validate_json_structure() {
    // @step Given I have corrupted spec/work-units.json
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join("spec")).unwrap();
    std::fs::write(tmp.path().join("spec/work-units.json"), "{ not json").unwrap();

    // @step When ensure_work_units_file is called
    let result = ensure_work_units_file(tmp.path());

    // @step Then it should throw a helpful error
    let msg = match result {
        Ok(_) => panic!("a corrupted work-units.json must be an error"),
        Err(err) => err.to_string(),
    };

    // @step And should indicate the file is invalid JSON
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "unexpected error message: {msg}"
    );
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("spec/work-units.json")).unwrap(),
        "{ not json",
        "a corrupted file must never be overwritten"
    );
}

#[test]
fn auto_created_files_have_proper_structure() {
    // @step Given I have a fresh project
    let tmp = TempDir::new().unwrap();

    // @step When work-units.json is auto-created
    ensure_work_units_file(tmp.path()).unwrap();
    let raw = std::fs::read_to_string(tmp.path().join("spec/work-units.json")).unwrap();
    let on_disk: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // @step Then it should have meta.version field
    assert_eq!(on_disk["meta"]["version"].as_str(), Some("1.0.0"));
    assert_eq!(on_disk["version"].as_str(), Some("0.7.1"));

    // @step Then it should have meta.lastUpdated timestamp
    let last_updated = on_disk["meta"]["lastUpdated"].as_str().unwrap_or("");
    assert_eq!(last_updated.len(), "2026-06-01T00:00:00.000Z".len());
    assert!(
        last_updated.ends_with('Z') && last_updated.as_bytes().get(10) == Some(&b'T'),
        "meta.lastUpdated must be an ISO-8601 UTC timestamp, got {last_updated:?}"
    );

    // @step And it should have workUnits empty object
    assert_eq!(
        on_disk["workUnits"].as_object().map(serde_json::Map::len),
        Some(0)
    );

    // @step And it should have states with all 7 Kanban states
    let states = on_disk["states"].as_object().unwrap();
    assert_eq!(states.len(), 7);

    // @step And states should each be empty arrays
    for state in KANBAN_STATES {
        assert_eq!(
            states.get(state).and_then(|v| v.as_array()).map(Vec::len),
            Some(0),
            "state {state} must be an empty array"
        );
    }
}

#[test]
fn calling_ensure_work_units_file_multiple_times_returns_same_data_without_overwriting() {
    // @step Given I have spec/work-units.json with existing work unit "AUTH-001"
    let tmp = TempDir::new().unwrap();
    let path = seed(&tmp);

    // @step And the work unit has title "Original Title"
    assert!(SEEDED_WORK_UNITS.contains("\"title\": \"Original Title\""));

    // @step When ensure_work_units_file is called multiple times
    let results: Vec<WorkUnitsData> = (0..3)
        .map(|_| ensure_work_units_file(tmp.path()).unwrap())
        .collect();

    // @step Then it should return the existing data each time
    for data in &results {
        assert!(data.work_units.contains_key("AUTH-001"));
        assert_eq!(data.work_units.len(), 2);
    }

    // @step And the work unit title should still be "Original Title"
    for data in &results {
        assert_eq!(data.work_units["AUTH-001"].title, "Original Title");
    }

    // @step And the file should not be modified or overwritten
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SEEDED_WORK_UNITS);
}
