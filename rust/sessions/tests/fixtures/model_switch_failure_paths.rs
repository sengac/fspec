use super::{Fixture, SessionManagerHandle, TestResult, PROFILE_MODEL};
use std::ffi::OsString;

pub(super) fn request_environment() -> Vec<(&'static str, Option<OsString>)> {
    [
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "OPENAI_MODEL",
        "OPENAI_CONTEXT_WINDOW",
        "OPENAI_STREAMING",
    ]
    .into_iter()
    .map(|key| (key, std::env::var_os(key)))
    .collect()
}

#[cfg(unix)]
pub(super) struct DirectoryPermissions {
    path: std::path::PathBuf,
    original: std::fs::Permissions,
}

#[cfg(unix)]
impl DirectoryPermissions {
    pub(super) fn set(path: &std::path::Path, mode: u32) -> std::io::Result<Self> {
        use std::os::unix::fs::PermissionsExt;
        let guard = Self {
            path: path.to_owned(),
            original: std::fs::metadata(path)?.permissions(),
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
        Ok(guard)
    }
}

#[cfg(unix)]
impl Drop for DirectoryPermissions {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, self.original.clone());
    }
}

/// Feature: spec/features/model-switch-durable-identity.feature
/// @step Given a persisted session whose manifest can be published but not acknowledged
/// @step When the session switches to an OpenAI profile model
/// @step Then the switch succeeds with matching durable and runtime selection and limits
/// @step And the selected profile request environment is retained
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn published_identity_is_adopted_despite_lost_acknowledgement() -> TestResult {
    let fixture = Fixture::new()?;
    let permissions = DirectoryPermissions::set(&fixture.directory.path().join("sessions"), 0o300)?;
    let result = fixture
        .manager
        .set_model(&fixture.session_id, "openai:Strada", PROFILE_MODEL);
    drop(permissions);
    let durable = codelet_core::persistence::SessionStore::new()?.load(fixture.original.id)?;
    assert_eq!(durable.provider, format!("openai:Strada/{PROFILE_MODEL}"));
    assert_eq!(durable.revision, fixture.original.revision + 1);
    assert!(
        result.is_ok(),
        "published selection must be adopted: {result:?}"
    );
    let session = fixture.manager.get_session(&fixture.session_id.value)?;
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.provider_manager().selected_model_string(),
        Some(durable.provider)
    );
    let model = fixture.manager.get_session_model(&fixture.session_id);
    assert_eq!(model.provider_id, "openai:Strada");
    assert_eq!(model.model_id, PROFILE_MODEL);
    assert_eq!(model.context_window, 200000);
    assert_eq!(model.max_output_tokens, 4096);
    assert_eq!(std::env::var("OPENAI_BASE_URL")?, "http://127.0.0.1:1/v1");
    assert_eq!(std::env::var("OPENAI_CONTEXT_WINDOW")?, "200000");
    Ok(())
}

/// Feature: spec/features/model-switch-durable-identity.feature
/// @step Given a persisted session and a saved profile for an unsupported provider
/// @step When the session attempts to switch to that profile
/// @step Then the switch fails without changing the durable identity or runtime selection and limits
/// @step And the prior request environment remains unchanged
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_profile_preparation_preserves_request_environment() -> TestResult {
    let fixture = Fixture::new()?;
    let config_path = fixture.directory.path().join("fspec-config.json");
    let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    config["providers"]["unsupported"] = config["providers"]["openai"].clone();
    std::fs::write(config_path, serde_json::to_vec(&config)?)?;
    let environment = request_environment();
    let result =
        fixture
            .manager
            .set_model(&fixture.session_id, "unsupported:Strada", PROFILE_MODEL);
    assert!(
        result.is_err(),
        "unsupported provider must reject preparation"
    );
    fixture.assert_unchanged().await?;
    assert_eq!(request_environment(), environment);
    Ok(())
}
