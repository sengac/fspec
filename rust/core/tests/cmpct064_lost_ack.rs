//! Feature: spec/features/session-manifest-lost-acknowledgement.feature

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use std::fs;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn lost_acknowledgement_cannot_roll_back_a_promoted_revision() -> TestResult {
    // @step Given a new revision was durably promoted without its acknowledgement reaching the caller
    if let Ok(path) = std::env::var("CMPCT064_LOST_ACK_STORE") {
        codelet_common::set_data_directory(path.into())?;
        reset_stores_for_tests();
        let mut store = SessionStore::new()?;
        let mut session = (*store.list_all().first().ok_or("missing session")?).clone();
        session.name = "promoted".into();
        store.commit(&session)?;
        std::process::exit(74);
    }
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let session = create_session("old", directory.path())?;
    reset_stores_for_tests();
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "lost_acknowledgement_cannot_roll_back_a_promoted_revision",
            "--nocapture",
        ])
        .env("CMPCT064_LOST_ACK_STORE", directory.path())
        .output()?;
    assert_eq!(
        child.status.code(),
        Some(74),
        "writer must terminate after promotion without returning its result: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let path = directory
        .path()
        .join("sessions")
        .join(format!("{}.json", session.id));
    let before = fs::read(&path)?;
    // @step When a fresh store opens the session
    let restored = SessionStore::new()?.load(session.id)?;
    // @step Then it observes the promoted revision without creating another commit
    assert_eq!(restored.name, "promoted");
    assert_eq!(restored.revision, session.revision + 1);
    assert_eq!(fs::read(path)?, before);
    Ok(())
}
