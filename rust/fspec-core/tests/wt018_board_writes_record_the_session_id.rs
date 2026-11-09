//! WT-018: board writes made from a worktree record the session id.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Commands run through `dispatch_command`, the front door every agent
//! tool call and CLI bridge uses. "A worktree" is a plain copy of the main
//! checkout holding a `.fspec/state-root` pointer as fspec writes it (JSON
//! `{stateRoot, sessionId}`); the resolver reads a plain file, so no git is
//! involved.

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use codelet_test_helpers::{TestError, TestResult};
use serde_json::{json, Value};

fn board_json() -> Value {
    let unit = |id: &str, status: &str| {
        json!({
            "id": id, "title": id, "type": "story", "status": status,
            "createdAt": "2026-11-08T00:00:00.000Z", "updatedAt": "2026-11-08T00:00:00.000Z",
            "stateHistory": [{"state": status, "timestamp": "2026-11-08T00:00:00.000Z"}]
        })
    };
    json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-08T00:00:00.000Z"},
        "workUnits": {
            "STORY-001": unit("STORY-001", "backlog"),
            "STORY-002": unit("STORY-002", "backlog"),
            "STORY-003": unit("STORY-003", "specifying"),
            "STORY-004": unit("STORY-004", "specifying")
        },
        "states": {
            "backlog": ["STORY-001", "STORY-002"], "specifying": ["STORY-003", "STORY-004"],
            "testing": [], "implementing": [], "validating": [], "done": [], "blocked": []
        }
    })
}

fn write_project(root: &Path) -> Result<(), TestError> {
    let spec = root.join("spec");
    fs::create_dir_all(spec.join("features"))?;
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board_json())?,
    )?;
    let prefix = json!({"prefix": "STORY", "description": "Stories", "createdAt": "2026-11-08T00:00:00.000Z"});
    fs::write(
        spec.join("prefixes.json"),
        serde_json::to_string_pretty(&json!({"prefixes": {"STORY": prefix}}))?,
    )?;
    let foundation = json!({
        "version": "2.0.0",
        "project": {"name": "wt018", "vision": "v", "projectType": "cli-tool"},
        "problemSpace": {"primaryProblem": {"title": "t", "description": "d", "impact": "high"}},
        "solutionSpace": {"overview": "o", "capabilities": []},
        "personas": [],
        "architectureDiagrams": []
    });
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(&foundation)?,
    )?;
    Ok(())
}

/// A main checkout and a plain copy of it in main's `.fspec/worktrees`
/// whose pointer names session `s1`.
fn main_and_worktree() -> Result<(tempfile::TempDir, PathBuf, PathBuf), TestError> {
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    write_project(&main)?;
    let worktree = main.join(".fspec").join("worktrees").join("s1");
    write_project(&worktree)?;
    fs::create_dir_all(worktree.join(".fspec"))?;
    fs::write(
        worktree.join(".fspec").join("state-root"),
        json!({"stateRoot": main, "sessionId": "s1"}).to_string(),
    )?;
    Ok((dir, main, worktree))
}

fn dispatch(root: &Path, command: &str, args: Value) -> Result<(), TestError> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if result.success {
        Ok(())
    } else {
        Err(TestError::from(format!(
            "{command} failed: {}",
            result.error.unwrap_or(result.data)
        )))
    }
}

fn board(root: &Path) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&fs::read_to_string(
        root.join("spec").join("work-units.json"),
    )?)?)
}

/// The id of the one card titled `title`.
fn card_titled(board: &Value, title: &str) -> Result<String, TestError> {
    board["workUnits"]
        .as_object()
        .and_then(|units| {
            units
                .iter()
                .find(|(_, unit)| unit["title"] == title)
                .map(|(id, _)| id.clone())
        })
        .ok_or_else(|| TestError::from(format!("no card titled {title}")))
}

fn last(board: &Value, id: &str, field: &str) -> Value {
    board["workUnits"][id][field]
        .as_array()
        .and_then(|items| items.last())
        .cloned()
        .unwrap_or(Value::Null)
}

/// Every session marker anywhere inside `value`.
fn session_markers(value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if ["sessionId", "createdInSession", "editedInSessions"].contains(&key.as_str()) {
                    found.push(format!("{key}={inner}"));
                }
                found.extend(session_markers(inner));
            }
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| found.extend(session_markers(item))),
        _ => {}
    }
    found
}

/// The three writes of the scenario, run from `root`.
fn run_the_commands(root: &Path, moved: &str, title: &str, ruled: &str) -> TestResult {
    dispatch(
        root,
        "update-work-unit-status",
        json!({"workUnitId": moved, "status": "specifying"}),
    )?;
    dispatch(
        root,
        "create-story",
        json!({"prefix": "STORY", "title": title}),
    )?;
    dispatch(
        root,
        "add-rule",
        json!({"workUnitId": ruled, "rule": "A rule"}),
    )?;
    Ok(())
}

#[test]
fn board_writes_made_from_a_worktree_record_the_session_id() -> TestResult {
    // @step Given a worktree whose state-root pointer names session "s1"
    let (_dir, main, worktree) = main_and_worktree()?;

    // @step When update-work-unit-status moves STORY-001 to specifying, create-story creates a card and add-rule adds a rule to STORY-003 in the worktree
    run_the_commands(&worktree, "STORY-001", "Made in s1", "STORY-003")?;
    let after = board(&main)?;

    // @step Then STORY-001's new stateHistory entry carries sessionId "s1"
    let entry = last(&after, "STORY-001", "stateHistory");
    assert_eq!(entry["state"], "specifying", "{entry}");
    assert_eq!(entry["sessionId"], "s1", "{entry}");
    let first = &after["workUnits"]["STORY-001"]["stateHistory"][0];
    assert!(
        first.get("sessionId").is_none(),
        "the entry written before the session must stay as it was: {first}"
    );

    // @step And the new card carries createdInSession "s1"
    let created = card_titled(&after, "Made in s1")?;
    assert_eq!(after["workUnits"][&created]["createdInSession"], "s1");

    // @step And STORY-003's new rule carries the session id "s1"
    let rule = last(&after, "STORY-003", "rules");
    assert_eq!(rule["text"], "A rule", "{rule}");
    assert_eq!(rule["sessionId"], "s1", "{rule}");
    assert_eq!(
        after["workUnits"]["STORY-003"]["editedInSessions"],
        json!(["s1"])
    );
    assert!(
        session_markers(&after["workUnits"]["STORY-002"]).is_empty()
            && session_markers(&after["workUnits"]["STORY-004"]).is_empty(),
        "cards the session did not touch carry no session id"
    );

    // @step And the same commands run in the main checkout record no session id
    run_the_commands(&main, "STORY-002", "Made in main", "STORY-004")?;
    let after_main = board(&main)?;
    let main_card = card_titled(&after_main, "Made in main")?;
    for id in ["STORY-002", "STORY-004", main_card.as_str()] {
        let markers = session_markers(&after_main["workUnits"][id]);
        assert!(markers.is_empty(), "{id} carries {markers:?}");
    }
    Ok(())
}
