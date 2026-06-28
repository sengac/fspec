#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Pre-tool-use hook integration tests for native tools
//!
//! Feature: spec/features/agent-lifecycle-hooks.feature
//!
//! Tests for HOOK-017: Verifies that ALL native tools (Bash, Read, Write, Edit,
//! Ls, Glob, Grep, AstGrep, AstGrepRefactor, Fspec, Bridge, ApplyPatch) call
//! pre_tool_hook_check() before executing, so pre_tool_use hooks fire for all
//! tools regardless of provider.

use std::sync::Arc;

use serial_test::serial;
use uuid::Uuid;

use codelet_tools::pre_tool_hook::{
    register_pre_tool_hook, unregister_pre_tool_hook, PreToolHookDecision, PreToolHookHandler,
};

// ===== Helpers =====

/// Register a Deny hook handler for a session. Returns a counter that tracks
/// how many times the handler was called.
fn register_deny_handler(session_id: Uuid) -> Arc<std::sync::atomic::AtomicUsize> {
    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = call_count.clone();
    let handler: PreToolHookHandler = Arc::new(move |_sid, _tool_name, _tool_input| {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        PreToolHookDecision::Deny("Blocked by test hook".to_string())
    });
    register_pre_tool_hook(session_id, handler);
    call_count
}

/// Register an Allow hook handler for a session.
fn register_allow_handler(session_id: Uuid) {
    let handler: PreToolHookHandler =
        Arc::new(move |_sid, _tool_name, _tool_input| PreToolHookDecision::Allow);
    register_pre_tool_hook(session_id, handler);
}

fn cleanup(session_id: Uuid) {
    unregister_pre_tool_hook(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for BashTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_bash_tool() {
    use codelet_tools::BashTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the BashTool.call() method is invoked
    let tool = BashTool::new(session_id);
    let args = codelet_tools::bash::BashArgs {
        command: "echo hello".to_string(),
        cwd: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "BashTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the command should never have executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for ReadTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_read_tool() {
    use codelet_tools::read::ReadTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the ReadTool.call() method is invoked
    let tool = ReadTool::new(session_id);
    let args = codelet_tools::read::ReadArgs {
        file_path: "/tmp/nonexistent-hook-test.txt".to_string(),
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "ReadTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the file should never have been read
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for WriteTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_write_tool() {
    use codelet_tools::write::WriteTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tmp = tempfile::TempDir::new().unwrap();
    let test_file = tmp.path().join("hook-test-write.txt");

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the WriteTool.call() method is invoked
    let tool = WriteTool::new(session_id);
    let args = codelet_tools::write::WriteArgs {
        file_path: test_file.to_string_lossy().to_string(),
        content: "should not be written".to_string(),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "WriteTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the file should never have been written
    assert!(!test_file.exists(), "File should not have been created");
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for EditTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_edit_tool() {
    use codelet_tools::edit::EditTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tmp = tempfile::TempDir::new().unwrap();
    let test_file = tmp.path().join("hook-test-edit.txt");
    std::fs::write(&test_file, "original content").unwrap();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the EditTool.call() method is invoked
    let tool = EditTool::new(session_id);
    let args = codelet_tools::edit::EditArgs {
        file_path: test_file.to_string_lossy().to_string(),
        old_string: "original".to_string(),
        new_string: "modified".to_string(),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "EditTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the file should never have been modified
    let content = std::fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "original content", "File should be unchanged");
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for LsTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_ls_tool() {
    use codelet_tools::ls::LsTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the LsTool.call() method is invoked
    let tool = LsTool::new(session_id);
    let args = codelet_tools::ls::LsArgs {
        path: Some("/tmp".to_string()),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "LsTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the directory should never have been listed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for GlobTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_glob_tool() {
    use codelet_tools::glob::GlobTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the GlobTool.call() method is invoked
    let tool = GlobTool::new(session_id);
    let args = codelet_tools::glob::GlobArgs {
        pattern: "*.rs".to_string(),
        path: Some("/tmp".to_string()),
        case_insensitive: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "GlobTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the glob should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for GrepTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_grep_tool() {
    use codelet_tools::grep::GrepTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the GrepTool.call() method is invoked
    let tool = GrepTool::new(session_id);
    let args = codelet_tools::grep::GrepArgs {
        pattern: "test".to_string(),
        path: Some("/tmp".to_string()),
        glob: None,
        output_mode: None,
        limit: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "GrepTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the search should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for AstGrepTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_astgrep_tool() {
    use codelet_tools::astgrep::AstGrepTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the AstGrepTool.call() method is invoked
    let tool = AstGrepTool::new(session_id);
    let args = codelet_tools::astgrep::AstGrepArgs {
        pattern: "fn $NAME($$$ARGS) { $$$BODY }".to_string(),
        language: "rust".to_string(),
        path: Some("/tmp".to_string()),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "AstGrepTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the AST search should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for AstGrepRefactorTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_astgrep_refactor_tool() {
    use codelet_tools::astgrep_refactor::AstGrepRefactorTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the AstGrepRefactorTool.call() method is invoked
    let tool = AstGrepRefactorTool::new(session_id);
    let args = codelet_tools::astgrep_refactor::AstGrepRefactorArgs {
        pattern: "fn old() { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: "/tmp/nonexistent.rs".to_string(),
        replacement: Some("fn new() { $$$BODY }".to_string()),
        target_file: None,
        preview: None,
        batch: None,
        transforms: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(
        result.is_err(),
        "AstGrepRefactorTool should have been blocked"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the refactor should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for FspecTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_fspec_tool() {
    use codelet_tools::fspec::FspecTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the FspecTool.call() method is invoked
    let tool = FspecTool::new(session_id);
    let args = codelet_tools::fspec::FspecArgs {
        command: "help".to_string(),
        args: "{}".to_string(),
        project_root: "/tmp".to_string(),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "FspecTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the fspec command should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for BridgeTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_bridge_tool() {
    use codelet_tools::bridge::BridgeTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the BridgeTool.call() method is invoked
    let tool = BridgeTool::new(session_id);
    let args = codelet_tools::bridge::BridgeToolArgs {
        action: codelet_tools::bridge::BridgeAction::List,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "BridgeTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the bridge action should never have been executed
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use hook fires for ApplyPatchTool
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_hook_fires_for_apply_patch_tool() {
    use codelet_tools::apply_patch::ApplyPatchTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Deny hook handler for the session
    let call_count = register_deny_handler(session_id);

    // @step When the ApplyPatchTool.call() method is invoked
    let tool = ApplyPatchTool::new(session_id);
    let args = codelet_tools::apply_patch::ApplyPatchArgs {
        patch: "--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-old\n+new\n".to_string(),
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ToolError::Blocked with the deny reason
    assert!(result.is_err(), "ApplyPatchTool should have been blocked");
    let err = result.unwrap_err();
    assert!(
        format!("{err:?}").contains("Blocked"),
        "Error should be ToolError::Blocked, got: {err:?}"
    );

    // @step And the patch should never have been applied
    assert!(
        call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "Hook handler should have been called"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: pre_tool_use Allow handler lets native tool proceed
// =============================================================================

#[tokio::test]
#[serial]
async fn pre_tool_use_allow_handler_lets_native_tool_proceed() {
    use codelet_tools::BashTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given a registered pre_tool_use Allow hook handler for the session
    register_allow_handler(session_id);

    // @step When the BashTool.call() method is invoked with a safe command
    let tool = BashTool::new(session_id);
    let args = codelet_tools::bash::BashArgs {
        command: "echo hook_allow_test".to_string(),
        cwd: None,
    };
    let result = tool.call(args).await;

    // @step Then the command should execute successfully
    assert!(
        result.is_ok(),
        "BashTool should have succeeded with Allow handler"
    );

    // @step And the output should contain the command result
    let output = result.unwrap();
    assert!(
        output.contains("hook_allow_test"),
        "Output should contain command result, got: {output}"
    );

    cleanup(session_id);
}

// =============================================================================
// Scenario: No registered handler lets native tool proceed without overhead
// =============================================================================

#[tokio::test]
#[serial]
async fn no_registered_handler_lets_native_tool_proceed() {
    use codelet_tools::BashTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();

    // @step Given no pre_tool_use hook handler is registered for the session
    // (no register call — intentionally empty)

    // @step When the BashTool.call() method is invoked
    let tool = BashTool::new(session_id);
    let args = codelet_tools::bash::BashArgs {
        command: "echo no_hook_test".to_string(),
        cwd: None,
    };
    let result = tool.call(args).await;

    // @step Then the command should execute successfully with no hook overhead
    assert!(
        result.is_ok(),
        "BashTool should succeed when no hook handler is registered"
    );
    let output = result.unwrap();
    assert!(
        output.contains("no_hook_test"),
        "Output should contain command result, got: {output}"
    );
}
