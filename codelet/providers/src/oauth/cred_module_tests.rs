//! Tests for the `cred::` Rhai building-block module (PROV-086).
//!
//! Feature: spec/features/rhai-cred-namespace-for-scripts.feature
//!
//! Each test maps directly to a Gherkin scenario with `@step` comments
//! matching the scenario step text.  Tests exercise the real filesystem
//! (via a per-test `tempfile::TempDir` + `FSPEC_HOME` env override).
//!
//! Tests that mutate `FSPEC_HOME` serialise through `env_lock()` to
//! prevent cross-test env races inside the single-process test binary.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use rhai::Dynamic;

use crate::oauth::building_blocks::{
    fspec_home, register_all_modules_for_provider,
};
use crate::oauth::engine::{build_default_engine, build_provider_engine};

/// RAII guard serialising all tests that mutate `FSPEC_HOME`.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Set `FSPEC_HOME` to a fresh temp directory, return it plus a drop-guard
/// that restores the previous value.
fn with_temp_fspec_home() -> (tempfile::TempDir, FspecHomeGuard) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let guard = FspecHomeGuard {
        original: std::env::var("FSPEC_HOME").ok(),
    };
    std::env::set_var("FSPEC_HOME", dir.path());
    (dir, guard)
}

struct FspecHomeGuard {
    original: Option<String>,
}

impl Drop for FspecHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => std::env::set_var("FSPEC_HOME", v),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

// =====================================================================
// Scenario: cred::write persists a map as JSON with 0600 permissions
// =====================================================================
#[test]
fn cred_write_persists_map_with_secure_permissions() {
    let _lock = env_lock();
    // @step Given FSPEC_HOME is set to a temporary directory
    let (_dir, _env) = with_temp_fspec_home();

    // @step And a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step When a script calls cred::write("acme", #{access_token: "abc", refresh_token: "def"})
    let _ = engine
        .eval::<Dynamic>(
            r#"cred::write("acme", #{access_token: "abc", refresh_token: "def"})"#,
        )
        .expect("cred::write should succeed");

    // @step Then the file acme.json is created under FSPEC_HOME with 0600 permissions on Unix
    let path: PathBuf = fspec_home().join("acme.json");
    assert!(path.exists(), "acme.json should exist at {}", path.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }

    // @step And the file parses as a JSON object containing access_token and refresh_token
    let contents = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(json["access_token"], "abc");
    assert_eq!(json["refresh_token"], "def");
}

// =====================================================================
// Scenario: cred::read returns the written map
// =====================================================================
#[test]
fn cred_read_returns_written_map() {
    let _lock = env_lock();
    // @step Given FSPEC_HOME is set to a temporary directory
    let (_dir, _env) = with_temp_fspec_home();

    // @step And a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step And a script has previously called cred::write("acme", #{access_token: "abc"})
    let _ = engine
        .eval::<Dynamic>(r#"cred::write("acme", #{access_token: "abc"})"#)
        .expect("write");

    // @step When a script calls cred::read("acme")
    let result: Dynamic = engine
        .eval(r#"cred::read("acme")"#)
        .expect("cred::read should succeed");

    // @step Then the script receives a Map whose access_token equals "abc"
    let map = result.cast::<rhai::Map>();
    let token = map
        .get("access_token")
        .expect("access_token present")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(token, "abc");
}

// =====================================================================
// Scenario: cred::read returns unit when the credential file is absent
// =====================================================================
#[test]
fn cred_read_returns_unit_when_missing() {
    let _lock = env_lock();
    // @step Given FSPEC_HOME is set to a temporary directory
    let (_dir, _env) = with_temp_fspec_home();

    // @step And a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step And no credential file exists for "acme"
    assert!(!fspec_home().join("acme.json").exists());

    // @step When a script calls cred::read("acme")
    let result: Dynamic = engine
        .eval(r#"cred::read("acme")"#)
        .expect("cred::read should not error on missing file");

    // @step Then the script receives unit ()
    assert!(result.is_unit(), "expected unit, got {result:?}");
}

// =====================================================================
// Scenario: cred::read rejects a name that does not match the active
// provider
// =====================================================================
#[test]
fn cred_read_rejects_foreign_provider_name() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step When a script calls cred::read("other_provider_auth")
    let err = engine
        .eval::<Dynamic>(r#"cred::read("other_provider_auth")"#)
        .expect_err("foreign provider must be rejected");

    // @step Then the engine returns a runtime error mentioning access denied and the active provider name
    let message = format!("{err}");
    assert!(
        message.contains("access denied"),
        "error should mention access denied: {message}"
    );
    assert!(
        message.contains("acme"),
        "error should mention active provider: {message}"
    );
}

// =====================================================================
// Scenario: cred::write rejects a path-traversal name
// =====================================================================
#[test]
fn cred_write_rejects_path_traversal_name() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step When a script calls cred::write("../../etc/passwd", #{})
    let err = engine
        .eval::<Dynamic>(r#"cred::write("../../etc/passwd", #{})"#)
        .expect_err("path traversal must be rejected");

    // @step Then the engine returns a runtime error and no file is written outside FSPEC_HOME
    let message = format!("{err}");
    assert!(
        message.contains("access denied"),
        "error should mention access denied: {message}"
    );
    // Double-check that nothing escaped.  The traversal target must not
    // exist, but we can at least assert the FSPEC_HOME directory contains
    // no files at all.
    let count = std::fs::read_dir(fspec_home())
        .map(|it| it.count())
        .unwrap_or(0);
    assert_eq!(count, 0, "no files should be created on traversal attempt");
}

// =====================================================================
// Scenario: cred::path returns the absolute credential path without
// touching the filesystem
// =====================================================================
#[test]
fn cred_path_returns_absolute_path_without_side_effects() {
    let _lock = env_lock();
    // @step Given FSPEC_HOME is set to a temporary directory
    let (dir, _env) = with_temp_fspec_home();

    // @step And a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step When a script calls cred::path("acme")
    let path: String = engine
        .eval(r#"cred::path("acme")"#)
        .expect("cred::path should succeed");

    // @step Then the script receives a string ending with "acme.json" inside FSPEC_HOME
    let expected_suffix = "acme.json";
    assert!(
        path.ends_with(expected_suffix),
        "{path} should end with {expected_suffix}"
    );
    let fspec_home_str = dir.path().to_string_lossy().into_owned();
    assert!(
        path.starts_with(&*fspec_home_str),
        "{path} should start with FSPEC_HOME {fspec_home_str}"
    );

    // @step And no file is created on disk
    assert!(!PathBuf::from(&path).exists(), "cred::path must not create files");
}

// =====================================================================
// Scenario: cred::delete is idempotent when the credential file is
// absent
// =====================================================================
#[test]
fn cred_delete_is_idempotent_when_missing() {
    let _lock = env_lock();
    // @step Given FSPEC_HOME is set to a temporary directory
    let (_dir, _env) = with_temp_fspec_home();

    // @step And a provider engine bound to provider name "acme"
    let engine = build_provider_engine("acme");

    // @step And no credential file exists for "acme"
    assert!(!fspec_home().join("acme.json").exists());

    // @step When a script calls cred::delete("acme")
    let result = engine.eval::<Dynamic>(r#"cred::delete("acme")"#);

    // @step Then the call succeeds without error
    assert!(result.is_ok(), "cred::delete should be idempotent: {result:?}");
}

// =====================================================================
// Scenario: default engine does not expose the cred namespace
// =====================================================================
#[test]
fn default_engine_does_not_expose_cred_namespace() {
    // @step Given an engine built via build_default_engine()
    let engine = build_default_engine();

    // @step When a script attempts to call cred::path("anything")
    let result = engine.eval::<Dynamic>(r#"cred::path("anything")"#);

    // @step Then the engine returns an error because the cred module is not registered
    assert!(
        result.is_err(),
        "default engine must not expose cred::, got {result:?}"
    );
}

// =====================================================================
// Scenario: provider engine registers cred alongside the other building
// block modules
// =====================================================================
#[test]
fn provider_engine_registers_cred_and_other_modules() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given an engine built via build_provider_engine("acme")
    let engine = build_provider_engine("acme");

    // @step When a script calls oauth::generate_state(), crypto::sha256("x"), json::parse("{}"), and cred::path("acme") in sequence
    let script = r#"
        let _s = oauth::generate_state();
        let _h = crypto::sha256("x");
        let _j = json::parse("{}");
        let p = cred::path("acme");
        p
    "#;
    let path: String = engine
        .eval(script)
        .expect("all module calls should succeed");

    // @step Then each call succeeds, confirming http, crypto, json, oauth, and cred modules are all registered
    assert!(path.ends_with("acme.json"));

    // Also verify http:: is registered (namespace present even though we
    // cannot make a real HTTP call in the test).  We do this by
    // introspecting register_all_modules_for_provider.
    let modules = register_all_modules_for_provider("acme");
    let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"http"));
    assert!(names.contains(&"crypto"));
    assert!(names.contains(&"json"));
    assert!(names.contains(&"oauth"));
    assert!(names.contains(&"cred"));
}
