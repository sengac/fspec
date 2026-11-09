//! Feature: spec/features/destroyed-session-releases-ownership.feature
//!
//! BUG-288: destroy_session releases the session's manifest writer lease
//! (the OS lock on sessions/<id>.lock) so another owner can take it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "cmpct060/support.rs"]
pub mod support;

use std::fs::OpenOptions;

fn lock_file(id: &str) -> Result<std::fs::File, Box<dyn std::error::Error>> {
    let path = codelet_common::get_data_dir()?
        .join("sessions")
        .join(format!("{id}.lock"));
    Ok(OpenOptions::new().read(true).write(true).open(path)?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_destroyed_sessions_ownership_lock_can_be_taken() -> support::TestResult {
    // @step Given a live session whose manifest the daemon has committed
    let (_directory, manager, session_id, _) = support::manager().await?;
    codelet_core::persistence::update_session_manifest(
        uuid::Uuid::parse_str(&session_id.value)?,
        |manifest| {
            manifest.name = "owned".into();
            Ok(())
        },
    )?;

    // @step And its ownership lock is held by the daemon
    let contender = lock_file(&session_id.value)?;
    assert!(
        contender.try_lock().is_err(),
        "the live session's lease must hold the lock"
    );

    // @step When the session is destroyed
    manager.destroy_session(&session_id.value)?;

    // @step Then the ownership lock file can be locked by another owner
    contender.try_lock()?;
    Ok(())
}

/// BUG-333: releasing ownership on destroy keeps the session listed and
/// resumable — only `persistence_delete_session` evicts it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_destroyed_session_stays_listed_and_resumable() -> support::TestResult {
    // @step Given a live session whose manifest the daemon has committed
    let (_directory, manager, session_id, _) = support::manager().await?;
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    let project = codelet_core::persistence::update_session_manifest(uuid, |manifest| {
        manifest.name = "closed".into();
        Ok(())
    })?
    .project;
    let contender = lock_file(&session_id.value)?;

    // @step When the session is destroyed
    manager.destroy_session(&session_id.value)?;

    // @step Then the ownership lock file can be locked by another owner
    contender.try_lock()?;
    contender.unlock()?;

    // @step And the session is still listed for its project
    let listed = codelet_core::persistence::list_sessions_for_project(&project)?;
    assert!(
        listed.iter().any(|manifest| manifest.id == uuid),
        "a destroyed session must stay listed for its project"
    );

    // @step And resuming the project's last session returns the destroyed session
    let last = codelet_core::persistence::resume_last_session(&project)?;
    assert_eq!(
        last.id, uuid,
        "the destroyed session stays the project's last session"
    );
    Ok(())
}
