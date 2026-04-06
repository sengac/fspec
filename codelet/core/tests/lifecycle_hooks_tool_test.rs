#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Lifecycle Hooks — Tool Use Short-Circuit Tests
//!
//! Feature: spec/features/agent-lifecycle-hooks.feature
//!
//! Tests for HOOK-017: pre_tool_use short-circuit behavior.
//! Verifies that Allow/Deny decisions stop evaluation of remaining
//! hook groups, while Continue passes through to the next group.

use std::fs;

use regex::Regex;
use serde_json::json;
use tempfile::TempDir;

use codelet_core::lifecycle_hooks::{
    CompiledHookCommand, CompiledHookGroup, CompiledLifecycleHooks, HookContext, HookMatcher,
    PreToolHookDecision, run_pre_tool,
};

// ===== Helpers =====

fn make_context(workspace: &std::path::Path) -> HookContext {
    HookContext {
        session_id: "session-017-test".to_string(),
        cwd: workspace.to_string_lossy().to_string(),
        transcript_path: workspace
            .join("transcript.json")
            .to_string_lossy()
            .to_string(),
    }
}

/// Create a shell script that writes a marker file, then outputs JSON with the given permissionDecision.
fn create_marker_script(dir: &std::path::Path, name: &str, decision: &str) -> String {
    let marker_file = dir.join(format!("{name}.marker"));
    let script = dir.join(format!("{name}.sh"));
    let json_output = format!(
        r#"{{ "hookSpecificOutput": {{ "permissionDecision": "{decision}" }} }}"#
    );
    let script_content = format!(
        "#!/bin/sh\ntouch '{}'\necho '{}'\n",
        marker_file.to_string_lossy(),
        json_output,
    );
    fs::write(&script, script_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    script.to_string_lossy().to_string()
}

/// Build CompiledLifecycleHooks with two pre_tool_use groups matching "Bash".
fn make_two_group_hooks(first_cmd: &str, second_cmd: &str) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![
            CompiledHookGroup {
                matcher: HookMatcher::Pattern(Regex::new("^(?:Bash)$").unwrap()),
                commands: vec![CompiledHookCommand {
                    command: first_cmd.to_string(),
                    timeout: 10,
                }],
            },
            CompiledHookGroup {
                matcher: HookMatcher::Pattern(Regex::new("^(?:Bash)$").unwrap()),
                commands: vec![CompiledHookCommand {
                    command: second_cmd.to_string(),
                    timeout: 10,
                }],
            },
        ],
        post_tool_use: vec![],
    }
}

// ===== Scenario: pre_tool_use short-circuits on Allow decision =====

#[tokio::test]
async fn pre_tool_use_short_circuits_on_allow() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let ctx = make_context(workspace);

    // @step Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
    //   | group  | matcher | decision |
    //   | first  | Bash    | Allow    |
    //   | second | Bash    | Deny     |
    let first_cmd = create_marker_script(workspace, "first", "allow");
    let second_cmd = create_marker_script(workspace, "second", "deny");
    let hooks = make_two_group_hooks(&first_cmd, &second_cmd);

    // @step When the agent invokes the "Bash" tool
    let tool_input = json!({"command": "echo hello"});
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &tool_input).await;

    // @step Then only the first hook group should execute
    assert!(
        workspace.join("first.marker").exists(),
        "First hook group should have executed"
    );
    assert!(
        !workspace.join("second.marker").exists(),
        "Second hook group should NOT have executed (short-circuited)"
    );

    // @step And the tool should be allowed (second group's Deny is never reached)
    assert_eq!(outcome.decision, PreToolHookDecision::Allow);
}

// ===== Scenario: pre_tool_use short-circuits on Deny decision =====

#[tokio::test]
async fn pre_tool_use_short_circuits_on_deny() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let ctx = make_context(workspace);

    // @step Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
    //   | group  | matcher | decision |
    //   | first  | Bash    | Deny     |
    //   | second | Bash    | Allow    |
    let first_cmd = create_marker_script(workspace, "first", "deny");
    let second_cmd = create_marker_script(workspace, "second", "allow");
    let hooks = make_two_group_hooks(&first_cmd, &second_cmd);

    // @step When the agent invokes the "Bash" tool
    let tool_input = json!({"command": "echo hello"});
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &tool_input).await;

    // @step Then only the first hook group should execute
    assert!(
        workspace.join("first.marker").exists(),
        "First hook group should have executed"
    );
    assert!(
        !workspace.join("second.marker").exists(),
        "Second hook group should NOT have executed (short-circuited)"
    );

    // @step And the tool call should be denied (second group's Allow is never reached)
    assert_eq!(outcome.decision, PreToolHookDecision::Deny);
}

// ===== Scenario: pre_tool_use Continue does not short-circuit =====

#[tokio::test]
async fn pre_tool_use_continue_does_not_short_circuit() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let ctx = make_context(workspace);

    // @step Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
    //   | group  | matcher | decision |
    //   | first  | Bash    | Continue |
    //   | second | Bash    | Deny     |
    //
    // Continue = exit code 0, no permissionDecision JSON → engine interprets as Continue
    let marker_file = workspace.join("first.marker");
    let first_script = workspace.join("first.sh");
    let first_script_content = format!(
        "#!/bin/sh\ntouch '{}'\nexit 0\n",
        marker_file.to_string_lossy()
    );
    fs::write(&first_script, first_script_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&first_script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let first_cmd = first_script.to_string_lossy().to_string();

    let second_cmd = create_marker_script(workspace, "second", "deny");
    let hooks = make_two_group_hooks(&first_cmd, &second_cmd);

    // @step When the agent invokes the "Bash" tool
    let tool_input = json!({"command": "echo hello"});
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &tool_input).await;

    // @step Then both hook groups should execute
    assert!(
        workspace.join("first.marker").exists(),
        "First hook group should have executed"
    );
    assert!(
        workspace.join("second.marker").exists(),
        "Second hook group should have executed (Continue does not short-circuit)"
    );

    // @step And the tool call should be denied by the second group
    assert_eq!(outcome.decision, PreToolHookDecision::Deny);
}

// ===== Scenario: pre_tool_use all groups return Continue falls through =====

#[tokio::test]
async fn pre_tool_use_all_continue_falls_through() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let ctx = make_context(workspace);

    // @step Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
    //   | group  | matcher | decision |
    //   | first  | Bash    | Continue |
    //   | second | Bash    | Continue |
    //
    // Continue = exit code 0, no permissionDecision JSON → engine interprets as Continue
    let marker_file_a = workspace.join("first.marker");
    let first_script = workspace.join("first.sh");
    let first_script_content = format!(
        "#!/bin/sh\ntouch '{}'\nexit 0\n",
        marker_file_a.to_string_lossy()
    );
    fs::write(&first_script, first_script_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&first_script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let first_cmd = first_script.to_string_lossy().to_string();

    let marker_file_b = workspace.join("second.marker");
    let second_script = workspace.join("second.sh");
    let second_script_content = format!(
        "#!/bin/sh\ntouch '{}'\nexit 0\n",
        marker_file_b.to_string_lossy()
    );
    fs::write(&second_script, second_script_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&second_script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let second_cmd = second_script.to_string_lossy().to_string();

    let hooks = make_two_group_hooks(&first_cmd, &second_cmd);

    // @step When the agent invokes the "Bash" tool
    let tool_input = json!({"command": "echo hello"});
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &tool_input).await;

    // @step Then both hook groups should execute
    assert!(
        workspace.join("first.marker").exists(),
        "First hook group should have executed"
    );
    assert!(
        workspace.join("second.marker").exists(),
        "Second hook group should have executed"
    );

    // @step And the final decision should be Continue (fall through to normal permission checks)
    assert_eq!(outcome.decision, PreToolHookDecision::Continue);
}
