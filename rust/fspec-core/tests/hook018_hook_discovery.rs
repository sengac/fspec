#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/hook-discovery-and-event-naming.feature
//
// HOOK-018: discovering the command hooks of an event and naming a
// command's pre-/post- events (codelet_fspec_core::command_hooks).

use std::fs;
use std::path::Path;

use codelet_fspec_core::command_hooks::{
    discover_hooks, event_names, load_hook_config, HookConfig,
};
use tempfile::TempDir;

fn config_with(root: &Path, raw: &str) -> HookConfig {
    fs::create_dir_all(root.join("spec")).unwrap();
    fs::write(root.join("spec/fspec-hooks.json"), raw).unwrap();
    load_hook_config(root).unwrap_or_else(|e| panic!("config should load: {e}"))
}

fn names(hooks: &[codelet_fspec_core::command_hooks::CommandHook]) -> Vec<&str> {
    hooks.iter().map(|h| h.name.as_str()).collect()
}

const POST_IMPLEMENTING: &str = r#"{"hooks":{"post-implementing":[
  {"name":"lint","command":"echo lint"},
  {"name":"notify","command":"echo notify"}]}}"#;

#[test]
fn discover_hooks_for_specific_event() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with hooks for "post-implementing"
    let config = config_with(dir.path(), POST_IMPLEMENTING);
    // @step When I discover hooks for event "post-implementing"
    let hooks = discover_hooks(&config, "post-implementing");
    // @step Then I should receive an array of matching hooks
    assert_eq!(hooks.len(), 2);
    // @step And the hooks should be returned in config order
    assert_eq!(names(hooks), vec!["lint", "notify"]);
}

#[test]
fn discover_hooks_for_non_existent_event() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration
    let config = config_with(dir.path(), POST_IMPLEMENTING);
    // @step When I discover hooks for event "pre-xyz"
    let hooks = discover_hooks(&config, "pre-xyz");
    // @step Then I should receive an empty array
    assert!(hooks.is_empty());
    // @step And no error should be returned
    // discover_hooks returns a slice, never an error: an unknown event is empty.
}

#[test]
fn generate_event_names_from_command_name() {
    // @step Given the command name is "update-work-unit-status"
    let command = "update-work-unit-status";
    // @step When I generate event names for this command
    let names = event_names(command);
    // @step Then the pre-event name should be "pre-update-work-unit-status"
    assert_eq!(names.pre, "pre-update-work-unit-status");
    // @step And the post-event name should be "post-update-work-unit-status"
    assert_eq!(names.post, "post-update-work-unit-status");
}

#[test]
fn event_names_are_case_sensitive() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with hooks for "post-implementing"
    let config = config_with(dir.path(), POST_IMPLEMENTING);
    // @step When I discover hooks for event "post-Implementing"
    let hooks = discover_hooks(&config, "post-Implementing");
    // @step Then I should receive an empty array
    assert!(hooks.is_empty());
    // @step And the hooks for "post-implementing" should not match
    assert_eq!(discover_hooks(&config, "post-implementing").len(), 2);
}

#[test]
fn multiple_hooks_for_same_event() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with multiple hooks for "post-testing"
    // @step And the hooks are named "unit-tests", "integration-tests", "e2e-tests"
    let config = config_with(
        dir.path(),
        r#"{"hooks":{"post-testing":[
          {"name":"unit-tests","command":"echo unit"},
          {"name":"integration-tests","command":"echo integration"},
          {"name":"e2e-tests","command":"echo e2e"}]}}"#,
    );
    // @step When I discover hooks for event "post-testing"
    let hooks = discover_hooks(&config, "post-testing");
    // @step Then I should receive all three hooks
    assert_eq!(hooks.len(), 3);
    // @step And the hooks should be in the order they appear in config
    assert_eq!(
        names(hooks),
        vec!["unit-tests", "integration-tests", "e2e-tests"]
    );
}
