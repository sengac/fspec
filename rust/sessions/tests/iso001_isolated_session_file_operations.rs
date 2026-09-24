//! AUDIT-002 (P1 band 2 + P0) — Isolated session file-operations, real tools.
//!
//! Feature: spec/features/isolated-session-file-operations.feature
//!
//! These scenarios drive the REAL file tools (Read/Write/Edit/Ls/Grep/Glob/
//! AstGrep/AstGrepRefactor via `rig::tool::Tool::call`) against REAL isolated
//! sessions created through the non-singleton `SessionManager` the way the
//! fspec binary's `build_service` registers the tool callbacks (WT-004
//! precedent, `wt004_session_tool_callbacks.rs`). No mocks or test doubles
//! for the isolation mechanism — per the feature's doc-string rule.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod iso001_support;

use std::time::Duration;

use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk};
use codelet_tools::astgrep::AstGrepArgs;
use codelet_tools::astgrep_refactor::AstGrepRefactorArgs;
use codelet_tools::edit::EditArgs;
use codelet_tools::glob::GlobArgs;
use codelet_tools::grep::GrepArgs;
use codelet_tools::ls::LsArgs;
use codelet_tools::read::ReadArgs;
use codelet_tools::write::WriteArgs;
use codelet_tools::{
    AstGrepRefactorTool, AstGrepTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, ToolError,
    WriteTool,
};
use rig::tool::Tool;

use iso001_support::{iso_env, next_user_notification, plain_env, unique_tmp_dir, unique_tmp_file};

/// Assert a tool call failed with a validation error naming the original
/// project (isolation block).
fn assert_blocked(err: ToolError, tool: &str) {
    match err {
        ToolError::Validation { tool: t, message } => {
            assert_eq!(t, tool, "error must name the {tool} tool");
            assert!(
                message.contains("original project"),
                "error must name the blocked original project, got: {message}"
            );
        }
        other => panic!("expected a Validation error from {tool}, got {other:?}"),
    }
}

// =============================================================================
// BLOCKING SCENARIOS
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_blocked_original_project_file() {
    // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    // @step And an isolated session is created via sessionManagerCreateIsolated NAPI binding
    // @step And the session has worktree at "/project/.fspec/worktrees/<session-id>"
    let mut env = iso_env("main.ts", "main project content\n").await;

    // @step When the Read tool is invoked with file_path "/project/src/main.ts"
    let tool = ReadTool::new(env.session_id);
    let result = tool
        .call(ReadArgs {
            file_path: env.repo.join("src/main.ts").to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("read must be blocked"), "read");

    // @step And the file should NOT be read
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/main.ts")).expect("re-read"),
        "main project content\n",
        "original project file must be untouched"
    );

    // @step And a block notification should be emitted
    let chunk = next_user_notification(
        &mut env.chunks_rx,
        &SessionId::new(env.session_id.to_string()),
        Duration::from_secs(5),
    )
    .await;
    if let StreamChunk::UserNotification { message, severity } = chunk {
        assert_eq!(severity, NotificationSeverity::Warning);
        assert!(message.contains("blocked"), "notification: {message}");
    }
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_blocked_path_traversal() {
    // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains directory "src/"
    let env = iso_env("main.ts", "main project content\n").await;

    // @step When the Read tool is invoked with file_path "../../../src/main.ts"
    // (constructed relative to the worktree root: the worktree lives at
    // <repo>/.fspec/worktrees/<id>, so ../../../ reaches <repo>/src/main.ts)
    let tool = ReadTool::new(env.session_id);
    let result = tool
        .call(ReadArgs {
            file_path: env
                .worktree
                .join("../../../src/main.ts")
                .to_string_lossy()
                .to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("traversal must be blocked"), "read");

    // @step And the file should NOT be read
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/main.ts")).expect("re-read"),
        "main project content\n",
        "the original project file must remain untouched"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_blocked_symlink_escape() {
    // @step Given a git repository at "/project" with file "/project/src/secret.ts" containing "secret content"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("secret.ts", "secret content\n").await;

    // @step And the worktree contains a symlink "escape" pointing to "/project/src/"
    std::os::unix::fs::symlink(env.repo.join("src"), env.worktree.join("escape").as_path())
        .expect("create escape symlink");

    // @step When the Read tool is invoked with file_path "escape/secret.ts"
    let tool = ReadTool::new(env.session_id);
    let result = tool
        .call(ReadArgs {
            file_path: env
                .worktree
                .join("escape/secret.ts")
                .to_string_lossy()
                .to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("symlink escape must be blocked"), "read");

    // @step And the file should NOT be read
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/secret.ts")).expect("re-read"),
        "secret content\n"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_write_blocked_original_project() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let mut env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Write tool is invoked with file_path "/project/src/malicious.ts" and content "injected code"
    let tool = WriteTool::new(env.session_id);
    let result = tool
        .call(WriteArgs {
            file_path: env
                .repo
                .join("src/malicious.ts")
                .to_string_lossy()
                .to_string(),
            content: "injected code".to_string(),
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("write must be blocked"), "write");

    // @step And the file should NOT exist at "/project/src/malicious.ts"
    assert!(
        !env.repo.join("src/malicious.ts").exists(),
        "blocked write must not create the file"
    );

    // @step And a block notification should be emitted
    let chunk = next_user_notification(
        &mut env.chunks_rx,
        &SessionId::new(env.session_id.to_string()),
        Duration::from_secs(5),
    )
    .await;
    assert!(
        matches!(chunk, StreamChunk::UserNotification { .. }),
        "expected a UserNotification chunk, got {chunk:?}"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_edit_blocked_original_project() {
    // @step Given a git repository at "/project" with file "/project/src/config.ts" containing "original"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let mut env = iso_env("config.ts", "original\n").await;

    // @step When the Edit tool is invoked with file_path "/project/src/config.ts" replacing "original" with "modified"
    let tool = EditTool::new(env.session_id);
    let result = tool
        .call(EditArgs {
            file_path: env
                .repo
                .join("src/config.ts")
                .to_string_lossy()
                .to_string(),
            old_string: "original".to_string(),
            new_string: "modified".to_string(),
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("edit must be blocked"), "edit");

    // @step And the file at "/project/src/config.ts" should still contain "original"
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/config.ts")).expect("re-read"),
        "original\n"
    );

    // @step And a block notification should be emitted
    let chunk = next_user_notification(
        &mut env.chunks_rx,
        &SessionId::new(env.session_id.to_string()),
        Duration::from_secs(5),
    )
    .await;
    assert!(
        matches!(chunk, StreamChunk::UserNotification { .. }),
        "expected a UserNotification chunk, got {chunk:?}"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_ls_blocked_original_project() {
    // @step Given a git repository at "/project" with directory "/project/src/" containing files
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Ls tool is invoked with path "/project/src/"
    let tool = LsTool::new(env.session_id);
    let result = tool
        .call(LsArgs {
            path: Some(env.repo.join("src").to_string_lossy().to_string()),
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("ls must be blocked"), "ls");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_grep_blocked_original_project() {
    // @step Given a git repository at "/project" with files containing searchable content
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "TODO: refactor this\n").await;

    // @step When the Grep tool is invoked with pattern "TODO" and path "/project/src/"
    let tool = GrepTool::new(env.session_id);
    let result = tool
        .call(GrepArgs {
            pattern: "TODO".to_string(),
            path: Some(env.repo.join("src").to_string_lossy().to_string()),
            output_mode: Some("files_with_matches".to_string()),
            glob: None,
            limit: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("grep must be blocked"), "grep");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_glob_blocked_original_project() {
    // @step Given a git repository at "/project" with TypeScript files
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("app.ts", "export const a = 1;\n").await;

    // @step When the Glob tool is invoked with pattern "**/*.ts" and path "/project/"
    let tool = GlobTool::new(env.session_id);
    let result = tool
        .call(GlobArgs {
            pattern: "**/*.ts".to_string(),
            path: Some(env.repo.to_string_lossy().to_string()),
            case_insensitive: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("glob must be blocked"), "glob");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_astgrep_blocked_original_project() {
    // @step Given a git repository at "/project" with TypeScript files
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("app.ts", "function alpha() { return 1; }\n").await;

    // @step When the AstGrep tool is invoked with pattern "function $NAME()" language "typescript" and path "/project/"
    let tool = AstGrepTool::new(env.session_id);
    let result = tool
        .call(AstGrepArgs {
            pattern: "function $NAME($$$ARGS) { $$$BODY }".to_string(),
            language: "typescript".to_string(),
            path: Some(env.repo.to_string_lossy().to_string()),
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(result.expect_err("astgrep must be blocked"), "ast_grep");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_astgrep_refactor_blocked_original_project() {
    // @step Given a git repository at "/project" with file "/project/src/refactor-me.ts"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("refactor-me.ts", "function beta() { return 2; }\n").await;

    // @step When the AstGrepRefactor tool is invoked with source_file "/project/src/refactor-me.ts"
    let tool = AstGrepRefactorTool::new(env.session_id);
    let result = tool
        .call(AstGrepRefactorArgs {
            pattern: "function $NAME($$$ARGS) { $$$BODY }".to_string(),
            language: "typescript".to_string(),
            source_file: env
                .repo
                .join("src/refactor-me.ts")
                .to_string_lossy()
                .to_string(),
            target_file: None,
            replacement: Some("function renamed($$$ARGS) { $$$BODY }".to_string()),
            transforms: None,
            batch: None,
            preview: None,
        })
        .await;

    // @step Then the tool should return an error containing "blocked from original project"
    assert_blocked(
        result.expect_err("astgrep-refactor must be blocked"),
        "ast_grep_refactor",
    );

    // @step And the file at "/project/src/refactor-me.ts" should be unchanged
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/refactor-me.ts")).expect("re-read"),
        "function beta() { return 2; }\n"
    );
}

// =============================================================================
// ALLOWED SCENARIOS
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_allowed_relative_worktree() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains file "src/app.ts" with content "worktree content"
    let env = iso_env("app.ts", "worktree content\n").await;

    // @step When the Read tool is invoked with file_path "src/app.ts"
    let tool = ReadTool::new(env.session_id);
    let content = tool
        .call(ReadArgs {
            file_path: env
                .worktree
                .join("src/app.ts")
                .to_string_lossy()
                .to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("read within worktree must succeed");

    // @step Then the tool should succeed
    assert!(!content.is_empty());

    // @step And the content should be "worktree content"
    assert!(
        content.contains("worktree content"),
        "content must include the worktree file text, got: {content}"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_allowed_absolute_worktree() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains file "src/app.ts" with content "worktree content"
    let env = iso_env("app.ts", "worktree content\n").await;

    // @step When the Read tool is invoked with file_path "/project/.fspec/worktrees/<session-id>/src/app.ts"
    let tool = ReadTool::new(env.session_id);
    let content = tool
        .call(ReadArgs {
            file_path: env
                .worktree
                .join("src/app.ts")
                .to_string_lossy()
                .to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("absolute worktree read must succeed");

    // @step Then the tool should succeed
    assert!(!content.is_empty());

    // @step And the content should be "worktree content"
    assert!(content.contains("worktree content"));
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_read_allowed_tmp() {
    // @step Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step And a file exists at "/tmp/test-file.txt" with content "temp content"
    let tmp_file = unique_tmp_file("test-file.txt", "temp content\n");

    // @step When the Read tool is invoked with file_path "/tmp/test-file.txt"
    let tool = ReadTool::new(env.session_id);
    let content = tool
        .call(ReadArgs {
            file_path: tmp_file.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("/tmp read must succeed");

    // @step Then the tool should succeed
    assert!(!content.is_empty());

    // @step And the content should be "temp content"
    assert!(content.contains("temp content"));
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_ls_allowed_tmp() {
    // @step Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Ls tool is invoked with path "/tmp"
    let tool = LsTool::new(env.session_id);
    let out = tool
        .call(LsArgs {
            path: Some(unique_tmp_dir().to_string_lossy().to_string()),
        })
        .await
        .expect("/tmp listing must succeed");

    // @step Then the tool should succeed
    assert!(!out.is_empty());

    // @step And the output should list directory contents
    assert!(out.lines().count() >= 1, "ls output: {out}");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_grep_allowed_tmp() {
    // @step Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step And a file exists at "/tmp/searchable.txt" with content "findme pattern"
    let tmp_dir = unique_tmp_dir();
    std::fs::write(tmp_dir.join("searchable.txt"), "findme pattern\n").expect("write searchable");

    // @step When the Grep tool is invoked with pattern "findme" and path "/tmp"
    let tool = GrepTool::new(env.session_id);
    let out = tool
        .call(GrepArgs {
            pattern: "findme".to_string(),
            path: Some(tmp_dir.to_string_lossy().to_string()),
            output_mode: Some("content".to_string()),
            glob: None,
            limit: None,
        })
        .await
        .expect("/tmp grep must succeed");

    // @step Then the tool should succeed
    assert!(!out.is_empty());

    // @step And the results should include matches from /tmp
    assert!(out.contains("findme"), "grep results: {out}");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_write_allowed_relative_worktree() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Write tool is invoked with file_path "src/new-file.ts" and content "new content"
    let tool = WriteTool::new(env.session_id);
    tool
        .call(WriteArgs {
            file_path: env
                .worktree
                .join("src/new-file.ts")
                .to_string_lossy()
                .to_string(),
            content: "new content".to_string(),
        })
        .await
        .expect("write within worktree must succeed");

    // @step Then the tool should succeed
    assert!(env.worktree.join("src/new-file.ts").exists());

    // @step And the file should exist at worktree path "src/new-file.ts" with content "new content"
    assert_eq!(
        std::fs::read_to_string(env.worktree.join("src/new-file.ts")).expect("read"),
        "new content"
    );

    // @step And the file should NOT exist at "/project/src/new-file.ts"
    assert!(!env.repo.join("src/new-file.ts").exists());
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_write_allowed_tmp() {
    // @step Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Write tool is invoked with file_path "/tmp/test-write.txt" and content "written content"
    let tmp_file = unique_tmp_file("test-write.txt", "");
    let tool = WriteTool::new(env.session_id);
    tool
        .call(WriteArgs {
            file_path: tmp_file.to_string_lossy().to_string(),
            content: "written content".to_string(),
        })
        .await
        .expect("/tmp write must succeed");

    // @step Then the tool should succeed
    assert!(tmp_file.exists());

    // @step And the file should exist at "/tmp/test-write.txt" with content "written content"
    assert_eq!(
        std::fs::read_to_string(&tmp_file).expect("read"),
        "written content"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_ls_allowed_worktree_dir() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains directory "src/" with files
    let env = iso_env("main.rs", "fn main() {}\n").await;

    // @step When the Ls tool is invoked with path "src/"
    let tool = LsTool::new(env.session_id);
    let out = tool
        .call(LsArgs {
            path: Some(env.worktree.join("src").to_string_lossy().to_string()),
        })
        .await
        .expect("worktree ls must succeed");

    // @step Then the tool should succeed
    assert!(!out.is_empty());

    // @step And the output should list files in the worktree src/ directory
    assert!(out.contains("main.rs"), "ls output: {out}");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_grep_allowed_worktree() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains file "src/app.ts" with content "// FIXME: fix this"
    let env = iso_env("app.ts", "// FIXME: fix this\n").await;

    // @step When the Grep tool is invoked with pattern "FIXME" and path "src/"
    let tool = GrepTool::new(env.session_id);
    let out = tool
        .call(GrepArgs {
            pattern: "FIXME".to_string(),
            path: Some(env.worktree.join("src").to_string_lossy().to_string()),
            output_mode: Some("content".to_string()),
            glob: None,
            limit: None,
        })
        .await
        .expect("worktree grep must succeed");

    // @step Then the tool should succeed
    assert!(!out.is_empty());

    // @step And the results should include matches from worktree
    assert!(out.contains("FIXME"), "grep results: {out}");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_glob_allowed_worktree() {
    // @step Given a git repository at "/project"
    // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    // @step And the worktree contains TypeScript files in "src/"
    let env = iso_env("app.ts", "export const a = 1;\n").await;

    // @step When the Glob tool is invoked with pattern "**/*.ts" and path "src/"
    let tool = GlobTool::new(env.session_id);
    let out = tool
        .call(GlobArgs {
            pattern: "**/*.ts".to_string(),
            path: Some(env.worktree.join("src").to_string_lossy().to_string()),
            case_insensitive: None,
        })
        .await
        .expect("worktree glob must succeed");

    // @step Then the tool should succeed
    assert!(!out.is_empty());

    // @step And the results should only include worktree files
    assert!(out.contains("app.ts"), "glob results: {out}");
}

// =============================================================================
// BACKWARD COMPATIBILITY — NON-ISOLATED SESSIONS
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_non_isolated_read_allowed_original_project() {
    // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main content"
    // @step And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    let env = plain_env("main.ts", "main content\n").await;

    // @step When the Read tool is invoked with file_path "/project/src/main.ts"
    let tool = ReadTool::new(env.session_id);
    let content = tool
        .call(ReadArgs {
            file_path: env
                .repo
                .join("src/main.ts")
                .to_string_lossy()
                .to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("non-isolated read must succeed");

    // @step Then the tool should succeed
    assert!(!content.is_empty());

    // @step And the content should be "main content"
    assert!(content.contains("main content"));
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_non_isolated_write_allowed_all_paths() {
    // @step Given a git repository at "/project"
    // @step And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    let env = plain_env("main.rs", "fn main() {}\n").await;

    // @step When the Write tool is invoked with file_path "/project/src/new.ts" and content "new content"
    let tool = WriteTool::new(env.session_id);
    tool
        .call(WriteArgs {
            file_path: env
                .repo
                .join("src/new.ts")
                .to_string_lossy()
                .to_string(),
            content: "new content".to_string(),
        })
        .await
        .expect("non-isolated write must succeed");

    // @step Then the tool should succeed
    assert!(env.repo.join("src/new.ts").exists());

    // @step And the file should exist at "/project/src/new.ts" with content "new content"
    assert_eq!(
        std::fs::read_to_string(env.repo.join("src/new.ts")).expect("read"),
        "new content"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_non_isolated_access_tmp_anywhere() {
    // @step Given a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    let env = plain_env("main.rs", "fn main() {}\n").await;

    // @step And a file exists at "/tmp/anywhere.txt" with content "accessible"
    let tmp_file = unique_tmp_file("anywhere.txt", "accessible\n");

    // @step When the Read tool is invoked with file_path "/tmp/anywhere.txt"
    let tool = ReadTool::new(env.session_id);
    let content = tool
        .call(ReadArgs {
            file_path: tmp_file.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("non-isolated /tmp read must succeed");

    // @step Then the tool should succeed
    assert!(!content.is_empty());

    // @step And the content should be "accessible"
    assert!(content.contains("accessible"));
}
