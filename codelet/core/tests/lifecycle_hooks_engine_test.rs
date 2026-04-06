#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Lifecycle Hooks Engine — Execution & Output Interpretation Tests
//!
//! Feature: spec/features/agent-lifecycle-hooks.feature
//!
//! Tests for HOOK-015: Hook execution engine — command execution, JSON payloads,
//! environment variables, timeout handling, exit code interpretation,
//! JSON response parsing (Claude Code compatible), and context injection.

use std::fs;

use serde_json::json;
use tempfile::TempDir;

use codelet_core::lifecycle_hooks::{
    CompiledHookCommand, CompiledHookDefinition, CompiledHookGroup, CompiledLifecycleHooks,
    HookContext, HookMatcher, PreToolHookDecision, run_post_tool, run_pre_tool,
    run_session_end, run_session_start,
};

// ===== Helpers =====

/// Create a HookContext for testing with a real temp directory as workspace.
fn make_context(workspace: &std::path::Path) -> HookContext {
    HookContext {
        session_id: "test-session-abc-123".to_string(),
        cwd: workspace.to_string_lossy().to_string(),
        transcript_path: workspace
            .join("transcript.json")
            .to_string_lossy()
            .to_string(),
    }
}

/// Create a CompiledLifecycleHooks with a single session_start hook.
fn make_session_start_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![CompiledHookDefinition {
            name: "test-hook".to_string(),
            command: command.to_string(),
            blocking: false,
            timeout,
        }],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![],
    }
}

/// Create a CompiledLifecycleHooks with a single session_end hook.
fn make_session_end_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![CompiledHookDefinition {
            name: "test-hook".to_string(),
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

/// Create a CompiledLifecycleHooks with a single pre_tool_use group matching all tools.
fn make_pre_tool_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![CompiledHookGroup {
            matcher: HookMatcher::Any,
            commands: vec![CompiledHookCommand {
                command: command.to_string(),
                timeout,
            }],
        }],
        post_tool_use: vec![],
    }
}

/// Create a CompiledLifecycleHooks with a single post_tool_use group matching all tools.
fn make_post_tool_hooks(command: &str, timeout: u64) -> CompiledLifecycleHooks {
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![CompiledHookGroup {
            matcher: HookMatcher::Any,
            commands: vec![CompiledHookCommand {
                command: command.to_string(),
                timeout,
            }],
        }],
    }
}

/// Create a CompiledLifecycleHooks with a single post_tool_use group with specific matcher.
fn make_post_tool_hooks_with_matcher(
    command: &str,
    timeout: u64,
    pattern: &str,
) -> CompiledLifecycleHooks {
    let regex = regex::Regex::new(&format!("^(?:{pattern})$"))
        .unwrap_or_else(|e| panic!("invalid test regex: {e}"));
    CompiledLifecycleHooks {
        global_timeout: 60,
        global_shell: None,
        session_start: vec![],
        session_end: vec![],
        user_prompt_submit: vec![],
        notification: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![CompiledHookGroup {
            matcher: HookMatcher::Pattern(regex),
            commands: vec![CompiledHookCommand {
                command: command.to_string(),
                timeout,
            }],
        }],
    }
}

// ===== Command Execution: JSON Payload on stdin =====

/// @HOOK-015 Scenario: Hook command receives JSON payload on stdin
#[tokio::test]
async fn test_hook_receives_json_payload_on_stdin() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("payload.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that echoes stdin to a file
    let cmd = format!("cat > '{}'", capture_file.display());
    let hooks = make_pre_tool_hooks(&cmd, 10);

    // @step When the agent invokes the "Bash" tool with input "ls -la"
    let tool_input = json!({"command": "ls -la"});
    let _outcome = run_pre_tool(&hooks, &ctx, "Bash", &tool_input).await;

    // @step Then the hook should receive a JSON payload on stdin containing:
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading capture file: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&captured)
        .unwrap_or_else(|e| panic!("parsing payload JSON: {e}"));

    assert_eq!(payload["hook_event_name"], "PreToolUse");
    assert_eq!(payload["tool_name"], "Bash");

    // @step And the tool_input field should contain "ls -la"
    assert_eq!(payload["tool_input"]["command"], "ls -la");
}

/// @HOOK-015 Scenario: Hook command receives environment variables
#[tokio::test]
async fn test_hook_receives_environment_variables() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("envs.txt");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_start" hook that writes env vars to a file
    let cmd = format!("printenv | grep FSPEC_ | sort > '{}'", capture_file.display());
    let hooks = make_session_start_hooks(&cmd, 10);

    // @step When the session_start hook executes
    let _outcome = run_session_start(&hooks, &ctx, "startup").await;

    // @step Then the hook process should have FSPEC_PROJECT_DIR set to the workspace path
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading env capture: {e}"));

    assert!(
        captured.contains(&format!("FSPEC_PROJECT_DIR={}", tmp.path().display())),
        "should have FSPEC_PROJECT_DIR, got: {captured}"
    );

    // @step And the hook process should have FSPEC_SESSION_ID set to the session UUID
    assert!(
        captured.contains("FSPEC_SESSION_ID=test-session-abc-123"),
        "should have FSPEC_SESSION_ID, got: {captured}"
    );

    // @step And the hook process should have FSPEC_HOOK_EVENT set to "SessionStart"
    assert!(
        captured.contains("FSPEC_HOOK_EVENT=SessionStart"),
        "should have FSPEC_HOOK_EVENT, got: {captured}"
    );

    // @step And the hook process should have FSPEC_TRANSCRIPT_PATH set to the transcript file path
    let expected_path = tmp.path().join("transcript.json");
    assert!(
        captured.contains(&format!(
            "FSPEC_TRANSCRIPT_PATH={}",
            expected_path.display()
        )),
        "should have FSPEC_TRANSCRIPT_PATH, got: {captured}"
    );
}

/// @HOOK-015 Scenario: SessionStart payload includes session source
#[tokio::test]
async fn test_session_start_payload_includes_source() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("payload.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_start" hook
    let cmd = format!("cat > '{}'", capture_file.display());
    let hooks = make_session_start_hooks(&cmd, 10);

    // @step When a new session starts fresh
    let _outcome = run_session_start(&hooks, &ctx, "startup").await;

    // @step Then the hook payload should include source "startup"
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading capture: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&captured)
        .unwrap_or_else(|e| panic!("parsing JSON: {e}"));
    assert_eq!(payload["source"], "startup");

    // @step When a session is resumed
    let _outcome = run_session_start(&hooks, &ctx, "resume").await;

    // @step Then the hook payload should include source "resume"
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading capture: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&captured)
        .unwrap_or_else(|e| panic!("parsing JSON: {e}"));
    assert_eq!(payload["source"], "resume");
}

/// @HOOK-015 Scenario: PostToolUse payload includes tool response
#[tokio::test]
async fn test_post_tool_use_payload_includes_tool_response() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("payload.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "post_tool_use" hook group matching all tools
    let cmd = format!("cat > '{}'", capture_file.display());
    let hooks = make_post_tool_hooks(&cmd, 10);

    // @step When the agent invokes the "Read" tool and it returns file contents
    let tool_input = json!({"file_path": "/tmp/test.txt"});
    let _outcome =
        run_post_tool(&hooks, &ctx, "Read", &tool_input, "file contents here").await;

    // @step Then the post_tool_use hook should receive a payload containing tool_name, tool_input, and tool_response
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading capture: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&captured)
        .unwrap_or_else(|e| panic!("parsing JSON: {e}"));

    assert_eq!(payload["hook_event_name"], "PostToolUse");
    assert_eq!(payload["tool_name"], "Read");
    assert_eq!(payload["tool_input"]["file_path"], "/tmp/test.txt");
    assert_eq!(payload["tool_response"], "file contents here");
}

/// @HOOK-015 Scenario: SessionEnd payload includes termination reason
#[tokio::test]
async fn test_session_end_payload_includes_reason() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let capture_file = tmp.path().join("payload.json");
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_end" hook
    let cmd = format!("cat > '{}'", capture_file.display());
    let hooks = make_session_end_hooks(&cmd, 10);

    // @step When the session ends because the user cancelled it
    let _outcome = run_session_end(&hooks, &ctx, "cancelled").await;

    // @step Then the hook payload should include reason "cancelled"
    let captured = fs::read_to_string(&capture_file)
        .unwrap_or_else(|e| panic!("reading capture: {e}"));
    let payload: serde_json::Value = serde_json::from_str(&captured)
        .unwrap_or_else(|e| panic!("parsing JSON: {e}"));
    assert_eq!(payload["reason"], "cancelled");
}

// ===== Timeout Handling =====

/// @HOOK-015 Scenario: Hook command killed on timeout
#[tokio::test]
async fn test_hook_killed_on_timeout() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_start" hook with timeout 1 second
    // @step And the hook command sleeps for 10 seconds
    let hooks = make_session_start_hooks("sleep 10", 1);

    // @step When the session_start hook executes
    let start = std::time::Instant::now();
    let outcome = run_session_start(&hooks, &ctx, "startup").await;
    let elapsed = start.elapsed();

    // @step Then the hook process should be killed after 1 second
    assert!(
        elapsed.as_secs() < 5,
        "should have timed out quickly, took {elapsed:?}"
    );

    // @step And the outcome should be a warning
    assert!(
        outcome
            .messages
            .iter()
            .any(|m| m.level == codelet_core::lifecycle_hooks::HookMessageLevel::Warning),
        "should have a warning message about timeout"
    );
}

/// @HOOK-015 Scenario: pre_tool_use timeout is treated as Deny for safety
#[tokio::test]
async fn test_pre_tool_use_timeout_is_deny() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook with timeout 1 second
    // @step And the hook command sleeps for 10 seconds
    let hooks = make_pre_tool_hooks("sleep 10", 1);

    // @step When the agent invokes the "Bash" tool
    let start = std::time::Instant::now();
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;
    let elapsed = start.elapsed();

    // @step Then the hook process should be killed after 1 second
    assert!(
        elapsed.as_secs() < 5,
        "should have timed out quickly, took {elapsed:?}"
    );

    // @step And the tool call should be denied
    assert_eq!(
        outcome.decision,
        PreToolHookDecision::Deny,
        "pre_tool_use timeout should be Deny for safety"
    );
}

/// @HOOK-015 Scenario: post_tool_use timeout is treated as Warning
#[tokio::test]
async fn test_post_tool_use_timeout_is_warning() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "post_tool_use" hook with timeout 1 second
    // @step And the hook command sleeps for 10 seconds
    let hooks = make_post_tool_hooks("sleep 10", 1);

    // @step When the agent completes a tool call
    let start = std::time::Instant::now();
    let outcome = run_post_tool(&hooks, &ctx, "Bash", &json!({}), "output").await;
    let elapsed = start.elapsed();

    // @step Then the hook process should be killed after 1 second
    assert!(
        elapsed.as_secs() < 5,
        "should have timed out quickly, took {elapsed:?}"
    );

    // @step And a warning should be emitted but execution should continue
    assert!(
        outcome
            .messages
            .iter()
            .any(|m| m.level == codelet_core::lifecycle_hooks::HookMessageLevel::Warning),
        "should have a warning message about timeout"
    );
}

// ===== Exit Code Interpretation =====

/// @HOOK-015 Scenario: Exit code 0 means success
#[tokio::test]
async fn test_exit_code_0_means_continue() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 0
    let hooks = make_pre_tool_hooks("exit 0", 10);

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the hook outcome should be Continue (no opinion)
    assert_eq!(outcome.decision, PreToolHookDecision::Continue);

    // @step And the tool call should proceed with normal policy
    // (Continue means the engine doesn't block or allow — it defers)
}

/// @HOOK-015 Scenario: Exit code 2 with stderr means deny
#[tokio::test]
async fn test_exit_code_2_with_stderr_means_deny() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 2 and stderr "Destructive command blocked"
    let hooks =
        make_pre_tool_hooks("echo 'Destructive command blocked' >&2; exit 2", 10);

    // @step When the agent tries to execute "rm -rf /"
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({"command": "rm -rf /"})).await;

    // @step Then the tool call should be denied
    assert_eq!(outcome.decision, PreToolHookDecision::Deny);

    // @step And the deny reason should contain "Destructive command blocked"
    let reason = outcome.reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("Destructive command blocked"),
        "deny reason should contain stderr message, got: {reason}"
    );
}

/// @HOOK-015 Scenario: Exit code 2 without stderr means warning
#[tokio::test]
async fn test_exit_code_2_without_stderr_means_warning() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 2 and empty stderr
    let hooks = make_pre_tool_hooks("exit 2", 10);

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the hook outcome should be a warning
    // @step And the tool call should continue
    assert_eq!(
        outcome.decision,
        PreToolHookDecision::Continue,
        "exit 2 with empty stderr should be a warning, not deny"
    );
    assert!(
        outcome
            .messages
            .iter()
            .any(|m| m.level == codelet_core::lifecycle_hooks::HookMessageLevel::Warning),
        "should have a warning message"
    );
}

/// @HOOK-015 Scenario: Non-zero exit code other than 2 is a non-blocking warning
#[tokio::test]
async fn test_nonzero_exit_other_than_2_is_warning() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 1
    let hooks = make_pre_tool_hooks("exit 1", 10);

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then a warning should be emitted about the hook failure
    assert!(
        outcome
            .messages
            .iter()
            .any(|m| m.level == codelet_core::lifecycle_hooks::HookMessageLevel::Warning),
        "should have a warning message about non-zero exit"
    );

    // @step And the tool call should continue
    assert_eq!(
        outcome.decision,
        PreToolHookDecision::Continue,
        "non-zero exit (not 2) should continue"
    );
}

// ===== JSON Response Interpretation (Claude Code Compatible) =====

/// @HOOK-015 Scenario: pre_tool_use hook returns JSON with permissionDecision allow
#[tokio::test]
async fn test_pre_tool_json_permission_allow() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
    let hooks = make_pre_tool_hooks(
        r#"echo '{"hookSpecificOutput":{"permissionDecision":"allow"}}'"#,
        10,
    );

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the tool should execute immediately without any permission prompt
    assert_eq!(outcome.decision, PreToolHookDecision::Allow);
}

/// @HOOK-015 Scenario: pre_tool_use hook returns JSON with permissionDecision deny
#[tokio::test]
async fn test_pre_tool_json_permission_deny() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
    let hooks = make_pre_tool_hooks(
        r#"echo '{"hookSpecificOutput":{"permissionDecision":"deny"},"decision":"deny","reason":"Policy violation"}'"#,
        10,
    );

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the tool call should be denied
    assert_eq!(outcome.decision, PreToolHookDecision::Deny);

    // @step And the deny reason should contain "Policy violation"
    let reason = outcome.reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("Policy violation"),
        "deny reason should contain 'Policy violation', got: {reason}"
    );
}

/// @HOOK-015 Scenario: pre_tool_use hook returns JSON with permissionDecision ask
#[tokio::test]
async fn test_pre_tool_json_permission_ask() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
    let hooks = make_pre_tool_hooks(
        r#"echo '{"hookSpecificOutput":{"permissionDecision":"ask"}}'"#,
        10,
    );

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the user should be prompted for interactive permission
    assert_eq!(outcome.decision, PreToolHookDecision::Ask);
}

/// @HOOK-015 Scenario: pre_tool_use hook returns JSON with continue false
#[tokio::test]
async fn test_pre_tool_json_continue_false() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
    let hooks = make_pre_tool_hooks(
        r#"echo '{"continue":false,"reason":"Blocked by policy"}'"#,
        10,
    );

    // @step When the agent invokes a tool
    let outcome = run_pre_tool(&hooks, &ctx, "Bash", &json!({})).await;

    // @step Then the tool call should be denied
    assert_eq!(outcome.decision, PreToolHookDecision::Deny);

    // @step And the deny reason should contain "Blocked by policy"
    let reason = outcome.reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("Blocked by policy"),
        "deny reason should contain 'Blocked by policy', got: {reason}"
    );
}

/// @HOOK-015 Scenario: Hook injects additional context as system message
#[tokio::test]
async fn test_hook_injects_additional_context_json() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_start" hook that outputs JSON:
    let hooks = make_session_start_hooks(
        r#"echo '{"hookSpecificOutput":{"additionalContext":"Always follow company coding standards"}}'"#,
        10,
    );

    // @step When the session starts
    let outcome = run_session_start(&hooks, &ctx, "startup").await;

    // @step Then "Always follow company coding standards" should be injected as a system message into the conversation
    assert!(
        outcome
            .additional_context
            .iter()
            .any(|c| c.contains("Always follow company coding standards")),
        "should contain additional context, got: {:?}",
        outcome.additional_context
    );
}

/// @HOOK-015 Scenario: session_start hook injects context via plain text stdout
#[tokio::test]
async fn test_session_start_plain_text_context() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "session_start" hook that outputs plain text "Remember: use TypeScript only"
    let hooks =
        make_session_start_hooks("echo 'Remember: use TypeScript only'", 10);

    // @step When the session starts
    let outcome = run_session_start(&hooks, &ctx, "startup").await;

    // @step Then "Remember: use TypeScript only" should be injected as additional context
    assert!(
        outcome
            .additional_context
            .iter()
            .any(|c| c.contains("Remember: use TypeScript only")),
        "should contain plain text as context, got: {:?}",
        outcome.additional_context
    );
}

/// @HOOK-015 Scenario: post_tool_use hook injects additional context
#[tokio::test]
async fn test_post_tool_injects_context() {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let ctx = make_context(tmp.path());

    // @step Given a spec/fspec-hooks.json with a "post_tool_use" hook matching "Write|Edit" that outputs JSON:
    let hooks = make_post_tool_hooks_with_matcher(
        r#"echo '{"hookSpecificOutput":{"additionalContext":"Lint warning: unused import on line 5"}}'"#,
        10,
        "Write|Edit",
    );

    // @step When the agent completes a Write tool call
    let outcome =
        run_post_tool(&hooks, &ctx, "Write", &json!({}), "file written").await;

    // @step Then "Lint warning: unused import on line 5" should be injected as a system message
    assert!(
        outcome
            .additional_context
            .iter()
            .any(|c| c.contains("Lint warning: unused import on line 5")),
        "should contain lint warning context, got: {:?}",
        outcome.additional_context
    );
}
