//! Feature: spec/features/checked-session-creation.feature

use codelet_core::SessionManagerHandle;
use codelet_sessions::SessionManager;
use serial_test::serial;

struct CredentialGuard(Option<std::ffi::OsString>);

impl Drop for CredentialGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(key) => std::env::set_var("ANTHROPIC_API_KEY", key),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn checked_creation_distinguishes_declines_from_application_failures(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a fresh session manager with an offline model catalog
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov101_models.json"),
    )?;
    let _credential = CredentialGuard(std::env::var_os("ANTHROPIC_API_KEY"));
    std::env::set_var("ANTHROPIC_API_KEY", "test-offline-session-storage");
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    // @step When checked creation is attempted before model selection and after the sessions path becomes unusable
    let decline = manager.create_session_checked(None);
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let sessions = directory.path().join("sessions");
    std::fs::write(&sessions, b"not a directory")?;
    let failure = manager.create_session_checked(None);
    // @step Then the missing model returns a no-default-model error while the storage failure returns its own cause and publishes no session
    let decline = decline
        .err()
        .ok_or("a missing default model must be an error, not a session id")?;
    assert!(decline.contains("no default model"), "{decline}");
    let error = failure
        .err()
        .ok_or("a persistence failure must remain an error")?;
    assert!(error.contains("persist session manifest"), "{error}");
    assert!(error.contains("sessions"), "{error}");
    assert!(!error.contains("no default model"), "{error}");
    assert_eq!(manager.session_count().await, 0);
    assert_eq!(std::fs::read(sessions)?, b"not a directory");
    Ok(())
}
