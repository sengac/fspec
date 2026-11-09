//! PROV-175 — profile config numbers are validated instead of coerced.
//!
//! Feature: spec/features/profile-numbers-are-validated.feature

use codelet_sessions::profile_sections::load_local_server_profiles;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An isolated user directory whose fspec-config.json declares profile
/// "local" with `contextWindow` written as `number` (raw JSON).
fn user_dir_with_context_window(number: &str) -> Result<tempfile::TempDir, std::io::Error> {
    let user = tempfile::tempdir()?;
    std::fs::write(
        user.path().join("fspec-config.json"),
        format!(
            r#"{{"providers": {{"openai": {{"profiles": {{"local": {{"baseUrl": "http://127.0.0.1:1/v1", "contextWindow": {number}}}}}}}}}}}"#
        ),
    )?;
    std::env::set_var("FSPEC_USER_DIR", user.path());
    Ok(user)
}

#[test]
#[serial]
fn a_fractional_profile_number_is_an_error_naming_the_profile_field_and_value() -> TestResult {
    // @step Given profile "local" declares contextWindow 80.7
    let _user = user_dir_with_context_window("80.7")?;
    // @step When the local server profiles are loaded
    let loaded = load_local_server_profiles();
    // @step Then loading fails naming "local", "contextWindow" and 80.7
    let error = loaded.err().ok_or("the profile loaded")?;
    for part in ["local", "contextWindow", "80.7"] {
        assert!(error.contains(part), "{part} missing from: {error}");
    }
    Ok(())
}

#[test]
#[serial]
fn an_oversized_profile_number_is_an_error_not_saturated() -> TestResult {
    // @step Given profile "local" declares contextWindow 5000000000
    let _user = user_dir_with_context_window("5000000000")?;
    // @step When the local server profiles are loaded
    let loaded = load_local_server_profiles();
    // @step Then loading fails naming 5000000000
    let error = loaded.err().ok_or("the profile loaded")?;
    assert!(error.contains("5000000000"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn an_integral_float_is_the_same_whole_number() -> TestResult {
    // @step Given profile "local" declares contextWindow 128000.0
    let _user = user_dir_with_context_window("128000.0")?;
    // @step When the local server profiles are loaded
    let profiles = load_local_server_profiles()?;
    // @step Then profile "local" has a context window of 128000
    let local = profiles
        .iter()
        .find(|profile| profile.name == "local")
        .ok_or("profile local is missing")?;
    assert_eq!(local.context_window, Some(128_000));
    Ok(())
}
