//! BUG-405 — session operations never fall back to a SessionManager
//! singleton.
//!
//! Feature: spec/features/no-session-manager-singleton.feature

// The shared WT-004 fixtures; the WT-004 suite that owns them allows these
// lints for the same helpers.
#[path = "wt004_session_tool_callbacks_support/mod.rs"]
#[allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod support;

use std::sync::Arc;

use codelet_sessions::SessionManager;
use codelet_tools::tool_admission::AdmissionMiddleware;
use rig::tool::ToolMiddleware;
use support::{fresh_git_repo, ENV_GUARD};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A session whose owning manager has been dropped: the session outlives the
/// manager, so its owning-manager back-reference no longer resolves (BUG-420
/// makes a manager without a self-reference impossible to build).
async fn orphaned_session() -> Result<
    (
        Arc<codelet_sessions::background_session::BackgroundSession>,
        String,
        tempfile::TempDir,
    ),
    String,
> {
    let manager = SessionManager::new();
    let repo = fresh_git_repo();
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().ok_or("utf8")?,
            "bug405",
        )
        .await?;
    let session = manager.get_session(&id)?;
    drop(manager);
    if session.owning_manager().is_some() {
        return Err("the dropped manager is still reachable".to_string());
    }
    Ok((session, id, repo))
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_admission_for_a_session_with_no_owning_manager_is_refused() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    let _manager = support::manager_with_seeded_cache();
    // @step Given a session whose owning session manager has been dropped
    let (_session, id, repo) = orphaned_session().await?;
    // @step When a tool invocation is admitted for that session
    let tool = codelet_tools::ls::LsTool::new(Uuid::parse_str(&id)?);
    let args = serde_json::json!({"work_unit_id": "NONE", "path": repo.path()}).to_string();
    let result = AdmissionMiddleware::new(id.clone()).call(&tool, args).await;
    // @step Then the admission fails naming the missing session manager
    let error = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(error.contains("no owning session manager"), "{error}");
    Ok(())
}

#[serial_test::serial]
#[test]
fn a_block_notification_with_no_registered_chunk_sender_is_reported() {
    // @step Given no chunk sender is registered for the footer and block notifications
    codelet_sessions::footer_poller::unregister_chunk_sender();
    // @step When the emission target is resolved
    let target = codelet_sessions::footer_poller::emission_target();
    // @step Then it is an error instead of a global broadcast
    assert!(
        target.is_err(),
        "an unregistered sender must not resolve to a global broadcast"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn binding_a_work_unit_reports_a_failed_focus_update() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    let _manager = support::manager_with_seeded_cache();
    // @step Given a session whose owning session manager has been dropped
    let (session, _id, _repo) = orphaned_session().await?;
    // @step When a work unit is bound to the session
    let result = session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("specifying".into()),
    );
    // @step Then the binding fails instead of being logged and skipped
    let error = result.err().unwrap_or_default();
    assert!(error.contains("no owning session manager"), "{error}");
    Ok(())
}

#[test]
fn the_session_manager_singleton_no_longer_exists() -> TestResult {
    // @step Given the codelet-sessions and agent-loop sources
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut stack = vec![root.join("sessions/src"), root.join("agent-loop/src")];
    let mut offenders = Vec::new();
    // @step When they are scanned for the SessionManager singleton
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            let code: String = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            if code.contains("SessionManager::instance()")
                || code.contains("fn instance() -> &'static SessionManager")
            {
                offenders.push(path.display().to_string());
            }
        }
    }
    // @step Then neither defines nor calls SessionManager::instance()
    assert!(offenders.is_empty(), "{offenders:?}");
    Ok(())
}
