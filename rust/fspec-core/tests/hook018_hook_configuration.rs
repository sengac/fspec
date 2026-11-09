#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/hook-configuration-schema-and-validation.feature
//
// HOOK-018: loading the command hooks of spec/fspec-hooks.json
// (codelet_fspec_core::command_hooks::load_hook_config).

use std::fs;
use std::path::Path;

use codelet_fspec_core::command_hooks::{load_hook_config, HookConfig};
use tempfile::TempDir;

fn write_hooks(root: &Path, raw: &str) {
    fs::create_dir_all(root.join("spec")).unwrap();
    fs::write(root.join("spec/fspec-hooks.json"), raw).unwrap();
}

fn write_script(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
}

fn load(root: &Path) -> HookConfig {
    load_hook_config(root).unwrap_or_else(|e| panic!("config should load: {e}"))
}

#[test]
fn load_valid_hook_configuration_with_single_hook() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"hooks":{"post-implementing":[{"name":"lint","command":"spec/hooks/lint.sh","blocking":true}]}}"#,
    );
    write_script(dir.path(), "spec/hooks/lint.sh");
    // @step When I load the hook configuration
    let result = load_hook_config(dir.path());
    // @step Then the configuration should be valid
    let config = result.unwrap_or_else(|e| panic!("config should load: {e}"));
    // @step And the hook "lint" should be registered for event "post-implementing"
    let hooks = config.hooks_for("post-implementing");
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].name, "lint");
    assert_eq!(hooks[0].command, "spec/hooks/lint.sh");
    // @step And the hook should have blocking set to true
    assert!(hooks[0].blocking);
    // @step And the hook should have timeout set to 60 (default)
    assert_eq!(hooks[0].timeout_secs, 60);
}

#[test]
fn load_hook_configuration_with_timeout_override() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"hooks":{"post-testing":[{"name":"e2e-tests","command":"test.sh","timeout":300}]}}"#,
    );
    // @step When I load the hook configuration
    let config = load(dir.path());
    // @step Then the hook "e2e-tests" should have timeout set to 300
    let hooks = config.hooks_for("post-testing");
    assert_eq!(hooks[0].name, "e2e-tests");
    assert_eq!(hooks[0].timeout_secs, 300);
    assert!(!hooks[0].blocking, "blocking defaults to false");
}

#[test]
fn load_hook_configuration_with_conditions() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"hooks":{"post-implementing":[{"name":"security","command":"audit.sh","condition":{"tags":["@security"],"prefix":["AUTH"]}}]}}"#,
    );
    // @step When I load the hook configuration
    let config = load(dir.path());
    let hook = &config.hooks_for("post-implementing")[0];
    let condition = hook.condition.as_ref().expect("condition loaded");
    // @step Then the hook "security" should have condition tags set to ["@security"]
    assert_eq!(hook.name, "security");
    assert_eq!(condition.tags, vec!["@security".to_string()]);
    // @step And the hook "security" should have condition prefix set to ["AUTH"]
    assert_eq!(condition.prefix, vec!["AUTH".to_string()]);
}

#[test]
fn reject_invalid_json_configuration() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with invalid JSON content
    write_hooks(dir.path(), "{ \"hooks\": { invalid json }");
    // @step When I try to load the hook configuration
    let result = load_hook_config(dir.path());
    // @step Then loading should fail
    let message = match result {
        Ok(config) => panic!("invalid JSON must not load: {config:?}"),
        Err(err) => err.to_string(),
    };
    // @step And the error message should contain "Invalid JSON"
    assert!(message.contains("Invalid JSON"), "{message}");
    // @step And the error message should name "spec/fspec-hooks.json"
    assert!(message.contains("spec/fspec-hooks.json"), "{message}");
}

#[test]
fn reject_configuration_with_non_existent_hook_command() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"hooks":{"post-implementing":[{"name":"missing","command":"spec/hooks/missing.sh"}]}}"#,
    );
    // @step And the file "spec/hooks/missing.sh" does not exist
    assert!(!dir.path().join("spec/hooks/missing.sh").exists());
    // @step When I try to load the hook configuration
    let result = load_hook_config(dir.path());
    // @step Then loading should fail
    let message = match result {
        Ok(config) => panic!("a missing script must not load: {config:?}"),
        Err(err) => err.to_string(),
    };
    // @step And the error message should contain "Hook command not found: spec/hooks/missing.sh"
    assert!(
        message.contains("Hook command not found: spec/hooks/missing.sh"),
        "{message}"
    );
}

#[test]
fn load_configuration_with_global_defaults() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"global":{"timeout":120,"shell":"/bin/bash"},"hooks":{"post-implementing":[{"name":"lint","command":"spec/hooks/lint.sh"}]}}"#,
    );
    write_script(dir.path(), "spec/hooks/lint.sh");
    // @step When I load the hook configuration
    let config = load(dir.path());
    // @step Then the global timeout should be set to 120
    assert_eq!(config.global_timeout_secs, Some(120));
    // @step And the global shell should be set to "/bin/bash"
    assert_eq!(config.shell.as_deref(), Some("/bin/bash"));
    // @step And the hook "lint" should use the global timeout of 120
    assert_eq!(config.hooks_for("post-implementing")[0].timeout_secs, 120);
}

#[test]
fn missing_hook_configuration_means_no_command_hooks() {
    let dir = TempDir::new().unwrap();
    // @step Given the project has no file "spec/fspec-hooks.json"
    assert!(!dir.path().join("spec/fspec-hooks.json").exists());
    // @step When I load the hook configuration
    let config = load(dir.path());
    // @step Then the configuration should have no command hooks
    assert!(config.events.is_empty(), "{config:?}");
}

#[test]
fn agent_lifecycle_entries_are_not_read_as_command_hooks() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a file "spec/fspec-hooks.json" with content:
    write_hooks(
        dir.path(),
        r#"{"hooks":{"pre_tool_use":[{"matcher":"Bash","hooks":[{"command":"echo tool"}]}],"post-list-work-units":[{"name":"notify","command":"echo listed"}]}}"#,
    );
    // @step When I load the hook configuration
    let result = load_hook_config(dir.path());
    // @step Then the configuration should be valid
    let config = result.unwrap_or_else(|e| panic!("config should load: {e}"));
    // @step And the only command-hook event should be "post-list-work-units"
    let events: Vec<&str> = config.events.keys().map(String::as_str).collect();
    assert_eq!(events, vec!["post-list-work-units"]);
}
