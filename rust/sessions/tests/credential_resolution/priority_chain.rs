//! Feature: spec/features/move-credential-management-to-rust.feature
use super::*;

// =========================================================================
// Priority Chain Tests (Rules 1, 2)
// =========================================================================

#[test]
#[serial]
fn test_resolve_credential_from_credentials_file() {
    // Scenario: Resolve credential from credentials file
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let api_key = "test-api-key-from-file";

    // @step Given a credentials file exists with an API key for provider "anthropic"
    setup_credentials_file(&data_dir, "anthropic", api_key);

    // @step And no ANTHROPIC_API_KEY environment variable is set
    clear_anthropic_env_vars();

    // @step When credential resolution is requested for provider "anthropic"
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();

    // @step Then the API key from the credentials file should be returned
    assert_eq!(
        result,
        Some(api_key.to_string()),
        "Should return API key from credentials file"
    );
}

#[test]
#[serial]
fn test_resolve_credential_from_environment_variable() {
    // Scenario: Resolve credential from environment variable when file has no key
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let api_key = "test-api-key-from-env";

    // @step Given no API key exists in the credentials file for provider "anthropic"
    setup_empty_credentials_file(&data_dir);

    // @step And the ANTHROPIC_API_KEY environment variable is set
    env::set_var("ANTHROPIC_API_KEY", api_key);

    // @step When credential resolution is requested for provider "anthropic"
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();

    // @step Then the API key from the environment variable should be returned
    assert_eq!(
        result,
        Some(api_key.to_string()),
        "Should return API key from environment variable"
    );

    // Cleanup
    clear_anthropic_env_vars();
}

#[test]
#[serial]
fn test_resolve_credential_from_dotenv_file() {
    // Scenario: Resolve credential from project .env file as fallback
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let project_dir = TempDir::new().unwrap();
    let project_path = project_dir.path().to_path_buf();
    let api_key = "test-api-key-from-dotenv";

    // @step Given no API key exists in the credentials file for provider "anthropic"
    setup_empty_credentials_file(&data_dir);

    // @step And no ANTHROPIC_API_KEY environment variable is set
    clear_anthropic_env_vars();

    // @step And a .env file exists in the project directory with ANTHROPIC_API_KEY
    setup_dotenv_file(&project_path, "ANTHROPIC_API_KEY", api_key);

    // @step When credential resolution is requested for provider "anthropic" with project path
    let result =
        resolve_credential("anthropic", Some(project_path.as_path()), Some(&data_dir)).unwrap();

    // @step Then the API key from the .env file should be returned
    assert_eq!(
        result,
        Some(api_key.to_string()),
        "Should return API key from .env file"
    );
}

#[test]
#[serial]
fn test_return_no_credential_when_no_source_has_key() {
    // Scenario: Return no credential when no source has the key
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // @step Given no API key exists in any credential source for provider "anthropic"
    setup_empty_credentials_file(&data_dir);
    clear_anthropic_env_vars();
    // The PROV-026 claude_auth.json fallback reads FSPEC_HOME; point it at the
    // empty temp dir so a real login on this machine is not picked up.
    let saved_home = env::var("FSPEC_HOME").ok();
    env::set_var("FSPEC_HOME", &data_dir);

    // @step When credential resolution is requested for provider "anthropic"
    let result = resolve_credential("anthropic", None, Some(&data_dir));
    match saved_home {
        Some(home) => env::set_var("FSPEC_HOME", home),
        None => env::remove_var("FSPEC_HOME"),
    }
    let result = result.unwrap();

    // @step Then no API key should be returned
    assert!(
        result.is_none(),
        "Should return None when no credential source has the key"
    );
}

// =========================================================================
// Provider Extraction Tests
// =========================================================================

#[test]
fn test_extract_provider_from_model_string() {
    // Scenario: Extract provider from model string

    // @step Given a model string "anthropic/claude-sonnet-4-20250514"
    let model = "anthropic/claude-sonnet-4-20250514";

    // @step When a session is created with this model
    let provider = extract_provider_from_model(model);

    // @step Then the provider "anthropic" should be extracted
    assert_eq!(
        provider, "anthropic",
        "Should extract 'anthropic' from model string"
    );

    // @step And credential resolution should use "anthropic" as the provider ID
    // Verified by the extract function returning the correct provider
}
