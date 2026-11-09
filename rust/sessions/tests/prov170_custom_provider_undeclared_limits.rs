//! PROV-170: a custom provider model that declares no context window is listed
//! without one — the session wire no longer carries a guessed 200000.
//!
//! Feature: spec/features/custom-provider-napi-model-limits.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use serial_test::serial;

/// Restores the process environment and working directory on drop.
struct ProcessGuard {
    vars: Vec<(&'static str, Option<std::ffi::OsString>)>,
    cwd: PathBuf,
}

impl ProcessGuard {
    fn new(vars: &[(&'static str, &Path)], cwd: &Path) -> Self {
        let saved = vars
            .iter()
            .map(|(key, value)| {
                let prior = std::env::var_os(key);
                std::env::set_var(key, value);
                (*key, prior)
            })
            .collect();
        let prior_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(cwd).expect("set cwd");
        Self {
            vars: saved,
            cwd: prior_cwd,
        }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.cwd);
        for (key, prior) in self.vars.drain(..) {
            match prior {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
#[serial]
fn session_provider_listing_sends_no_context_window_for_a_custom_model_that_declares_none() {
    // @step Given a custom provider "acme" whose model "opus-4.7" declares no context_window
    let home = tempfile::tempdir().expect("home");
    let project = tempfile::tempdir().expect("project");
    let user_dir = tempfile::tempdir().expect("user dir");
    let codex_home = tempfile::tempdir().expect("codex home");
    let credentials = home.path().join(".fspec").join("credentials");
    fs::create_dir_all(&credentials).expect("credentials dir");
    let providers_dir = project.path().join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).expect("providers dir");
    fs::write(
        providers_dir.join("acme.json"),
        serde_json::json!({
            "name": "acme",
            "display_name": "Acme",
            "facade": "claude",
            "base_url": "https://api.example.test",
            "api_key_env_var": "ACME_API_KEY",
            "models": { "opus-4.7": { "id": "acme-opus-4-7" } }
        })
        .to_string(),
    )
    .expect("write provider config");
    let _guard = ProcessGuard::new(
        &[
            ("HOME", home.path()),
            ("FSPEC_HOME", &credentials),
            ("FSPEC_USER_DIR", user_dir.path()),
            ("CODEX_HOME", codex_home.path()),
            ("ACME_API_KEY", Path::new("sk-acme-test")),
        ],
        project.path(),
    );
    let handle: Arc<dyn SessionManagerHandle> =
        SessionManager::new() as Arc<dyn SessionManagerHandle>;

    // @step When the session manager lists providers
    let providers = handle.list_providers().expect("the provider list is read");

    // @step Then the "opus-4.7" model entry has no context window
    let entry = providers
        .iter()
        .flat_map(|provider| provider.models.iter())
        .find(|model| model.id == "opus-4.7" && model.is_custom)
        .unwrap_or_else(|| panic!("custom model opus-4.7 missing from {providers:?}"));
    assert_eq!(entry.context_window, None);
}
