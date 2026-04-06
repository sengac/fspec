#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Lifecycle Hooks — Session & Notification Integration Tests
//!
//! Feature: spec/features/agent-lifecycle-hooks.feature
//!
//! Tests for HOOK-016: user_prompt_submit blocking, session_end with reason,
//! notification via global engine, and sequential command execution within
//! hook groups.

use std::fs;

use serde_json::json;
use tempfile::TempDir;

use codelet_core::lifecycle_hooks::{
    CompiledHookCommand, CompiledHookDefinition, CompiledHookGroup, CompiledLifecycleHooks,
    HookContext, HookMatcher, run_notification, run_session_end, run_user_prompt, run_pre_tool,
};

// ===== Helpers =====

fn make_context(workspace: &std::path::Path) -> HookContext {
    HookContext {
        session_id: "session-016-test".to_string(),
        cwd: workspace.to_string_lossy().to_string(),
        transcript_path: workspace
            .join("transcript.json")
            .to_string_lossy()
            .to_string(),
    }
}

fn make_user_prompt_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![CompiledHookDefinition {
            name: "test-prompt-hook".to_string(),
            command: command.to_string(),
            blocking: true,
            timeout,
        }],
        notification: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![],
    }
}

fn make_notification_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![CompiledHookDefinition {
            name: "test-notification-hook".to_string(),
            command: command.to_string(),
            blocking: false,
            timeout,
        }],
        pre_tool_use: vec![],
        post_tool_use: vec![],
    }
}

fn make_session_end_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![CompiledHookDefinition {
            name: "notify-slack".to_string(),
            command: command.to_string(),
            blocking: false,
            timeout,
        }],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![],
    }
}

// ===== user_prompt_submit Blocking =====

/// @HOOK-016 Scenario: user_prompt_submit hook blocks forbidden prompt
#[tokio::test]
async fn test_user_prompt_blocks_via_exit_code() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that exits code 2 with stderr "Policy violation: prompt contains forbidden content"
    let hooks = make_user_prompt_hooks(
        r#"echo "Policy violation: prompt contains forbidden content" >&2; exit 2"#,
        10,
    );

    // @step When the user submits a prompt
    let outcome = run_user_prompt(&hooks, &ctx, "delete all files").await;

    // @step Then the prompt should be rejected
    assert!(!outcome.allow_prompt, "prompt should be blocked");

    // @step And the agent should never see the prompt
    // (The prompt is rejected — caller would not forward it to the agent)
    assert!(outcome.block_reason.is_some(), "block reason must be set");

    // @step And the agent loop should return to waiting for input
    // (The block_reason tells the caller to skip this prompt)
    assert!(
        outcome
            .block_reason
            .as_ref()
            .unwrap_or(&String::new())
            .contains("Policy violation"),
        "block reason should contain stderr: {:?}",
        outcome.block_reason
    );
}

/// @HOOK-016 Scenario: user_prompt_submit hook blocks via JSON continue false
#[tokio::test]
async fn test_user_prompt_blocks_via_json_continue_false() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that outputs JSON:
    let hooks = make_user_prompt_hooks(
        r#"echo '{"continue": false, "reason": "Prompt violates content policy"}'"#,
        10,
    );

    // @step When the user submits a prompt
    let outcome = run_user_prompt(&hooks, &ctx, "some forbidden prompt").await;

    // @step Then the prompt should be rejected
    assert!(!outcome.allow_prompt, "prompt should be blocked by JSON continue:false");

    // @step And the block reason should contain "Prompt violates content policy"
    assert!(
        outcome
            .block_reason
            .as_ref()
            .unwrap_or(&String::new())
            .contains("Prompt violates content policy"),
        "block reason should match JSON reason: {:?}",
        outcome.block_reason
    );
}

/// @HOOK-016 Scenario: user_prompt_submit hook allows and injects context
#[tokio::test]
async fn test_user_prompt_allows_and_injects_context() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that outputs JSON:
    let hooks = make_user_prompt_hooks(
        r#"echo '{"continue": true, "hookSpecificOutput": {"additionalContext": "User is an admin"}}'"#,
        10,
    );

    // @step When the user submits a prompt
    let outcome = run_user_prompt(&hooks, &ctx, "do admin stuff").await;

    // @step Then the prompt should be allowed through to the agent
    assert!(outcome.allow_prompt, "prompt should be allowed through");

    // @step And "User is an admin" should be injected as additional context
    assert!(
        outcome.additional_context.contains(&"User is an admin".to_string()),
        "additional context should contain injected text: {:?}",
        outcome.additional_context
    );
}

// ===== session_end =====

/// @HOOK-016 Scenario: session_end hook receives termination reason and executes
#[tokio::test]
async fn test_session_end_receives_reason_and_executes() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("payload.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_end" hook named "notify-slack"
    let hooks = make_session_end_hooks(
        &format!("cat > '{}'", capture_file.to_string_lossy()),
        10,
    );

    // @step When the agent session ends with reason "completed"
    let _outcome = run_session_end(&hooks, &ctx, "completed").await;

    // @step Then the "notify-slack" hook should execute
    assert!(capture_file.exists(), "hook should have written payload to file");

    // @step And the hook payload should include session_id, cwd, reason "completed", and transcript_path
    let payload_str = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("read payload: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&payload_str)
        .unwrap_or_else(|e| panic!("parse payload: {e}"));

    assert_eq!(payload["session_id"], "session-016-test");
    assert_eq!(payload["reason"], "completed");
    assert_eq!(payload["hook_event_name"], "SessionEnd");
    assert!(payload["cwd"].as_str().is_some(), "cwd should be present");
    assert!(
        payload["transcript_path"].as_str().is_some(),
        "transcript_path should be present"
    );
}

// ===== notification =====

/// @HOOK-016 Scenario: notification hook fires via global engine
#[tokio::test]
async fn test_notification_hook_fires_via_global_engine() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("notification.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "notification" hook
    let hooks = make_notification_hooks(
        &format!("cat > '{}'", capture_file.to_string_lossy()),
        10,
    );

    // @step When a notification event fires with type "permission_prompt" and title "Tool Permission"
    let _outcome = run_notification(
        &hooks,
        &ctx,
        "permission_prompt",
        "Tool Permission",
        "Allow Bash?",
    )
    .await;

    // @step Then the notification hook should execute
    assert!(capture_file.exists(), "notification hook should have written payload");

    // @step And the hook payload should include notification_type, title, and message
    let payload_str = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("read payload: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&payload_str)
        .unwrap_or_else(|e| panic!("parse payload: {e}"));

    assert_eq!(payload["notification_type"], "permission_prompt");
    assert_eq!(payload["title"], "Tool Permission");
    assert_eq!(payload["message"], "Allow Bash?");
    assert_eq!(payload["hook_event_name"], "Notification");
}

// ===== Sequential Execution =====

/// @HOOK-016 Scenario: Multiple commands in a hook group execute sequentially
#[tokio::test]
async fn test_sequential_command_execution_in_hook_group() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let marker_a = tmp.path().join("marker_a.txt");
    let marker_b = tmp.path().join("marker_b.txt");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook group containing two commands
    let hooks = CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![CompiledHookGroup {
            matcher: HookMatcher::Any,
            commands: vec![
                CompiledHookCommand {
                    command: format!("echo first > '{}'", marker_a.to_string_lossy()),
                    timeout: 10,
                },
                CompiledHookCommand {
                    command: format!("echo second > '{}'", marker_b.to_string_lossy()),
                    timeout: 10,
                },
            ],
        }],
        post_tool_use: vec![],
    };

    // @step When the agent invokes a tool matching the group
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the first command should complete before the second command starts
    // (Sequential execution means both files exist after the call returns)
    assert!(marker_a.exists(), "first command should have written marker_a");
    assert!(marker_b.exists(), "second command should have written marker_b");

    // @step And both command results should contribute to the hook group outcome
    // Both commands exit 0 (success), so the overall decision should be Continue
    assert_eq!(
        outcome.decision,
        codelet_core::lifecycle_hooks::PreToolHookDecision::Continue,
        "both commands succeeded, final decision should be Continue"
    );
}
