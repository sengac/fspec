#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Lifecycle Hooks Config — Loading, Merging & Compilation Tests
//!
//! Feature: spec/features/agent-lifecycle-hooks.feature
//!
//! Tests for HOOK-014: Config data model, two-level loading, merging, and regex compilation.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// These types will be created during implementation
// For now, the test won't compile — that IS the "failing test" for Rust
use codelet_core::lifecycle_hooks::load_lifecycle_hooks;

/// Helper to create a temp project dir with spec/fspec-hooks.json
fn write_project_hooks(dir: &TempDir, json: &str) -> PathBuf {
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    let path = spec_dir.join("fspec-hooks.json");
    fs::write(&path, json).unwrap();
    dir.path().to_path_buf()
}

/// Helper to create a temp user-level ~/.fspec/fspec-hooks.json
fn write_user_hooks(dir: &TempDir, json: &str) -> PathBuf {
    let fspec_dir = dir.path().join(".fspec");
    fs::create_dir_all(&fspec_dir).unwrap();
    let path = fspec_dir.join("fspec-hooks.json");
    fs::write(&path, json).unwrap();
    dir.path().to_path_buf()
}

// ===== Config Loading & Merging =====

/// @HOOK-014 Scenario: Load agent lifecycle hooks from project-level fspec-hooks.json
#[test]
fn test_load_project_level_hooks() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "setup-env", "command": "./hooks/setup.sh", "timeout": 30 }
                ]
            }
        }"#,
    );

    // @step Given a project-level spec/fspec-hooks.json with a "session_start" hook entry
    // (done above)

    // @step When the lifecycle hook engine initializes for a new session
    let result = load_lifecycle_hooks(Some(&project_path), None);

    // @step Then the engine should load and compile the session_start hook
    let engine = result.expect("should load successfully");
    let compiled = engine.expect("should not be None");

    // @step And the hook should be available for execution at session start
    assert_eq!(compiled.session_start.len(), 1);
    assert_eq!(compiled.session_start[0].name, "setup-env");
    assert_eq!(compiled.session_start[0].command, "./hooks/setup.sh");
    assert_eq!(compiled.session_start[0].timeout, 30);
}

/// @HOOK-014 Scenario: Load agent lifecycle hooks from user-level fspec-hooks.json
#[test]
fn test_load_user_level_hooks() {
    let user_dir = TempDir::new().unwrap();
    let user_home = write_user_hooks(
        &user_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "company-policy", "command": "./hooks/company.sh" }
                ]
            }
        }"#,
    );

    // @step Given a user-level ~/.fspec/fspec-hooks.json with a "session_start" hook entry
    // @step And no project-level spec/fspec-hooks.json exists
    // (no project dir created)

    // @step When the lifecycle hook engine initializes for a new session
    let result = load_lifecycle_hooks(None, Some(&user_home));

    // @step Then the engine should load and compile the session_start hook from the user-level config
    let engine = result.expect("should load successfully");
    let compiled = engine.expect("should not be None");
    assert_eq!(compiled.session_start.len(), 1);
    assert_eq!(compiled.session_start[0].name, "company-policy");
}

/// @HOOK-014 Scenario: Concatenate user-level and project-level hooks for the same event
#[test]
fn test_concatenate_user_and_project_hooks() {
    let user_dir = TempDir::new().unwrap();
    let user_home = write_user_hooks(
        &user_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "company-policy", "command": "./hooks/company.sh" }
                ]
            }
        }"#,
    );

    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "project-setup", "command": "./hooks/setup.sh" }
                ]
            }
        }"#,
    );

    // @step Given a user-level ~/.fspec/fspec-hooks.json with a "session_start" hook named "company-policy"
    // @step And a project-level spec/fspec-hooks.json with a "session_start" hook named "project-setup"
    // (done above)

    // @step When the lifecycle hook engine initializes for a new session
    let result = load_lifecycle_hooks(Some(&project_path), Some(&user_home));

    // @step Then both hooks should be compiled for the session_start event
    let engine = result.expect("should load successfully");
    let compiled = engine.expect("should not be None");
    assert_eq!(compiled.session_start.len(), 2);

    // @step And the user-level "company-policy" hook should execute before the project-level "project-setup" hook
    assert_eq!(compiled.session_start[0].name, "company-policy");
    assert_eq!(compiled.session_start[1].name, "project-setup");
}

/// @HOOK-014 Scenario: Coexistence of agent lifecycle events and fspec CLI events in same config
#[test]
fn test_coexistence_agent_and_cli_events() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "agent-hook", "command": "./hooks/agent.sh" }
                ],
                "pre-update-work-unit-status": [
                    { "name": "cli-hook", "command": "npm run lint", "blocking": true }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json containing both "session_start" and "pre-update-work-unit-status" hooks
    // (done above)

    // @step When the lifecycle hook engine initializes for a new session
    let result = load_lifecycle_hooks(Some(&project_path), None);

    // @step Then the engine should load only the "session_start" agent lifecycle event
    let engine = result.expect("should load successfully");
    let compiled = engine.expect("should not be None");
    assert_eq!(compiled.session_start.len(), 1);
    assert_eq!(compiled.session_start[0].name, "agent-hook");

    // @step And the engine should ignore the "pre-update-work-unit-status" fspec CLI event
    // CLI events are simply not included in CompiledLifecycleHooks — no field for them
}

/// @HOOK-014 Scenario: No agent lifecycle events configured returns None engine
#[test]
fn test_no_agent_events_returns_none() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "pre-update-work-unit-status": [
                    { "name": "lint", "command": "npm run lint", "blocking": true }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with only fspec CLI command hooks and no agent lifecycle events
    // @step And no user-level ~/.fspec/fspec-hooks.json exists
    // (done above)

    // @step When the lifecycle hook engine attempts to initialize
    let result = load_lifecycle_hooks(Some(&project_path), None);

    // @step Then the engine should return None
    let engine = result.expect("should not error");
    assert!(engine.is_none(), "engine should be None when no agent events configured");

    // @step And zero overhead should be added to the agent loop
    // (None means no engine instantiated — implicit zero overhead)
}

/// @HOOK-014 Scenario: Invalid regex matcher prevents engine creation
#[test]
fn test_invalid_regex_prevents_engine_creation() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "pre_tool_use": [
                    {
                        "matcher": "[invalid regex",
                        "hooks": [{ "command": "./hooks/check.sh" }]
                    }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "[invalid regex"
    // (done above)

    // @step When the lifecycle hook engine attempts to initialize
    let result = load_lifecycle_hooks(Some(&project_path), None);

    // @step Then the engine should return an error indicating the invalid regex pattern
    assert!(result.is_err(), "invalid regex should produce an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("regex") || err_msg.contains("invalid"),
        "error should mention regex: {err_msg}"
    );

    // @step And no hooks should be compiled
    // (error means no engine created)
}

/// @HOOK-014 Scenario: Respect global timeout setting from config
/// Additional coverage: global.shell field is captured and forwarded
#[test]
fn test_global_timeout_setting() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "global": {
                "timeout": 30,
                "shell": "bash -c"
            },
            "hooks": {
                "session_start": [
                    { "name": "setup", "command": "./hooks/setup.sh" }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with global timeout set to 30 seconds
    // @step And a "session_start" hook with no per-hook timeout override
    // (done above — hook has no explicit timeout)

    // @step When the session_start hook executes
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let engine = result.expect("should load").expect("should not be None");

    // @step Then the hook should use the 30 second global timeout
    assert_eq!(engine.global_timeout, 30);
    assert_eq!(
        engine.session_start[0].timeout, 30,
        "hook should inherit global timeout"
    );

    // Verify global.shell is captured
    assert_eq!(
        engine.global_shell.as_deref(),
        Some("bash -c"),
        "global_shell should be forwarded from config"
    );
}

/// @HOOK-014 Scenario: Config is compiled once at session creation not hot-reloaded
#[test]
fn test_config_compiled_once_not_hot_reloaded() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "initial", "command": "./hooks/initial.sh" }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "session_start" hook
    // @step And the lifecycle hook engine has been initialized for a session
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    // @step When the spec/fspec-hooks.json file is modified to add a "user_prompt_submit" hook
    let new_config = r#"{
        "hooks": {
            "session_start": [
                { "name": "initial", "command": "./hooks/initial.sh" }
            ],
            "user_prompt_submit": [
                { "name": "new-hook", "command": "./hooks/new.sh" }
            ]
        }
    }"#;
    let spec_dir = project_path.join("spec");
    fs::write(spec_dir.join("fspec-hooks.json"), new_config).unwrap();

    // @step Then the running session should not see the new user_prompt_submit hook
    // The compiled engine is immutable — changes to files don't affect it
    assert!(
        compiled.user_prompt_submit.is_empty(),
        "compiled engine should NOT see the new hook"
    );

    // @step And only a new session should pick up the config change
    let new_result = load_lifecycle_hooks(Some(&project_path), None);
    let new_compiled = new_result.expect("should load").expect("should not be None");
    assert_eq!(
        new_compiled.user_prompt_submit.len(),
        1,
        "new load should see the added hook"
    );
}

// ===== Hook Group Format (pre_tool_use / post_tool_use) =====

/// @HOOK-014 Scenario: pre_tool_use hook group with regex matcher filters by tool name
#[test]
fn test_pre_tool_use_regex_matcher_filters_tool_name() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "pre_tool_use": [
                    {
                        "matcher": "Bash",
                        "hooks": [{ "command": "./hooks/security.sh" }]
                    }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "Bash"
    // @step And a hook command that exits with code 0
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    assert_eq!(compiled.pre_tool_use.len(), 1);

    // @step When the agent invokes the "Bash" tool
    // @step Then the pre_tool_use hook should execute
    assert!(
        compiled.pre_tool_use[0].matcher.matches("Bash"),
        "matcher should match 'Bash'"
    );

    // @step When the agent invokes the "Read" tool
    // @step Then the pre_tool_use hook should not execute
    assert!(
        !compiled.pre_tool_use[0].matcher.matches("Read"),
        "matcher should NOT match 'Read'"
    );
}

/// @HOOK-014 Scenario: pre_tool_use hook group with empty matcher matches all tools
#[test]
fn test_pre_tool_use_empty_matcher_matches_all() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "pre_tool_use": [
                    {
                        "hooks": [{ "command": "./hooks/log.sh" }]
                    }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with no matcher
    // @step And a hook command that exits with code 0
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    assert_eq!(compiled.pre_tool_use.len(), 1);

    // @step When the agent invokes the "Bash" tool
    // @step Then the pre_tool_use hook should execute
    assert!(
        compiled.pre_tool_use[0].matcher.matches("Bash"),
        "empty matcher should match 'Bash'"
    );

    // @step When the agent invokes the "Write" tool
    // @step Then the pre_tool_use hook should also execute
    assert!(
        compiled.pre_tool_use[0].matcher.matches("Write"),
        "empty matcher should match 'Write'"
    );
}

/// @HOOK-014 Scenario: Matcher regex is anchored with full-match semantics
#[test]
fn test_matcher_regex_anchored_full_match() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "pre_tool_use": [
                    {
                        "matcher": "Bash",
                        "hooks": [{ "command": "./hooks/check.sh" }]
                    }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "Bash"
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    // @step When the agent invokes a tool named "BashExtended"
    // @step Then the pre_tool_use hook should not execute because "BashExtended" does not match "^(?:Bash)$"
    assert!(
        !compiled.pre_tool_use[0].matcher.matches("BashExtended"),
        "anchored regex should NOT match 'BashExtended' — only exact 'Bash'"
    );

    // Also verify exact match works
    assert!(
        compiled.pre_tool_use[0].matcher.matches("Bash"),
        "anchored regex should match exact 'Bash'"
    );
}

// ===== HookDefinition Format (session/prompt/notification events) =====

/// @HOOK-014 Scenario: session_start hooks use HookDefinition format
#[test]
fn test_session_start_hook_definition_format() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "setup-env", "command": "./hooks/setup.sh", "timeout": 30 }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "session_start" entry using HookDefinition format
    // (done above, matching the doc string from the feature file)

    // @step When the lifecycle hook engine initializes for a new session
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    // @step Then the "setup-env" hook should be compiled with a 30 second timeout
    assert_eq!(compiled.session_start.len(), 1);
    assert_eq!(compiled.session_start[0].name, "setup-env");
    assert_eq!(compiled.session_start[0].command, "./hooks/setup.sh");
    assert_eq!(compiled.session_start[0].timeout, 30);
}

/// @HOOK-014 Scenario: user_prompt_submit hooks use HookDefinition format
#[test]
fn test_user_prompt_submit_hook_definition_format() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "user_prompt_submit": [
                    { "name": "policy-check", "command": "./hooks/policy.sh", "blocking": true }
                ]
            }
        }"#,
    );

    // @step Given a spec/fspec-hooks.json with a "user_prompt_submit" entry using HookDefinition format
    // (done above)

    // @step When a user submits a prompt
    let result = load_lifecycle_hooks(Some(&project_path), None);
    let compiled = result.expect("should load").expect("should not be None");

    // @step Then the user_prompt_submit hooks should execute sequentially with JSON payload on stdin
    assert_eq!(compiled.user_prompt_submit.len(), 1);
    assert_eq!(compiled.user_prompt_submit[0].name, "policy-check");
    assert_eq!(compiled.user_prompt_submit[0].command, "./hooks/policy.sh");
    assert!(compiled.user_prompt_submit[0].blocking);
}

/// Additional: Verify malformed JSON config file produces an error, not silent ignore
#[test]
fn test_malformed_json_config_produces_error() {
    let project_dir = TempDir::new().unwrap();
    let spec_dir = project_dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("fspec-hooks.json"), "{ invalid json }").unwrap();

    let result = load_lifecycle_hooks(Some(project_dir.path()), None);

    assert!(result.is_err(), "malformed JSON should produce an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("JSON") || err_msg.contains("parse"),
        "error should mention JSON parsing: {err_msg}"
    );
}

/// Additional: Verify global.shell defaults to None when not set
#[test]
fn test_global_shell_defaults_to_none() {
    let project_dir = TempDir::new().unwrap();
    let project_path = write_project_hooks(
        &project_dir,
        r#"{
            "hooks": {
                "session_start": [
                    { "name": "setup", "command": "./hooks/setup.sh" }
                ]
            }
        }"#,
    );

    let result = load_lifecycle_hooks(Some(&project_path), None);
    let engine = result.expect("should load").expect("should not be None");

    assert!(
        engine.global_shell.is_none(),
        "global_shell should be None when not configured"
    );
}
