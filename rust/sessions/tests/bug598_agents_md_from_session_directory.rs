//! BUG-598: AGENTS.md is read from the daemon's directory, not the
//! session's.
//!
//! Feature: spec/features/agents-md-is-read-from-the-session-working-directory.feature
//!
//! The test process plays the daemon: its working directory is set once to
//! a temp directory D holding an AGENTS.md. Each scenario builds a real
//! `SessionManager` session over temp directories (an isolated session is a
//! session with a worktree path) and reads the text of the session's
//! context reminders from its messages.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use codelet_rpc_types::SessionId;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use rig::message::{Message, UserContent};

const DAEMON_RULES: &str = "daemon rules";
const PROJECT_RULES: &str = "project rules";
const WORKTREE_RULES: &str = "worktree rules";

/// Process-wide setup, once: no host ~/.fspec, an offline model catalog,
/// and the "daemon" working directory D.
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
            std::fs::create_dir_all(&cwd)?;
            std::fs::write(cwd.join("AGENTS.md"), DAEMON_RULES)?;
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

async fn session(
    manager: &Arc<SessionManager>,
    project: &Path,
) -> Result<(SessionId, Arc<BackgroundSession>), TestError> {
    let id = SessionId::from(uuid::Uuid::new_v4().to_string());
    manager
        .create_session_with_id(
            &id.value,
            "anthropic/claude-sonnet-4",
            &project.to_string_lossy(),
            "test",
        )
        .await?;
    let session = manager.get_session(&id.value)?;
    Ok((id, session))
}

fn make_isolated(session: &BackgroundSession, worktree: &Path) -> Result<(), TestError> {
    *session.worktree_path.write().map_err(|e| e.to_string())? = Some(worktree.to_path_buf());
    Ok(())
}

/// Every text part of the session's user messages (its context reminders).
async fn reminders(session: &BackgroundSession) -> String {
    let inner = session.inner.lock().await;
    inner
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => Some(
                content
                    .iter()
                    .filter_map(|part| match part {
                        UserContent::Text(text) => Some(text.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scenario: A new session reads its project's AGENTS.md, not the daemon's
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_session_reads_its_projects_agents_md_not_the_daemons() -> TestResult {
    // @step Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    daemon()?;
    let manager = SessionManager::new();
    let project = setup_temp_directory()?;

    // @step And project P holds an AGENTS.md saying 'project rules'
    std::fs::write(project.path().join("AGENTS.md"), PROJECT_RULES)?;

    // @step When a session is created over project P
    let (_, session) = session(&manager, project.path()).await?;

    // @step Then its context reminders say 'project rules'
    let text = reminders(&session).await;
    assert!(text.contains(PROJECT_RULES), "got {text}");

    // @step And they never say 'daemon rules'
    assert!(!text.contains(DAEMON_RULES), "got {text}");
    Ok(())
}

/// Scenario: An isolated session reads its worktree's AGENTS.md after clearing history
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_isolated_session_reads_its_worktrees_agents_md_after_clearing_history() -> TestResult {
    // @step Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    let main = dirs.path().join("main");
    let worktree = main.join(".fspec/worktrees/session");
    std::fs::create_dir_all(&worktree)?;
    std::fs::write(main.join("AGENTS.md"), PROJECT_RULES)?;

    // @step And an isolated session over project P with worktree W holding an AGENTS.md saying 'worktree rules'
    std::fs::write(worktree.join("AGENTS.md"), WORKTREE_RULES)?;
    let (_, session) = session(&manager, &main).await?;
    make_isolated(&session, &worktree)?;

    // @step When the session clears its history
    let clearing = session.clone();
    tokio::task::spawn_blocking(move || clearing.clear_history())
        .await?
        .map_err(|e| e.to_string())?;

    // @step Then its context reminders say 'worktree rules'
    let text = reminders(&session).await;
    assert!(text.contains(WORKTREE_RULES), "got {text}");

    // @step And they never say 'daemon rules'
    assert!(!text.contains(DAEMON_RULES), "got {text}");
    Ok(())
}

/// Scenario: A project without AGENTS.md never gets the daemon's
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_project_without_agents_md_never_gets_the_daemons() -> TestResult {
    // @step Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    daemon()?;
    let manager = SessionManager::new();
    let project = setup_temp_directory()?;

    // @step And project P has no AGENTS.md in it or above it
    let mut above = Some(project.path());
    while let Some(dir) = above {
        assert!(
            !dir.join("AGENTS.md").exists(),
            "{} has an AGENTS.md",
            dir.display()
        );
        above = dir.parent();
    }

    // @step When a session is created over project P
    let (_, session) = session(&manager, project.path()).await?;

    // @step Then its context reminders never say 'daemon rules'
    let text = reminders(&session).await;
    assert!(!text.contains(DAEMON_RULES), "got {text}");
    Ok(())
}
