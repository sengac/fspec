#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-script-shadowing-builtin-providers.feature
//!
//! Integration tests for PROV-085: removes the BUILTIN_PROVIDER_NAMES guard
//! so custom Rhai provider configs may shadow built-in providers, and adds
//! the FSPEC_DISABLE_SCRIPT_SHADOWING escape hatch.
//!
//! The `ProviderConfig::from_file` load-level scenarios live in
//! `custom_config_and_loader_tests.rs` (they do not need an env override).
//! This file covers the shadow-resolution scenarios that touch
//! `ProviderType::from_str` and `custom_provider_registered`.

use std::fs;

use serde_json::json;
use serial_test::serial;
use std::str::FromStr;
use tempfile::TempDir;

#[path = "custom_test_helpers.rs"]
mod helpers;

use codelet_providers::custom::error::CustomProviderError;
use codelet_providers::custom::ProviderConfig;
use codelet_providers::ProviderType;
use helpers::{minimal_cfg, write_valid_script, CwdGuard, EnvGuard};

/// Install a global custom provider config named `slug` in a fresh
/// `$HOME/.fspec/providers/<slug>.json`. Returns the TempDir roots and
/// the env guards that must be kept alive for the lifetime of the test.
#[allow(clippy::type_complexity)]
fn install_shadowing_config(
    slug: &str,
) -> (TempDir, TempDir, EnvGuard, EnvGuard, CwdGuard) {
    let home_tmp = TempDir::new().expect("home tempdir");
    let project_tmp = TempDir::new().expect("project tempdir");

    let fspec_dir = home_tmp.path().join(".fspec");
    let credentials_dir = fspec_dir.join("credentials");
    let global_providers = fspec_dir.join("providers");
    fs::create_dir_all(&credentials_dir).unwrap();
    fs::create_dir_all(&global_providers).unwrap();

    let script = write_valid_script(&global_providers, &format!("{slug}.rhai"));
    let cfg = json!({
        "name": slug,
        "display_name": format!("Shadowing {slug}"),
        "base_url": "https://api.example.com",
        "script": script.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "SHADOW_KEY" },
        "models": { "smart": { "id": "model-smart-v2" } }
    });
    fs::write(
        global_providers.join(format!("{slug}.json")),
        serde_json::to_string(&cfg).unwrap(),
    )
    .unwrap();

    let home_guard = EnvGuard::set("HOME", home_tmp.path());
    let fspec_guard = EnvGuard::set("FSPEC_HOME", &credentials_dir);
    let cwd_guard = CwdGuard::set(project_tmp.path());

    (home_tmp, project_tmp, home_guard, fspec_guard, cwd_guard)
}

// =========================================================================
// Scenario: Load a custom provider config named 'claude' without NameConflict
// =========================================================================
#[test]
fn load_a_custom_provider_config_named_claude_without_nameconflict() {
    // @step Given a valid JSON provider config with name "claude" and a valid .rhai script on disk
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = minimal_cfg(
        "claude",
        &script_path.file_name().unwrap().to_string_lossy(),
    );
    let cfg_path = tmp.path().join("claude.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I call ProviderConfig::from_file on the JSON path
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then the result is Ok and the loaded ProviderConfig has name "claude"
    let loaded = result.expect("shadowing config should load without NameConflict");
    assert_eq!(loaded.name, "claude");
}

// =========================================================================
// Scenario: Load a custom provider config named 'codex' without NameConflict
// =========================================================================
#[test]
fn load_a_custom_provider_config_named_codex_without_nameconflict() {
    // @step Given a valid JSON provider config with name "codex" and a valid .rhai script on disk
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = minimal_cfg(
        "codex",
        &script_path.file_name().unwrap().to_string_lossy(),
    );
    let cfg_path = tmp.path().join("codex.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I call ProviderConfig::from_file on the JSON path
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then the result is Ok and the loaded ProviderConfig has name "codex"
    let loaded = result.expect("shadowing config should load without NameConflict");
    assert_eq!(loaded.name, "codex");
}

// =========================================================================
// Scenario: Shadowing custom config resolves provider slug to Custom variant
// =========================================================================
#[test]
#[serial]
fn shadowing_custom_config_resolves_provider_slug_to_custom_variant() {
    // @step Given a discovered custom provider config with name "claude" is registered in the global providers directory
    let (_home, _project, _home_guard, _fspec_guard, _cwd_guard) =
        install_shadowing_config("claude");

    // @step And the FSPEC_DISABLE_SCRIPT_SHADOWING environment variable is unset
    let _disable_guard = EnvGuard::remove("FSPEC_DISABLE_SCRIPT_SHADOWING");

    // @step When I call ProviderType::from_str("claude")
    let result = ProviderType::from_str("claude").expect("shadowing slug resolves");

    // @step Then the result is ProviderType::Custom("claude")
    match result {
        ProviderType::Custom(name) => assert_eq!(name, "claude"),
        other => panic!("expected ProviderType::Custom(\"claude\"), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Escape hatch env var disables shadowing and restores hardcoded built-in
// =========================================================================
#[test]
#[serial]
fn escape_hatch_env_var_disables_shadowing_and_restores_hardcoded_builtin() {
    // @step Given a discovered custom provider config with name "claude" is registered in the global providers directory
    let (_home, _project, _home_guard, _fspec_guard, _cwd_guard) =
        install_shadowing_config("claude");

    // @step And the FSPEC_DISABLE_SCRIPT_SHADOWING environment variable is set to "1"
    // Snapshot any prior value, set to "1" for the test, and restore on drop.
    let prior_disable = std::env::var("FSPEC_DISABLE_SCRIPT_SHADOWING").ok();
    std::env::set_var("FSPEC_DISABLE_SCRIPT_SHADOWING", "1");
    struct DisableRestore(Option<String>);
    impl Drop for DisableRestore {
        fn drop(&mut self) {
            match self.0.take() {
                Some(v) => std::env::set_var("FSPEC_DISABLE_SCRIPT_SHADOWING", v),
                None => std::env::remove_var("FSPEC_DISABLE_SCRIPT_SHADOWING"),
            }
        }
    }
    let _disable_guard = DisableRestore(prior_disable);

    // @step When I call ProviderType::from_str("claude")
    let result = ProviderType::from_str("claude").expect("claude resolves");

    // @step Then the result is ProviderType::Claude
    assert_eq!(result, ProviderType::Claude, "escape hatch should bypass shadowing");
}

// =========================================================================
// Scenario: Invalid name pattern still fails with InvalidName
// =========================================================================
#[test]
fn invalid_name_pattern_still_fails_with_invalidname() {
    // @step Given a provider config JSON with name "My Provider" containing whitespace and uppercase
    let tmp = TempDir::new().expect("tempdir");
    let script = write_valid_script(tmp.path(), "p.rhai");
    let cfg = json!({
        "name": "My Provider",
        "display_name": "Bad Name",
        "base_url": "https://api.example.com",
        "script": script.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "KEY" },
        "models": { "smart": { "id": "m" } }
    });
    let cfg_path = tmp.path().join("bad-name.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I call ProviderConfig::from_file on the JSON path
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then the result is an InvalidName error mentioning the allowed pattern ^[a-z][a-z0-9-]*$
    let err = result.expect_err("invalid name should fail");
    assert!(
        matches!(err, CustomProviderError::InvalidName { .. }),
        "expected InvalidName, got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("^[a-z][a-z0-9-]*$"),
        "error should include pattern: {msg}"
    );
}
