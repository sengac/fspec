//! PROV-172: a limit too large for the u32 wire fails limit resolution
//! instead of being truncated.
//!
//! Feature: spec/features/session-model-limits-wire.feature

use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::resolve_current_model_limits;
use serial_test::serial;

/// Restores the named environment variables on drop.
struct EnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvGuard {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(key, value)| {
                let prior = std::env::var_os(key);
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
                (*key, prior)
            })
            .collect();
        Self(saved)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, prior) in self.0.drain(..) {
            match prior {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
#[serial]
fn a_context_window_too_large_for_the_wire_fails_limit_resolution(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an OpenAI selection whose context window override is 5000000000
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let _env = EnvGuard::set(&[
        ("OPENAI_API_KEY", Some("sk-prov172-test")),
        ("OPENAI_BASE_URL", Some("http://127.0.0.1:9")),
        ("OPENAI_CONTEXT_WINDOW", Some("5000000000")),
        ("OPENAI_MAX_OUTPUT_TOKENS", Some("4096")),
    ]);
    let manager =
        ProviderManager::with_provider_and_model("openai", Some("local-model"), None, None)?;
    assert_eq!(manager.context_window(), Some(5_000_000_000));

    // @step When its model limits are resolved for the session
    let resolved = resolve_current_model_limits(&manager);

    // @step Then resolution fails with an error naming the context window, the model and 5000000000
    let error = match &resolved {
        Ok(limits) => return Err(format!("resolution must fail, got {limits:?}").into()),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("context window")
            && error.contains("local-model")
            && error.contains("5000000000"),
        "error must name the limit, the model and the value: {error}"
    );

    // @step And no truncated context window 705032704 is produced
    assert!(
        resolved
            .iter()
            .all(|limits| limits.context_window != Some(705_032_704)),
        "the context window must never be truncated to 705032704"
    );
    Ok(())
}
