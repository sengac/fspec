//! BUG-420 — a session manager always owns its self-reference.
//!
//! Feature: spec/features/session-manager-structural-self-reference.feature

#[path = "wt004_session_tool_callbacks_support/mod.rs"]
#[allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod support;

use std::sync::Arc;

use codelet_sessions::SessionManager;
use support::{fresh_git_repo, ENV_GUARD};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_created_by_a_new_manager_reaches_that_manager() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    let _seeded = support::manager_with_seeded_cache();
    // @step Given a session manager built with SessionManager::new
    let manager: Arc<SessionManager> = SessionManager::new();
    // @step When it creates a session
    let repo = fresh_git_repo();
    let id = uuid::Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().ok_or("utf8")?,
            "bug420",
        )
        .await?;
    // @step Then the session's owning manager is that same manager
    let owner = manager
        .get_session(&id)?
        .owning_manager()
        .ok_or("no owning manager")?;
    assert!(Arc::ptr_eq(&owner, &manager));
    Ok(())
}

#[test]
fn a_manager_cannot_be_built_without_its_self_reference() -> TestResult {
    // @step Given the codelet-sessions session manager sources
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = String::new();
    for entry in std::fs::read_dir(src.join("session_manager"))? {
        sources.push_str(&std::fs::read_to_string(entry?.path())?);
    }
    sources.push_str(&std::fs::read_to_string(src.join("session_manager.rs"))?);
    // @step When they are scanned for an optional self-reference
    let optional = [
        "fn init_self_weak",
        "fn new_arc",
        "self_weak.get().cloned().unwrap_or_default()",
        "OnceLock<Weak<SessionManager>>",
    ];
    let found: Vec<_> = optional
        .iter()
        .filter(|needle| sources.contains(*needle))
        .collect();
    // @step Then there is no init_self_weak, no new_arc and no defaulted self-reference stamp
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}
