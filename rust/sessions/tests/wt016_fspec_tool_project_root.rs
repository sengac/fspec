//! WT-016: the Fspec tool resolves its project root from the daemon's
//! working directory.
//!
//! Feature: spec/features/fspec-tool-resolves-its-project-root-from-the-session.feature
//!
//! The test process plays the daemon: its working directory is set once
//! to a temp directory whose board holds DAEMON-001. Each scenario builds
//! a real `SessionManager` session over temp directories (no git needed:
//! an isolated session is a session with a worktree path), registers the
//! real `codelet_agent_loop::fspec_handler`, and calls the Claude-facing
//! Fspec tool the way an agent does.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use rig::tool::Tool;
use serde_json::json;

/// Process-wide setup, once: no host ~/.fspec, an offline model catalog,
/// and the "daemon" working directory with its own board.
fn daemon() -> Result<&'static Path, TestError> {
    static DAEMON: OnceLock<Result<(tempfile::TempDir, PathBuf), String>> = OnceLock::new();
    let daemon = DAEMON.get_or_init(|| {
        let setup = || -> Result<(tempfile::TempDir, PathBuf), TestError> {
            codelet_test_helpers::isolated_user_directory()?;
            let data = setup_temp_directory()?;
            std::fs::create_dir_all(data.path().join("cache"))?;
            std::fs::write(
                data.path().join("cache/models.json"),
                include_str!("fixtures/rpc422_models.json"),
            )?;
            codelet_common::set_data_directory(data.path().to_owned())?;
            std::env::set_var("ANTHROPIC_API_KEY", "offline-test");
            let cwd = data.path().join("daemon-cwd");
            board(&cwd, "DAEMON-001")?;
            std::env::set_current_dir(&cwd)?;
            Ok((data, cwd))
        };
        setup().map_err(|e| e.to_string())
    });
    match daemon {
        Ok((_, cwd)) => Ok(cwd.as_path()),
        Err(e) => Err(e.clone().into()),
    }
}

/// Write a board at `<root>/spec/work-units.json` holding one card.
fn board(root: &Path, card: &str) -> Result<(), TestError> {
    std::fs::create_dir_all(root.join("spec"))?;
    std::fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_vec(&json!({
            "workUnits": {card: {"id": card, "title": card, "type": "task", "status": "backlog"}},
            "states": {"backlog": [card]}
        }))?,
    )?;
    Ok(())
}

/// A session over `project` (isolated when `worktree` is given) whose
/// Fspec tool runs through the real agent-loop handler.
async fn session(
    manager: &Arc<SessionManager>,
    project: &Path,
    worktree: Option<&Path>,
) -> Result<uuid::Uuid, TestError> {
    let id = uuid::Uuid::new_v4();
    manager
        .create_session_with_id(
            &id.to_string(),
            "anthropic/claude-sonnet-4",
            &project.to_string_lossy(),
            "test",
        )
        .await?;
    let session = manager.get_session(&SessionId::from(id.to_string()).value)?;
    *session.worktree_path.write().map_err(|e| e.to_string())? = worktree.map(Path::to_path_buf);
    let handler = codelet_agent_loop::fspec_handler::create_handler(session, false);
    codelet_tools::set_fspec_handler_for_session(id, Some(handler));
    Ok(id)
}

async fn fspec_tool(
    session: uuid::Uuid,
    args: serde_json::Value,
) -> Result<String, codelet_tools::ToolError> {
    codelet_tools::facade::claude_fspec_tool(session)
        .call(codelet_tools::facade::wrapper::FacadeArgs(args))
        .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relative_root_in_an_isolated_session_resolves_to_the_worktree() -> TestResult {
    // @step Given the daemon's working directory holds a board with DAEMON-001
    daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    let main = dirs.path().join("main");
    let worktree = dirs.path().join("main/.fspec/worktrees/session");
    board(&main, "MAIN-001")?;

    // @step And an isolated session whose worktree holds a board with TREE-001
    board(&worktree, "TREE-001")?;
    let id = session(&manager, &main, Some(&worktree)).await?;

    // @step When the session's Fspec tool runs list-work-units with project_root '.'
    let listed = fspec_tool(
        id,
        json!({"command": "list-work-units", "args": "{}", "project_root": "."}),
    )
    .await?;

    // @step Then the result lists TREE-001
    assert!(listed.contains("TREE-001"), "got {listed}");

    // @step And the result does not list DAEMON-001
    assert!(!listed.contains("DAEMON-001"), "got {listed}");
    assert!(!listed.contains("MAIN-001"), "got {listed}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_default_root_in_a_non_isolated_session_resolves_to_the_session_project() -> TestResult
{
    // @step Given the daemon's working directory holds a board with DAEMON-001
    daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;

    // @step And a non-isolated session whose project holds a board with PROJ-001
    board(dirs.path(), "PROJ-001")?;
    let id = session(&manager, dirs.path(), None).await?;

    // @step When the session's Fspec tool runs list-work-units without a project_root
    let listed = fspec_tool(id, json!({"command": "list-work-units", "args": "{}"})).await?;

    // @step Then the result lists PROJ-001
    assert!(listed.contains("PROJ-001"), "got {listed}");

    // @step And the result does not list DAEMON-001
    assert!(!listed.contains("DAEMON-001"), "got {listed}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_isolated_session_cannot_point_the_fspec_tool_at_the_main_checkout() -> TestResult {
    daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    let main = dirs.path().join("main");
    let worktree = dirs.path().join("main/.fspec/worktrees/session");
    board(&worktree, "TREE-001")?;

    // @step Given an isolated session whose project is the main checkout
    board(&main, "MAIN-001")?;
    let before = std::fs::read(main.join("spec/work-units.json"))?;
    let id = session(&manager, &main, Some(&worktree)).await?;

    // @step When the session's Fspec tool runs create-story with the main checkout's absolute path as project_root
    let result = fspec_tool(
        id,
        json!({
            "command": "create-story",
            "args": json!({"prefix": "MAIN", "title": "Escaped story"}).to_string(),
            "project_root": main.to_string_lossy(),
        }),
    )
    .await;

    // @step Then the tool fails with the 'blocked from original project' error
    match result {
        Err(error) => assert!(
            error.to_string().contains("blocked from original project"),
            "got {error}"
        ),
        Ok(output) => return Err(format!("expected a refusal, got {output}").into()),
    }

    // @step And the main checkout's board is unchanged
    assert_eq!(std::fs::read(main.join("spec/work-units.json"))?, before);
    Ok(())
}
