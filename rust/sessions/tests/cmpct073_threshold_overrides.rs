//! Feature: spec/features/saved-threshold-override-propagation.feature

use std::sync::atomic::Ordering;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;
use serial_test::serial;

struct Environment {
    _directory: tempfile::TempDir,
    previous_user_dir: Option<std::ffi::OsString>,
    manager: SessionManager,
}

impl Environment {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = setup_temp_directory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        let previous_user_dir = std::env::var_os("FSPEC_USER_DIR");
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("OPENAI_API_KEY", "test-threshold-profile");
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("fixtures/prov123_models.json"),
        )?;
        let model = |threshold| {
            json!({
                "id": "threshold-model", "contextWindow": 200000,
                "maxOutputTokens": 8192, "compactionThreshold": threshold
            })
        };
        std::fs::write(
            directory.path().join("fspec-config.json"),
            serde_json::to_vec(&json!({"providers":{"openai":{"profiles":{
                "tokens": {"baseUrl":"http://127.0.0.1:1/v1", "apiKey":"test",
                    "customModels":[model(json!({"type":"tokens","value":40000}))]},
                "percent": {"baseUrl":"http://127.0.0.1:1/v1", "apiKey":"test",
                    "customModels":[model(json!({"type":"percentage","value":35}))]},
                "invalid": {"baseUrl":"http://127.0.0.1:1/v1", "apiKey":"test",
                    "customModels":[model(json!({"type":"percentage","value":300}))]}
            }}}}))?,
        )?;
        Ok(Self {
            _directory: directory,
            previous_user_dir,
            manager: SessionManager::new(),
        })
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        match &self.previous_user_dir {
            Some(value) => std::env::set_var("FSPEC_USER_DIR", value),
            None => std::env::remove_var("FSPEC_USER_DIR"),
        }
        codelet_core::persistence::reset_stores_for_tests();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_session_creation_carries_saved_token_threshold_to_cached_limits(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a saved model profile with an explicit token threshold
    let environment = Environment::new()?;
    // @step When a session is created using that profile
    let id = environment
        .manager
        .create_session("openai:tokens/threshold-model", ".")
        .await?;
    let session = environment.manager.get_session(&id)?;
    let inner = session.inner.lock().await;
    let active = inner.provider_manager().compaction_threshold_override();
    // @step Then its cached threshold matches the active policy without changing capacity
    assert_eq!(
        session.cached_compaction_threshold.load(Ordering::Acquire),
        40000
    );
    assert_eq!(active, Some(("tokens", 40000)));
    assert_eq!(
        session.cached_context_window.load(Ordering::Acquire),
        200000
    );
    assert_eq!(
        session.cached_max_output_tokens.load(Ordering::Acquire),
        8192
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_unresolved_saved_profile_fails_before_session_creation(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an explicit model selection naming an unavailable saved profile
    let environment = Environment::new()?;
    // @step When a session is created using that profile
    let result = environment
        .manager
        .create_session("openai:unavailable/threshold-model", ".")
        .await;
    // @step Then creation fails clearly instead of treating configuration as absent
    assert!(
        result.is_err(),
        "unresolved explicit profile must fail: {result:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_model_switching_refreshes_saved_percentage_threshold(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a session using a saved token threshold
    let environment = Environment::new()?;
    let id = environment
        .manager
        .create_session("openai:tokens/threshold-model", ".")
        .await?;
    // @step When it switches to a profile with a saved percentage threshold
    let handle: &dyn SessionManagerHandle = &environment.manager;
    handle.set_model(
        &codelet_rpc_types::SessionId { value: id.clone() },
        "openai:percent",
        "threshold-model",
    )?;
    let session = environment.manager.get_session(&id)?;
    let inner = session.inner.lock().await;
    // @step Then cached limits and active policy use the new threshold not the old one
    assert_eq!(
        session.cached_compaction_threshold.load(Ordering::Acquire),
        70000
    );
    assert_eq!(
        inner.provider_manager().compaction_threshold_override(),
        Some(("percentage", 35))
    );
    assert_eq!(
        session.cached_context_window.load(Ordering::Acquire),
        200000
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_invalid_saved_explicit_threshold_fails_before_session_creation(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a saved model profile with an invalid explicit threshold
    let environment = Environment::new()?;
    // @step When a session is created using that profile
    let result = environment
        .manager
        .create_session("openai:invalid/threshold-model", ".")
        .await;
    // @step Then creation fails clearly instead of caching a guessed threshold
    assert!(
        result.is_err(),
        "invalid explicit configuration must fail, got {result:?}"
    );
    Ok(())
}
