#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/hook-condition-evaluation.feature
//
// HOOK-018: a command hook's condition (tags, prefix, epic, estimate range)
// decides whether it runs for the command's work unit. "Evaluate" uses
// hook_matches on the work unit read from the board; "included in execution"
// runs the command's pre-hooks through run_command_hooks.

use std::fs;
use std::path::Path;

use codelet_fspec_core::command_hooks::{
    hook_matches, load_hook_config, run_command_hooks, CommandHook, HookPhase, HookWorkUnit,
};
use serde_json::{json, Value};
use tempfile::TempDir;

/// A project whose board holds `unit` (id plus optional tags/epic/estimate)
/// and whose spec/fspec-hooks.json has one `pre-show-work-unit` hook named
/// "guard" with `condition` (or none).
fn project(unit: Option<Value>, condition: Option<Value>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).unwrap();
    let mut work_units = serde_json::Map::new();
    let mut backlog = Vec::new();
    if let Some(mut unit) = unit {
        let id = unit["id"].as_str().unwrap().to_string();
        unit["title"] = json!("Unit");
        unit["status"] = json!("backlog");
        unit["type"] = json!("story");
        unit["createdAt"] = json!("2026-11-07T00:00:00.000Z");
        unit["updatedAt"] = json!("2026-11-07T00:00:00.000Z");
        backlog.push(json!(id));
        work_units.insert(id, unit);
    }
    let board = json!({
        "meta": {"version": "1.0.0", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": work_units,
        "states": {"backlog": backlog, "specifying": [], "testing": [], "implementing": [],
                   "validating": [], "done": [], "blocked": []}
    });
    fs::write(spec.join("work-units.json"), board.to_string()).unwrap();
    let mut hook = json!({"name": "guard", "command": "echo guard-ran"});
    if let Some(condition) = condition {
        hook["condition"] = condition;
    }
    let config = json!({"hooks": {"pre-show-work-unit": [hook]}});
    fs::write(spec.join("fspec-hooks.json"), config.to_string()).unwrap();
    dir
}

fn the_hook(root: &Path) -> CommandHook {
    load_hook_config(root)
        .unwrap()
        .hooks_for("pre-show-work-unit")[0]
        .clone()
}

fn work_unit(root: &Path, id: &str) -> Option<HookWorkUnit> {
    HookWorkUnit::load(root, id).unwrap_or_else(|e| panic!("board should load: {e}"))
}

fn executed(root: &Path, id: Option<&str>) -> Vec<String> {
    let outcome = run_command_hooks(root, "show-work-unit", id, HookPhase::Pre)
        .unwrap_or_else(|e| panic!("hooks should run: {e}"));
    outcome
        .results
        .iter()
        .map(|r| r.hook_name.clone())
        .collect()
}

#[test]
fn hook_with_no_condition_always_matches() {
    // @step Given I have a hook with no condition
    // @step And I have a work unit context "AUTH-001"
    let dir = project(Some(json!({"id": "AUTH-001"})), None);
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_tag_condition_matches_work_unit_with_matching_tag() {
    // @step Given I have a hook with condition tags ["@security"]
    // @step And I have a work unit "AUTH-001" with tags ["@security", "@critical"]
    let dir = project(
        Some(json!({"id": "AUTH-001", "tags": ["@security", "@critical"]})),
        Some(json!({"tags": ["@security"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_tag_condition_does_not_match_work_unit_without_tag() {
    // @step Given I have a hook with condition tags ["@security"]
    // @step And I have a work unit "DASH-001" with tags ["@ui", "@critical"]
    let dir = project(
        Some(json!({"id": "DASH-001", "tags": ["@ui", "@critical"]})),
        Some(json!({"tags": ["@security"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "DASH-001").as_ref(),
    );
    // @step Then the hook should not match
    assert!(!matched);
    // @step And the hook should be excluded from execution
    assert!(executed(dir.path(), Some("DASH-001")).is_empty());
}

#[test]
fn hook_with_prefix_condition_matches_work_unit_with_matching_prefix() {
    // @step Given I have a hook with condition prefix ["AUTH", "SEC"]
    // @step And I have a work unit "AUTH-001"
    let dir = project(
        Some(json!({"id": "AUTH-001"})),
        Some(json!({"prefix": ["AUTH", "SEC"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_prefix_condition_does_not_match_work_unit_with_different_prefix() {
    // @step Given I have a hook with condition prefix ["AUTH", "SEC"]
    // @step And I have a work unit "DASH-001"
    let dir = project(
        Some(json!({"id": "DASH-001"})),
        Some(json!({"prefix": ["AUTH", "SEC"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "DASH-001").as_ref(),
    );
    // @step Then the hook should not match
    assert!(!matched);
    // @step And the hook should be excluded from execution
    assert!(executed(dir.path(), Some("DASH-001")).is_empty());
}

#[test]
fn hook_with_epic_condition_matches_work_unit_in_that_epic() {
    // @step Given I have a hook with condition epic "user-management"
    // @step And I have a work unit "AUTH-001" in epic "user-management"
    let dir = project(
        Some(json!({"id": "AUTH-001", "epic": "user-management"})),
        Some(json!({"epic": "user-management"})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_estimate_range_matches_work_unit_within_range() {
    // @step Given I have a hook with condition estimateMin 5 and estimateMax 13
    // @step And I have a work unit "AUTH-001" with estimate 8
    let dir = project(
        Some(json!({"id": "AUTH-001", "estimate": 8})),
        Some(json!({"estimateMin": 5, "estimateMax": 13})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_multiple_conditions_uses_and_logic() {
    // @step Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
    // @step And I have a work unit "AUTH-001" with tags ["@security"]
    let dir = project(
        Some(json!({"id": "AUTH-001", "tags": ["@security"]})),
        Some(json!({"tags": ["@security"], "prefix": ["AUTH"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "AUTH-001").as_ref(),
    );
    // @step Then the hook should match because both conditions are met
    assert!(matched);
    // @step And the hook should be included in execution
    assert_eq!(executed(dir.path(), Some("AUTH-001")), vec!["guard"]);
}

#[test]
fn hook_with_multiple_conditions_fails_if_any_condition_is_not_met() {
    // @step Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
    // @step And I have a work unit "DASH-001" with tags ["@security"]
    let dir = project(
        Some(json!({"id": "DASH-001", "tags": ["@security"]})),
        Some(json!({"tags": ["@security"], "prefix": ["AUTH"]})),
    );
    // @step When I evaluate if the hook should run
    let matched = hook_matches(
        &the_hook(dir.path()),
        work_unit(dir.path(), "DASH-001").as_ref(),
    );
    // @step Then the hook should not match because prefix does not match
    assert!(!matched);
    // @step And the hook should be excluded from execution
    assert!(executed(dir.path(), Some("DASH-001")).is_empty());
}

#[test]
fn context_without_work_unit_id_only_matches_unconditional_hooks() {
    // @step Given I have a hook with condition tags ["@security"]
    let dir = project(
        Some(json!({"id": "AUTH-001", "tags": ["@security"]})),
        Some(json!({"tags": ["@security"]})),
    );
    // @step And I have a context with no work unit ID
    let no_work_unit: Option<&HookWorkUnit> = None;
    // @step When I evaluate if the hook should run
    let matched = hook_matches(&the_hook(dir.path()), no_work_unit);
    // @step Then the hook should not match
    assert!(!matched);
    // @step And the hook should be excluded from execution
    assert!(executed(dir.path(), None).is_empty());
    let unconditional = project(Some(json!({"id": "AUTH-001"})), None);
    assert_eq!(executed(unconditional.path(), None), vec!["guard"]);
}
