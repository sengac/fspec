// Feature: spec/features/commands-write-the-spec-directory-they-read.feature
//
// BUG-523: 'fspec update-work-unit-status BUG-481 implementing' reported
// success but the board kept 'testing'. It ran with rust/ as its working
// directory: the spec lock and the loaders walked up to the repository's
// spec/, but the command wrote <working directory>/spec/work-units.json, a
// new stray board. These scenarios give every project a subdirectory "rust"
// with its own Cargo.toml (a project-boundary marker, as in this repository)
// and check that commands handed that subdirectory still read, lock and
// write the project's one spec/ directory.
//
// The concurrency scenario extends BUG-477's
// (bug477_concurrent_spec_writes.rs): its writers are SEPARATE PROCESSES.
// The test binary re-executes itself (`--exact bug523_child_worker`) with
// BUG523_* environment variables; each child drives the real commands
// through `dispatch_command` and records its failures in
// `<project>/outcomes/<name>.json`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SUBDIR: &str = "rust";
const STATUS_UNITS: usize = 8;
const RULE_UNITS: usize = 4;
const ROUNDS: usize = 4;
const BUG_WRITERS: usize = 2;
const FEATURE: &str = "subdirectory-writers";
const SCENARIOS: usize = 4;

// ---------------------------------------------------------------- fixture

fn status_id(i: usize) -> String {
    format!("STAT-{i:03}")
}

fn rule_id(i: usize) -> String {
    format!("RULE-{i:03}")
}

fn scenario_name(i: usize) -> String {
    format!("Scenario {i}")
}

fn unit(id: &str, work_type: &str) -> Value {
    json!({
        "id": id,
        "title": id,
        "type": work_type,
        "status": "specifying",
        "createdAt": "2026-11-05T00:00:00.000Z",
        "updatedAt": "2026-11-05T00:00:00.000Z",
        "stateHistory": [{"state": "specifying", "timestamp": "2026-11-05T00:00:00.000Z"}]
    })
}

/// A project (marked by .gitignore) whose spec/ has a board with
/// STAT-001..STAT-008 (tasks) and RULE-001..RULE-004 (stories), all in
/// specifying, the BUG prefix, a foundation, and a feature with four
/// scenarios plus its coverage file; and a subdirectory "rust" holding its
/// own Cargo.toml.
fn seed_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let spec = root.join("spec");
    fs::create_dir_all(spec.join("features"))?;
    fs::create_dir_all(root.join("outcomes"))?;
    fs::create_dir_all(root.join("tests"))?;
    fs::write(root.join(".gitignore"), "")?;
    fs::create_dir_all(root.join(SUBDIR))?;
    fs::write(
        root.join(SUBDIR).join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )?;

    let mut units = serde_json::Map::new();
    let mut specifying = Vec::new();
    for i in 1..=STATUS_UNITS {
        units.insert(status_id(i), unit(&status_id(i), "task"));
        specifying.push(status_id(i));
    }
    for i in 1..=RULE_UNITS {
        units.insert(rule_id(i), unit(&rule_id(i), "story"));
        specifying.push(rule_id(i));
    }
    let board = json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-05T00:00:00.000Z"},
        "workUnits": units,
        "states": {
            "backlog": [], "specifying": specifying, "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    let prefixes = json!({"prefixes": {"BUG": {
        "prefix": "BUG", "description": "Bugs", "createdAt": "2026-11-05T00:00:00.000Z"
    }}});
    fs::write(
        spec.join("prefixes.json"),
        serde_json::to_string_pretty(&prefixes)?,
    )?;
    let foundation = json!({
        "version": "2.0.0",
        "project": {"name": "bug523", "vision": "v", "projectType": "cli-tool"},
        "problemSpace": {"primaryProblem": {"title": "t", "description": "d", "impact": "high"}},
        "solutionSpace": {"overview": "o", "capabilities": []},
        "personas": [],
        "architectureDiagrams": []
    });
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(&foundation)?,
    )?;

    let mut feature = String::from("Feature: Subdirectory writers\n");
    let mut coverage_scenarios = Vec::new();
    for i in 1..=SCENARIOS {
        feature.push_str(&format!(
            "\n  Scenario: {}\n    Given a precondition\n    When an action\n    Then an outcome\n",
            scenario_name(i)
        ));
        coverage_scenarios.push(json!({"name": scenario_name(i), "testMappings": []}));
        fs::write(
            root.join("tests").join(format!("t{i}.rs")),
            "// @step Given a precondition\n// @step When an action\n// @step Then an outcome\n",
        )?;
    }
    fs::write(
        spec.join("features").join(format!("{FEATURE}.feature")),
        feature,
    )?;
    let coverage = json!({
        "scenarios": coverage_scenarios,
        "stats": {"totalScenarios": SCENARIOS, "coveredScenarios": 0, "coveragePercent": 0,
                  "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
    });
    fs::write(
        spec.join("features")
            .join(format!("{FEATURE}.feature.coverage")),
        serde_json::to_string_pretty(&coverage)?,
    )?;
    Ok(dir)
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn board(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    read_json(&root.join("spec").join("work-units.json"))
}

fn rules_of(board: &Value, id: &str) -> Vec<String> {
    board["workUnits"][id]["rules"]
        .as_array()
        .map(|rules| {
            rules
                .iter()
                .filter_map(|r| r["text"].as_str().or_else(|| r.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn assert_no_subdir_spec(root: &Path) {
    let stray = root.join(SUBDIR).join("spec");
    assert!(
        !stray.exists(),
        "a stray spec directory was created at {}",
        stray.display()
    );
}

fn dispatch(root: &Path, command: &str, args: Value) -> Result<String, String> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "no error text".to_string()));
    }
    if let Ok(data) = serde_json::from_str::<Value>(&result.data) {
        if data.get("success") == Some(&Value::Bool(false)) {
            return Err(result.data);
        }
    }
    Ok(result.data)
}

// ------------------------------------------------------------------ child

fn child_command(root: &Path, role: &str, name: &str) -> std::io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "bug523_child_worker", "--nocapture", "--quiet"])
        .env("BUG523_ROOT", root)
        .env("BUG523_ROLE", role)
        .env("BUG523_NAME", name);
    Ok(command)
}

fn spawn_child(root: &Path, role: &str, name: &str) -> std::io::Result<Child> {
    child_command(root, role, name)?.spawn()
}

/// The commands one child runs, and the project root it hands them.
fn child_commands(root: &Path, role: &str, index: usize) -> Vec<(PathBuf, &'static str, Value)> {
    let at_root = root.to_path_buf();
    let at_subdir = root.join(SUBDIR);
    match role {
        // Odd status units are changed from the project root, even ones
        // from the subdirectory (where BUG-481's change was lost).
        "status" => {
            let cwd = if index.is_multiple_of(2) {
                at_subdir
            } else {
                at_root
            };
            let id = status_id(index);
            (0..ROUNDS)
                .flat_map(|r| {
                    [
                        (
                            cwd.clone(),
                            "update-work-unit-status",
                            json!({"workUnitId": id, "status": "blocked",
                                   "blockedReason": format!("round {r}")}),
                        ),
                        (
                            cwd.clone(),
                            "update-work-unit-status",
                            json!({"workUnitId": id, "status": "specifying"}),
                        ),
                    ]
                })
                .collect()
        }
        "rules" => {
            let cwd = if index.is_multiple_of(2) {
                at_subdir
            } else {
                at_root
            };
            (0..ROUNDS * 2)
                .map(|k| {
                    (
                        cwd.clone(),
                        "add-rule",
                        json!({"workUnitId": rule_id(index), "rule": format!("rule {index}-{k}")}),
                    )
                })
                .collect()
        }
        "bugs" => (0..ROUNDS)
            .map(|k| {
                (
                    at_root.clone(),
                    "create-bug",
                    json!({"prefix": "BUG", "title": format!("bug {index}-{k}")}),
                )
            })
            .collect(),
        "links" => (1..=SCENARIOS)
            .map(|s| {
                (
                    at_root.clone(),
                    "link-coverage",
                    json!({"featureName": FEATURE, "scenario": scenario_name(s),
                           "testFile": format!("tests/t{s}.rs"), "testLines": "1-3"}),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Child-process entry point. A no-op unless BUG523_ROLE is set.
#[test]
fn bug523_child_worker() -> TestResult {
    let (Ok(root), Ok(role), Ok(name)) = (
        std::env::var("BUG523_ROOT"),
        std::env::var("BUG523_ROLE"),
        std::env::var("BUG523_NAME"),
    ) else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    let index: usize = name
        .rsplit('-')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let mut failures = Vec::new();
    for (cwd, command, args) in child_commands(&root, &role, index) {
        if let Err(error) = dispatch(&cwd, command, args) {
            failures.push(json!({"command": command, "error": error}));
        }
    }
    fs::write(
        root.join("outcomes").join(format!("{name}.json")),
        json!({"failures": failures}).to_string(),
    )?;
    Ok(())
}

// -------------------------------------------------------------- scenarios

/// Scenario: An agent tool call with a subdirectory as project root writes the project's board
#[test]
fn dispatch_with_a_subdirectory_root_writes_the_project_board() -> TestResult {
    // @step Given a project whose board has the work unit "RULE-001"
    // @step And a subdirectory "rust" of the project with its own Cargo.toml
    let project = seed_project()?;
    let root = project.path();

    // @step When dispatch_command runs add-rule for "RULE-001" with "rust" as the project root
    let outcome = dispatch(
        &root.join(SUBDIR),
        "add-rule",
        json!({"workUnitId": "RULE-001", "rule": "written from rust/"}),
    );

    // @step Then it succeeds
    assert!(outcome.is_ok(), "add-rule failed: {outcome:?}");

    // @step And "RULE-001" has the rule in the project's spec/work-units.json
    let rules = rules_of(&board(root)?, "RULE-001");
    assert_eq!(rules, vec!["written from rust/".to_string()]);

    // @step And no spec directory exists in "rust"
    assert_no_subdir_spec(root);
    Ok(())
}

/// Scenario: A writer given a root whose spec directory lies elsewhere fails and changes nothing
#[tokio::test]
async fn a_writer_handed_a_subdirectory_root_fails_and_changes_nothing() -> TestResult {
    // @step Given a project whose board has the work unit "RULE-001"
    // @step And a subdirectory "rust" of the project with its own Cargo.toml
    let project = seed_project()?;
    let root = project.path();
    let before = fs::read(root.join("spec").join("work-units.json"))?;

    // @step When add-rule runs directly with "rust" as the project root
    let args = json!({"workUnitId": "RULE-001", "rule": "must not land anywhere"}).to_string();
    let outcome = codelet_fspec_core::commands::add_rule::run(&args, &root.join(SUBDIR)).await;

    // @step Then it fails naming the project's spec directory
    let error = match outcome {
        Ok(out) => {
            return Err(format!("add-rule succeeded with a subdirectory root: {out}").into())
        }
        Err(e) => e.to_string(),
    };
    let project_spec = root.join("spec");
    assert!(
        error.contains(&project_spec.display().to_string()),
        "the error names {}: {error}",
        project_spec.display()
    );

    // @step And spec/work-units.json is byte-for-byte unchanged
    assert_eq!(fs::read(root.join("spec").join("work-units.json"))?, before);

    // @step And no spec directory exists in "rust"
    assert_no_subdir_spec(root);
    Ok(())
}

/// Scenario: init run from a subdirectory still initialises that subdirectory
#[test]
fn init_from_a_subdirectory_initialises_that_subdirectory() -> TestResult {
    // @step Given a project with a spec directory
    // @step And a subdirectory "rust" of the project with its own Cargo.toml
    let project = seed_project()?;
    let root = project.path();

    // @step When dispatch_command runs init for the claude agent with "rust" as the project root
    let outcome = dispatch(&root.join(SUBDIR), "init", json!({"agent": ["claude"]}));

    // @step Then it succeeds
    assert!(outcome.is_ok(), "init failed: {outcome:?}");

    // @step And "rust/spec/fspec-config.json" records the claude agent
    let config = read_json(&root.join(SUBDIR).join("spec").join("fspec-config.json"))?;
    assert_eq!(config["agent"], json!("claude"));

    // @step And the project's spec directory has no fspec-config.json
    assert!(!root.join("spec").join("fspec-config.json").exists());
    Ok(())
}

/// Scenario: Concurrent writers from the project root and from a subdirectory lose no update
#[test]
fn concurrent_writers_from_root_and_subdirectory_lose_no_update() -> TestResult {
    // @step Given a project whose board has eight status work units, four rule work units and a feature with four scenarios
    // @step And a subdirectory "rust" of the project with its own Cargo.toml
    let project = seed_project()?;
    let root = project.path();

    // @step When separate processes at the same time change statuses from the project root and from "rust", add rules, create bugs and link coverage
    let mut children = Vec::new();
    let mut expected = 0;
    for i in 1..=STATUS_UNITS {
        children.push(spawn_child(root, "status", &format!("status-{i}"))?);
        expected += 1;
    }
    for i in 1..=RULE_UNITS {
        children.push(spawn_child(root, "rules", &format!("rules-{i}"))?);
        expected += 1;
    }
    for i in 1..=BUG_WRITERS {
        children.push(spawn_child(root, "bugs", &format!("bugs-{i}"))?);
        expected += 1;
    }
    children.push(spawn_child(root, "links", "links-0")?);
    expected += 1;
    for mut child in children {
        let status = child.wait()?;
        assert!(status.success(), "child process exited with {status}");
    }

    // @step Then every process succeeds
    let mut seen = 0;
    for entry in fs::read_dir(root.join("outcomes"))? {
        let path = entry?.path();
        let outcome = read_json(&path)?;
        assert_eq!(
            outcome["failures"],
            json!([]),
            "{} had failing commands: {outcome}",
            path.display()
        );
        seen += 1;
    }
    assert_eq!(seen, expected, "every child records an outcome");

    // @step And every status change, rule and bug is in the project's spec/work-units.json
    let board = board(root)?;
    for i in 1..=STATUS_UNITS {
        let id = status_id(i);
        let wu = &board["workUnits"][&id];
        assert_eq!(wu["status"], json!("specifying"), "{id}: {wu}");
        let history = wu["stateHistory"].as_array().map(Vec::len).unwrap_or(0);
        assert_eq!(
            history,
            1 + 2 * ROUNDS,
            "{id} records every status change: {wu}"
        );
        let in_column = board["states"]["specifying"]
            .as_array()
            .is_some_and(|ids| ids.iter().any(|v| v == &json!(id)));
        assert!(in_column, "{id} is in the specifying column");
    }
    for i in 1..=RULE_UNITS {
        let id = rule_id(i);
        assert_eq!(
            rules_of(&board, &id).len(),
            ROUNDS * 2,
            "{id} has every rule"
        );
    }
    let bugs = board["workUnits"]
        .as_object()
        .map(|units| units.keys().filter(|k| k.starts_with("BUG-")).count())
        .unwrap_or(0);
    assert_eq!(bugs, BUG_WRITERS * ROUNDS, "every bug was created");

    // @step And every coverage link is in the feature's coverage file
    let coverage = read_json(
        &root
            .join("spec")
            .join("features")
            .join(format!("{FEATURE}.feature.coverage")),
    )?;
    let linked = coverage["scenarios"]
        .as_array()
        .map(|s| {
            s.iter()
                .filter(|sc| sc["testMappings"].as_array().is_some_and(|m| !m.is_empty()))
                .count()
        })
        .unwrap_or(0);
    assert_eq!(linked, SCENARIOS, "every scenario is linked: {coverage}");

    // @step And no spec directory exists in "rust"
    assert_no_subdir_spec(root);
    Ok(())
}
