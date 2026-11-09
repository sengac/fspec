//! BUG-056 — Work unit IDs reused after deletion violating immutability principle.
//!
//! Feature: spec/features/work-unit-ids-reused-after-deletion-violating-immutability-principle.feature
//!
//! create-bug, create-story and create-task give the next ID of a prefix as
//! one more than the larger of `prefixCounters[prefix]` and the highest
//! existing `<prefix>-NNN` suffix, then store the new number in
//! `prefixCounters`; delete-work-unit never lowers a counter.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const STATES: [&str; 7] = [
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

/// A temporary project with spec/foundation.json and the prefixes BUG,
/// AUTH, DASH and TASK registered.
struct Project(tempfile::TempDir);

impl Project {
    /// The project with work units `units` (id, type) in backlog and the
    /// top-level `prefixCounters` when given.
    fn new(
        units: &[(&str, &str)],
        counters: Option<Value>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/foundation.json", r#"{"version":"2.0.0"}"#)?;
        let mut prefixes = serde_json::Map::new();
        for prefix in ["BUG", "AUTH", "DASH", "TASK"] {
            prefixes.insert(
                prefix.to_string(),
                json!({"prefix": prefix, "description": "desc",
                       "createdAt": "2026-06-01T00:00:00.000Z"}),
            );
        }
        project.write(
            "spec/prefixes.json",
            &json!({ "prefixes": prefixes }).to_string(),
        )?;
        let mut work_units = serde_json::Map::new();
        for (id, kind) in units {
            work_units.insert(
                (*id).to_string(),
                json!({"id": id, "title": id, "type": kind, "status": "backlog",
                       "createdAt": "2026-06-01T00:00:00.000Z",
                       "updatedAt": "2026-06-01T00:00:00.000Z"}),
            );
        }
        let mut states = serde_json::Map::new();
        for state in STATES {
            let ids: Vec<&str> = if state == "backlog" {
                units.iter().map(|(id, _)| *id).collect()
            } else {
                Vec::new()
            };
            states.insert(state.to_string(), json!(ids));
        }
        let mut data = json!({
            "version": "0.7.1",
            "workUnits": work_units,
            "states": states,
        });
        if let Some(counters) = counters {
            data["prefixCounters"] = counters;
        }
        project.write("spec/work-units.json", &data.to_string())?;
        Ok(project)
    }

    fn write(&self, rel: &str, body: &str) -> std::io::Result<()> {
        let path = self.0.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }

    fn dispatch(&self, command: &str, args: Value) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: command.to_string(),
            args_json: args.to_string(),
            project_root: self.root().to_path_buf(),
        })
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn board(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(self.root().join("spec/work-units.json"))?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// The IDs of the board's work units.
    fn ids(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(self.board()?["workUnits"]
            .as_object()
            .map(|units| units.keys().cloned().collect())
            .unwrap_or_default())
    }

    /// Run `command` (create-bug, create-story or create-task) with
    /// `prefix` and `title`; the ID of the work unit it added.
    fn create(
        &self,
        command: &str,
        prefix: &str,
        title: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let before = self.ids()?;
        let result = self.dispatch(command, json!({"prefix": prefix, "title": title}));
        if !result.success {
            return Err(format!("{command} {prefix} failed: {result:?}").into());
        }
        let added: Vec<String> = self
            .ids()?
            .into_iter()
            .filter(|id| !before.contains(id))
            .collect();
        match added.as_slice() {
            [id] => Ok(id.clone()),
            _ => Err(format!("{command} added {added:?}").into()),
        }
    }

    fn counter(&self, prefix: &str) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        Ok(self.board()?["prefixCounters"][prefix].as_u64())
    }
}

#[test]
fn work_unit_id_preserved_after_deletion() -> TestResult {
    // @step Given I have a project with work-units.json
    let project = Project::new(&[], None)?;

    // @step And work units "BUG-001", "BUG-002", and "BUG-003" exist
    for title in ["First", "Second", "Third"] {
        project.create("create-bug", "BUG", title)?;
    }
    assert_eq!(project.ids()?, ["BUG-001", "BUG-002", "BUG-003"]);

    // @step And the prefixCounters.BUG is 3
    assert_eq!(project.counter("BUG")?, Some(3));

    // @step When I delete work unit "BUG-003"
    let deleted = project.dispatch("delete-work-unit", json!({"workUnitId": "BUG-003"}));
    assert!(deleted.success, "{deleted:?}");

    // @step And I create a new bug with title "New bug"
    let id = project.create("create-bug", "BUG", "New bug")?;

    // @step Then the new bug should have ID "BUG-004"
    assert_eq!(id, "BUG-004");

    // @step And the prefixCounters.BUG should be 4
    assert_eq!(project.counter("BUG")?, Some(4));

    // @step And the work unit "BUG-003" should not exist
    assert!(project.board()?["workUnits"].get("BUG-003").is_none());
    Ok(())
}

#[test]
fn multiple_prefixes_maintain_separate_high_water_marks() -> TestResult {
    // @step Given I have a project whose work-units.json has prefixCounters AUTH 1, DASH 5 and TASK 10
    let project = Project::new(&[], Some(json!({"AUTH": 1, "DASH": 5, "TASK": 10})))?;

    // @step When I create a new bug
    let bug = project.create("create-bug", "BUG", "A bug")?;

    // @step And I create a new story with prefix "AUTH"
    let auth = project.create("create-story", "AUTH", "An auth story")?;

    // @step And I create a new story with prefix "DASH"
    let dash = project.create("create-story", "DASH", "A dash story")?;

    // @step And I create a new task
    let task = project.create("create-task", "TASK", "A task")?;

    // @step Then the new bug should have ID "BUG-001"
    assert_eq!(bug, "BUG-001");

    // @step And the new AUTH story should have ID "AUTH-002"
    assert_eq!(auth, "AUTH-002");

    // @step And the new DASH story should have ID "DASH-006"
    assert_eq!(dash, "DASH-006");

    // @step And the new task should have ID "TASK-011"
    assert_eq!(task, "TASK-011");
    assert_eq!(
        project.board()?["prefixCounters"],
        json!({"AUTH": 2, "BUG": 1, "DASH": 6, "TASK": 11})
    );
    Ok(())
}

#[test]
fn next_ids_are_calculated_from_existing_ids_when_prefix_counters_is_absent() -> TestResult {
    // @step Given I have a work-units.json file without prefixCounters field
    // @step And existing work units:
    let project = Project::new(
        &[
            ("BUG-010", "bug"),
            ("BUG-025", "bug"),
            ("AUTH-003", "story"),
            ("DASH-007", "story"),
        ],
        None,
    )?;
    assert!(project.board()?.get("prefixCounters").is_none());

    // @step When I create a new bug
    let id = project.create("create-bug", "BUG", "A bug")?;

    // @step Then the new bug should have ID "BUG-026"
    assert_eq!(id, "BUG-026");
    assert_eq!(project.board()?["prefixCounters"], json!({"BUG": 26}));
    Ok(())
}

#[test]
fn backward_compatibility_with_existing_projects() -> TestResult {
    // @step Given I have an existing project with work-units.json
    // @step And the highest bug ID is "BUG-054"
    // @step And the work-units.json file has no prefixCounters field
    let project = Project::new(&[("BUG-012", "bug"), ("BUG-054", "bug")], None)?;
    assert!(project.board()?.get("prefixCounters").is_none());

    // @step When I create a new bug with title "Attachment duplication"
    let id = project.create("create-bug", "BUG", "Attachment duplication")?;

    // @step Then the new bug should have ID "BUG-055"
    assert_eq!(id, "BUG-055");

    // @step And the prefixCounters.BUG should be updated to 55
    assert_eq!(project.counter("BUG")?, Some(55));
    Ok(())
}
