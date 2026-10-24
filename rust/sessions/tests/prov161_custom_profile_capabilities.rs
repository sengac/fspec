#![doc = "Feature: spec/features/provider-runtime-custom-profile-capabilities.feature"]

use codelet_sessions::model_resolution;
use fixtures::{Fixture, TestResult};
use serde_json::{json, Value};

#[path = "fixtures/prov161_runtime.rs"]
mod fixtures;

struct ProviderDirectory(Option<std::ffi::OsString>);

impl ProviderDirectory {
    fn set(directory: &std::path::Path) -> Self {
        let previous = std::env::var_os("FSPEC_HOME");
        std::env::set_var("FSPEC_HOME", directory.join("credentials"));
        Self(previous)
    }
}

impl Drop for ProviderDirectory {
    fn drop(&mut self) {
        match &self.0 {
            Some(previous) => std::env::set_var("FSPEC_HOME", previous),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_custom_runtime_profiles_retain_provider_model_capabilities() -> TestResult {
    // @step Given a custom provider has a vision-capable model and a named runtime profile without connection overrides
    let fixture = Fixture::new()?;
    let _environment = ProviderDirectory::set(fixture.directory.path());
    let providers = fixture.directory.path().join("providers");
    std::fs::create_dir_all(&providers)?;
    std::fs::write(
        providers.join("private.json"),
        serde_json::to_vec(&json!({
            "name":"private", "display_name":"Private", "base_url":"http://127.0.0.1:1/v1",
            "facade":"openai", "models":{
                "smart":{"id":"private-vision-model", "supports_vision":true},
                "text":{"id":"private-text-model", "supports_vision":false}
            }
        }))?,
    )?;
    let config_path = fixture.directory.path().join("fspec-config.json");
    let mut config: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    config["providers"]["private"] = json!({
        "settings":{"maxImages":2}, "profiles":{"careful":{"maxImages":7}}
    });
    std::fs::write(&config_path, serde_json::to_vec(&config)?)?;

    // @step When that profile model is selected for a session
    let identifier = fixture.create_with_model("private:careful/smart").await?;

    // @step Then its vision capability and image limit remain those of the custom model and profile
    let background = fixture.manager.get_session(&identifier)?;
    let inner = background.inner.lock().await;
    assert!(model_resolution::resolve_model_vision(
        inner.provider_manager()
    ));
    assert!(codelet_tools::model_capabilities::session_has_capabilities(
        background.id
    ));
    assert!(codelet_tools::model_capabilities::session_supports_vision(
        background.id
    ));
    assert_eq!(
        codelet_tools::model_capabilities::session_model_max_images(background.id),
        Some(7)
    );
    drop(inner);
    let text_identifier = fixture.create_with_model("private:careful/text").await?;
    let text_session = fixture.manager.get_session(&text_identifier)?;
    let inner = text_session.inner.lock().await;
    assert!(!model_resolution::resolve_model_vision(
        inner.provider_manager()
    ));
    assert!(!codelet_tools::model_capabilities::session_supports_vision(
        text_session.id
    ));
    Ok(())
}
