//! BUG-612: deleting a profile removes the environment it set; a credential
//! delete still unsets the provider's variables (PROV-133).
//!
//! Feature: spec/features/deleting-a-profile-removes-the-environment-it-set.feature
//!
//! A real SessionManager over a temp FSPEC_USER_DIR (fspec-config.json with
//! openai profiles) and data directory; serialized because the process
//! environment is global.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::model_resolution::apply_profile_env_vars;
use codelet_sessions::SessionManager;
use serde_json::json;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KEY: &str = "OPENAI_API_KEY";
const BASE_URL: &str = "OPENAI_BASE_URL";

/// Openai profiles "local" (sk-a / sk-test) and "other" (sk-b), with the
/// environment starting from `key` and no base URL.
fn setup(
    dir: &Path,
    local_key: &str,
    key: Option<&str>,
) -> Result<Arc<dyn SessionManagerHandle>, Box<dyn std::error::Error>> {
    let config = json!({"providers": {"openai": {"profiles": {
        "local": {"baseUrl": "http://localhost:8000/v1", "apiKey": local_key},
        "other": {"baseUrl": "http://localhost:9000/v1", "apiKey": "sk-b"}
    }}}});
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&config)?,
    )?;
    std::env::set_var("FSPEC_USER_DIR", dir);
    codelet_common::set_data_directory(dir.to_path_buf())?;
    for var in [KEY, BASE_URL, "OPENAI_CONTEXT_WINDOW", "OPENAI_STREAMING"] {
        std::env::remove_var(var);
    }
    if let Some(key) = key {
        std::env::set_var(KEY, key);
    }
    Ok(SessionManager::new())
}

fn key() -> Option<String> {
    std::env::var(KEY).ok()
}

fn apply(profile: &str) -> TestResult {
    apply_profile_env_vars("openai", profile, "m1")?;
    Ok(())
}

#[test]
#[serial]
fn deleting_the_applied_profile_removes_its_key_from_the_environment() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given OPENAI_API_KEY is unset and an openai profile "local" with key sk-test is applied
    let handle = setup(tmp.path(), "sk-test", None)?;
    apply("local")?;
    assert_eq!(key().as_deref(), Some("sk-test"));

    // @step When the profile "local" is deleted
    handle.delete_profile("openai", "local")?;

    // @step Then OPENAI_API_KEY and OPENAI_BASE_URL are unset
    assert_eq!(key(), None);
    assert_eq!(std::env::var(BASE_URL).ok(), None);
    // @step And list_provider_credentials no longer reports openai as configured
    let openai = handle
        .list_provider_credentials()?
        .into_iter()
        .find(|info| info.provider_id == "openai")
        .ok_or("openai is not listed")?;
    assert!(!openai.configured, "{openai:?}");
    Ok(())
}

#[test]
#[serial]
fn deleting_a_profile_restores_the_key_the_user_exported() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given OPENAI_API_KEY is user-key when the daemon starts
    let handle = setup(tmp.path(), "sk-test", Some("user-key"))?;
    // @step And the openai profile "local" with key sk-test is applied
    apply("local")?;
    assert_eq!(key().as_deref(), Some("sk-test"));

    // @step When the profile "local" is deleted
    handle.delete_profile("openai", "local")?;

    // @step Then OPENAI_API_KEY is user-key
    assert_eq!(key().as_deref(), Some("user-key"));
    Ok(())
}

#[test]
#[serial]
fn deleting_a_profile_that_set_nothing_leaves_the_users_key() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given OPENAI_API_KEY is user-key when the daemon starts and no profile is applied
    let handle = setup(tmp.path(), "sk-test", Some("user-key"))?;

    // @step When the openai profile "local" is deleted
    handle.delete_profile("openai", "local")?;

    // @step Then OPENAI_API_KEY is user-key
    assert_eq!(key().as_deref(), Some("user-key"));
    Ok(())
}

#[test]
#[serial]
fn deleting_an_earlier_profile_never_brings_its_key_back() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given OPENAI_API_KEY is unset, the profile "local" with key sk-a is applied, then the profile "other" with key sk-b
    let handle = setup(tmp.path(), "sk-a", None)?;
    apply("local")?;
    apply("other")?;

    // @step When the profile "local" is deleted
    handle.delete_profile("openai", "local")?;
    // @step Then OPENAI_API_KEY is sk-b
    assert_eq!(key().as_deref(), Some("sk-b"));

    // @step When the profile "other" is deleted
    handle.delete_profile("openai", "other")?;
    // @step Then OPENAI_API_KEY is unset
    assert_eq!(key(), None);
    Ok(())
}

#[test]
#[serial]
fn a_credential_delete_still_unsets_the_providers_key() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given OPENAI_API_KEY is user-key when the daemon starts and the profile "local" with key sk-test is applied
    let handle = setup(tmp.path(), "sk-test", Some("user-key"))?;
    apply("local")?;

    // @step When the openai credentials are deleted
    handle.delete_provider_credentials("openai")?;
    // @step Then OPENAI_API_KEY is unset
    assert_eq!(key(), None);

    // @step When the profile "local" is deleted
    handle.delete_profile("openai", "local")?;
    // @step Then OPENAI_API_KEY is still unset
    assert_eq!(key(), None);
    Ok(())
}
