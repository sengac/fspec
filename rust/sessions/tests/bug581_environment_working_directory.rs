//! BUG-581: the agent's environment reminder names the daemon's working
//! directory.
//!
//! Feature: spec/features/the-environment-reminder-names-the-session-working-directory.feature
//!
//! The test process plays the daemon: its working directory is set once
//! to a temp directory D. Each scenario builds a real `SessionManager`
//! session over temp directories (an isolated session is a session with a
//! worktree path) and reads the latest environment reminder from the
//! session's messages.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use codelet_rpc_types::SessionId;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{setup_temp_directory, OrFail, TestError, TestResult};
use rig::message::{Message, UserContent};
use serde_json::json;

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

/// A project directory with a board holding AUTH-001.
fn project(root: &Path) -> Result<(), TestError> {
    std::fs::create_dir_all(root.join("spec"))?;
    std::fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_vec(&json!({
            "workUnits": {"AUTH-001": {"id": "AUTH-001", "title": "AUTH-001", "type": "task", "status": "implementing"}},
            "states": {"implementing": ["AUTH-001"]}
        }))?,
    )?;
    Ok(())
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

/// The text of the session's latest environment reminder.
async fn environment_reminder(session: &BackgroundSession) -> Result<String, TestError> {
    let inner = session.inner.lock().await;
    let reminder = inner
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            Message::User { content } => content.iter().find_map(|part| match part {
                UserContent::Text(text) if text.text.contains("Working directory:") => {
                    Some(text.text.clone())
                }
                _ => None,
            }),
            _ => None,
        });
    Ok(reminder.or_fail("the session has an environment reminder")?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_sessions_reminder_names_its_project_not_the_daemons_directory() -> TestResult {
    // @step Given the daemon's working directory is D
    let daemon_cwd = daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    project(dirs.path())?;

    // @step When a session is created over project P
    let (_, session) = session(&manager, dirs.path()).await?;

    // @step Then its environment reminder says 'Working directory: P'
    let reminder = environment_reminder(&session).await?;
    assert!(
        reminder.contains(&format!("Working directory: {}", dirs.path().display())),
        "got {reminder}"
    );

    // @step And the reminder does not name D
    assert!(
        !reminder.contains(&daemon_cwd.display().to_string()),
        "got {reminder}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_isolated_sessions_reminder_names_its_worktree_after_clearing_history() -> TestResult {
    // @step Given the daemon's working directory is D
    let daemon_cwd = daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    let main = dirs.path().join("main");
    let worktree = main.join(".fspec/worktrees/session");
    project(&main)?;
    std::fs::create_dir_all(&worktree)?;

    // @step And an isolated session over project P with worktree W
    let (_, session) = session(&manager, &main).await?;
    make_isolated(&session, &worktree)?;

    // @step When the session clears its history
    let clearing = session.clone();
    tokio::task::spawn_blocking(move || clearing.clear_history())
        .await?
        .map_err(|e| e.to_string())?;

    // @step Then its environment reminder says 'Working directory: W' and 'Isolation: ACTIVE'
    let reminder = environment_reminder(&session).await?;
    assert!(
        reminder.contains(&format!("Working directory: {}", worktree.display())),
        "got {reminder}"
    );
    assert!(reminder.contains("Isolation: ACTIVE"), "got {reminder}");

    // @step And the reminder does not name D
    assert!(
        !reminder.contains(&daemon_cwd.display().to_string()),
        "got {reminder}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_focus_change_keeps_the_isolated_sessions_working_directory_and_isolation_lines(
) -> TestResult {
    // @step Given the daemon's working directory is D
    let daemon_cwd = daemon()?;
    let manager = SessionManager::new();
    let dirs = setup_temp_directory()?;
    let main = dirs.path().join("main");
    let worktree = main.join(".fspec/worktrees/session");
    project(&main)?;
    std::fs::create_dir_all(&worktree)?;

    // @step And an isolated session over project P with worktree W
    let (id, session) = session(&manager, &main).await?;
    make_isolated(&session, &worktree)?;

    // @step When the session's work unit focus is set to AUTH-001
    manager.set_session_focus(&id, Some("AUTH-001"), "test")?;
    let wanted = "Current work unit: AUTH-001";
    codelet_test_helpers::wait_until("the focus reminder", || {
        let session = session.clone();
        async move {
            environment_reminder(&session)
                .await
                .is_ok_and(|reminder| reminder.contains(wanted))
        }
    })
    .await?;

    // @step Then its environment reminder says 'Working directory: W', 'Isolation: ACTIVE' and 'Current work unit: AUTH-001'
    let reminder = environment_reminder(&session).await?;
    assert!(
        reminder.contains(&format!("Working directory: {}", worktree.display())),
        "got {reminder}"
    );
    assert!(reminder.contains("Isolation: ACTIVE"), "got {reminder}");
    assert!(reminder.contains(wanted), "got {reminder}");

    // @step And the reminder does not name D
    assert!(
        !reminder.contains(&daemon_cwd.display().to_string()),
        "got {reminder}"
    );
    Ok(())
}
