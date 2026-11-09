//! BUG-463 — default model read failures are reported.
//!
//! Feature: spec/features/default-model-read-failures-are-reported.feature

use codelet_sessions::default_model_persistence::load_default_model_with_dir;
use codelet_sessions::SessionManager;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A data directory whose default-model.json is malformed.
fn malformed_data_dir() -> Result<tempfile::TempDir, std::io::Error> {
    let data = tempfile::tempdir()?;
    std::fs::write(data.path().join("default-model.json"), "{ not valid json")?;
    Ok(data)
}

#[test]
fn a_malformed_default_model_file_is_an_error_naming_it() -> TestResult {
    // @step Given a data directory whose default-model.json contains "{ not valid json"
    let data = malformed_data_dir()?;
    // @step When the persisted default model is loaded from that data directory
    let loaded = load_default_model_with_dir(data.path());
    // @step Then loading fails naming default-model.json
    let error = loaded.err().ok_or("a default model was loaded")?;
    assert!(error.contains("default-model.json"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn restoring_the_default_model_into_a_session_manager_reports_a_malformed_file() -> TestResult {
    // @step Given the data directory's default-model.json contains "{ not valid json"
    let data = malformed_data_dir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    // @step When a session manager restores the persisted default model
    let manager = SessionManager::new();
    let restored = manager.restore_persisted_default_model();
    // @step Then the restore fails naming default-model.json
    let error = restored.err().ok_or("the default model was restored")?;
    assert!(error.contains("default-model.json"), "{error}");
    // @step And the session manager has no default model
    assert_eq!(manager.get_default_model(), None);
    Ok(())
}
