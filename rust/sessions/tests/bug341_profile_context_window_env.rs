//! Feature: spec/features/openai-profile-context-window-env.feature
//!
//! BUG-341 — switching to a profile with no contextWindow must not keep the
//! previous profile's OPENAI_CONTEXT_WINDOW. Hermetic: FSPEC_USER_DIR points
//! at a temp fspec-config.json; every test is serial and restores the
//! variables it touches.

use codelet_sessions::model_resolution::apply_profile_env_vars;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KEYS: [&str; 5] = [
    "OPENAI_CONTEXT_WINDOW",
    "OPENAI_STREAMING",
    "OPENAI_BASE_URL",
    "OPENAI_API_KEY",
    "FSPEC_USER_DIR",
];

/// Restores the process-global variables these tests change.
struct EnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvGuard {
    fn capture() -> Self {
        Self(
            KEYS.iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect(),
        )
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Two openai profiles: "big" declares contextWindow 200000, "small" none.
fn seed_profiles() -> Result<tempfile::TempDir, std::io::Error> {
    let dir = tempfile::tempdir()?;
    let body = r#"{"providers":{"openai":{"profiles":{
        "big":{"baseUrl":"http://127.0.0.1:8001","apiKey":"k","contextWindow":200000},
        "small":{"baseUrl":"http://127.0.0.1:8002","apiKey":"k"}}}}}"#;
    std::fs::write(dir.path().join("fspec-config.json"), body)?;
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    Ok(dir)
}

#[test]
#[serial]
fn a_profile_that_declares_a_context_window_exports_it() -> TestResult {
    let _guard = EnvGuard::capture();
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    // @step Given a stored OpenAI profile "big" whose contextWindow is 200000
    let _config = seed_profiles()?;
    // @step When the profile environment variables are applied for "big"
    apply_profile_env_vars("openai", "big", "m")?;
    // @step Then OPENAI_CONTEXT_WINDOW is "200000"
    assert_eq!(
        std::env::var("OPENAI_CONTEXT_WINDOW").ok().as_deref(),
        Some("200000")
    );
    Ok(())
}

#[test]
#[serial]
fn switching_to_a_profile_without_a_context_window_clears_the_previous_one() -> TestResult {
    let _guard = EnvGuard::capture();
    // @step Given a stored OpenAI profile "big" whose contextWindow is 200000 and a profile "small" with no contextWindow
    let _config = seed_profiles()?;
    // @step And the profile environment variables were applied for "big"
    apply_profile_env_vars("openai", "big", "m")?;
    // @step When the profile environment variables are applied for "small"
    apply_profile_env_vars("openai", "small", "m")?;
    // @step Then OPENAI_CONTEXT_WINDOW is not set
    let leaked = std::env::var("OPENAI_CONTEXT_WINDOW").ok();
    assert_eq!(leaked, None, "small must not inherit big's window");
    Ok(())
}
