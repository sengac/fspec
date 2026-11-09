//! BUG-411 — session creation and default-model changes report their
//! failures.
//!
//! Feature: spec/features/session-creation-and-default-model-errors.feature

use codelet_sessions::SessionManager;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// A fresh data directory with a seeded models cache, made the process data
/// directory with fresh credential and persistence stores.
fn data_directory() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let data = codelet_test_helpers::shared_temp_subdirectory()?;
    std::fs::create_dir_all(data.join("cache"))?;
    std::fs::write(data.join("cache/models.json"), MODELS_FIXTURE)?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data.clone())?;
    codelet_sessions::credentials::reset_credential_store();
    Ok(data)
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_credential_resolution_failure_fails_session_creation() -> TestResult {
    // @step Given a provider whose stored credentials cannot be read
    let data = data_directory()?;
    std::fs::create_dir_all(data.join("credentials"))?;
    std::fs::write(data.join("credentials/credentials.json"), "{ not json")?;
    let project = codelet_test_helpers::setup_temp_directory()?;
    // @step When a session is created for that provider
    let manager = SessionManager::new();
    let result = manager
        .create_session_with_id(
            &uuid::Uuid::new_v4().to_string(),
            "anthropic/claude-opus-4-5",
            project.path().to_str().ok_or("utf8")?,
            "bug411",
        )
        .await;
    // @step Then creation fails with the credential error
    let error = result.err().unwrap_or_default();
    assert!(error.contains("credentials"), "{error}");
    codelet_sessions::credentials::reset_credential_store();
    Ok(())
}

#[serial_test::serial]
#[test]
fn a_default_model_that_cannot_be_persisted_is_reported() -> TestResult {
    // @step Given a data directory the default model cannot be written into
    let blocker = codelet_test_helpers::shared_temp_subdirectory()?.join("not-a-directory");
    std::fs::write(&blocker, "a file, not a directory")?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(blocker)?;
    // @step When the default model is set
    let result = SessionManager::new().set_default_model("anthropic/claude-opus-4-5");
    // @step Then the call fails with the persistence error
    assert!(result.is_err(), "persisting into a file path must fail");
    Ok(())
}
