//! Feature: spec/features/session-manifest-delete-ownership.feature

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn scenario_competing_processes_cannot_delete_an_owned_session() -> TestResult {
    if let Ok(directory) = std::env::var("CMPCT064_DELETE_CHILD") {
        codelet_common::set_data_directory(directory.into())?;
        reset_stores_for_tests();
        let mut store = SessionStore::new()?;
        let session_id = std::env::var("CMPCT064_DELETE_SESSION")?.parse()?;
        let result = if std::env::var("CMPCT064_DELETE_MODE")? == "store" {
            store.delete(session_id)
        } else {
            delete_session(session_id)
        };
        assert!(
            result.is_err_and(|reason| reason.contains("ownership")),
            "competing deletion must report ownership failure"
        );
        return Ok(());
    }
    // @step Given a process owns a committed session writer lock
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut outcomes = Vec::new();
    for mode in ["store", "facade"] {
        let session = create_session("owned deletion", directory.path())?;
        let path = directory
            .path()
            .join(format!("sessions/{}.json", session.id));
        let before = std::fs::read(&path)?;
        // @step When another process deletes through the store or public facade
        let child = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "scenario_competing_processes_cannot_delete_an_owned_session",
                "--nocapture",
            ])
            .env("CMPCT064_DELETE_CHILD", directory.path())
            .env("CMPCT064_DELETE_SESSION", session.id.to_string())
            .env("CMPCT064_DELETE_MODE", mode)
            .output()?;
        outcomes.push((
            mode,
            child.status.success(),
            std::fs::read(path).is_ok_and(|bytes| bytes == before),
            String::from_utf8_lossy(&child.stderr).into_owned(),
        ));
    }
    // @step Then deletion reports an ownership failure and preserves the committed bytes
    assert!(
        outcomes
            .iter()
            .all(|(_, success, intact, _)| *success && *intact),
        "both deletion APIs must respect the process writer lease: {outcomes:?}"
    );
    Ok(())
}
