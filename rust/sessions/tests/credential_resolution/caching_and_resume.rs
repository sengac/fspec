//! Feature: spec/features/move-credential-management-to-rust.feature
use super::*;

// =========================================================================
// Mtime-based Caching Tests (Rule 4)
// =========================================================================

#[test]
#[serial]
fn test_cache_credentials_when_file_unchanged() {
    // Scenario: Cache credentials when file unchanged
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let api_key = "cached-api-key";

    setup_credentials_file(&data_dir, "anthropic", api_key);

    // @step Given the CredentialStore has loaded credentials from disk
    init_credential_store_with_dir(&data_dir).unwrap();
    let _ = resolve_credential("anthropic", None, Some(&data_dir));
    reset_disk_read_count(); // Reset after initial load

    // @step And the credentials file mtime has not changed
    // (no file modification)

    // @step When credential resolution is requested
    let read_count_before = get_disk_read_count();
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();
    let read_count_after = get_disk_read_count();

    // @step Then the cached credentials should be returned without reading disk
    assert_eq!(result, Some(api_key.to_string()));
    assert_eq!(
        read_count_before, read_count_after,
        "Should not have read from disk when file unchanged"
    );
}

#[test]
#[serial]
fn test_reload_credentials_when_file_mtime_changes() {
    // Scenario: Reload credentials when file mtime changes
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let old_key = "old-api-key";
    let new_key = "new-api-key";

    // @step Given the CredentialStore has loaded credentials from disk
    setup_credentials_file(&data_dir, "anthropic", old_key);
    init_credential_store_with_dir(&data_dir).unwrap();
    let _ = resolve_credential("anthropic", None, Some(&data_dir));

    // @step And the credentials file is modified with a new API key
    rewrite_credentials_file_later(&data_dir, "anthropic", new_key);

    // @step When credential resolution is requested
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();

    // @step Then the credentials should be reloaded from disk
    // @step And the new API key should be returned
    assert_eq!(
        result,
        Some(new_key.to_string()),
        "Should return new API key after file change"
    );
}

// =========================================================================
// Session Resume Tests (Rule 3)
// =========================================================================

#[test]
#[serial]
fn test_session_resume_picks_up_credential_changes() {
    // Scenario: Session resume picks up credential changes
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let old_key = "old-key";
    let new_key = "new-key";

    // @step Given a Rust session exists with provider "anthropic"
    // @step And the session was created with API key "old-key"
    setup_credentials_file(&data_dir, "anthropic", old_key);
    init_credential_store_with_dir(&data_dir).unwrap();

    // @step And the credentials file is updated with API key "new-key"
    rewrite_credentials_file_later(&data_dir, "anthropic", new_key);

    // @step When the session is resumed
    refresh_credentials_on_resume(&data_dir).unwrap();

    // @step Then credential resolution should be re-executed
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();

    // @step And the new API key "new-key" should be used
    assert_eq!(
        result,
        Some(new_key.to_string()),
        "Should use new API key after session resume"
    );
}

// =========================================================================
// TypeScript Coordination Tests (Rules 5, 6)
// =========================================================================

#[test]
#[serial]
fn test_typescript_save_credential_triggers_rust_reload() {
    // Scenario: TypeScript saveCredential triggers Rust reload
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let old_key = "old-api-key";
    let new_key = "new-api-key";

    // Initialize with old key
    setup_credentials_file(&data_dir, "anthropic", old_key);
    init_credential_store_with_dir(&data_dir).unwrap();

    // Verify old key is resolved
    let initial = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();
    assert_eq!(
        initial,
        Some(old_key.to_string()),
        "Should initially resolve old key"
    );

    // @step Given TypeScript saves a new API key to the credentials file
    rewrite_credentials_file_later(&data_dir, "anthropic", new_key);

    // @step When credentials_reload() function is called
    // credentials_reload() also exports the reloaded keys to the provider
    // env vars; keep the caller's ANTHROPIC_API_KEY intact for later tests.
    let previous_env = env::var("ANTHROPIC_API_KEY").ok();
    let reloaded = codelet_sessions::credentials::credentials_reload();
    let exported_env = env::var("ANTHROPIC_API_KEY").ok();
    match previous_env {
        Some(value) => env::set_var("ANTHROPIC_API_KEY", value),
        None => env::remove_var("ANTHROPIC_API_KEY"),
    }

    // @step Then the CredentialStore should reload from disk
    assert_eq!(
        reloaded,
        Ok(true),
        "credentials_reload() must report that the changed file was reloaded"
    );
    assert_eq!(exported_env.as_deref(), Some(new_key));

    // @step And subsequent credential resolutions should return the new key
    let result = resolve_credential("anthropic", None, Some(&data_dir)).unwrap();
    assert_eq!(
        result,
        Some(new_key.to_string()),
        "Should return new key after credentials_reload()"
    );
}

// =========================================================================
// Session Creation Without API Key Parameter (Rule 5)
// =========================================================================

#[test]
#[serial]
fn test_session_creation_resolves_credentials_internally() {
    // Scenario: Session creation resolves credentials internally
    reset_credential_store(); // Ensure clean state

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let api_key = "internal-api-key";

    // @step Given a credentials file exists with an API key for provider "anthropic"
    setup_credentials_file(&data_dir, "anthropic", api_key);
    init_credential_store_with_dir(&data_dir).unwrap();

    // @step When sessionManagerCreateWithId is called without an api_key parameter
    let resolved = resolve_credential_for_session("anthropic/claude-sonnet-4-20250514", &data_dir);

    // @step Then Rust should resolve the credential internally
    assert!(
        resolved.is_ok(),
        "Rust should resolve credential internally"
    );

    // @step And the session should be created with the resolved API key
    assert_eq!(
        resolved.unwrap(),
        Some(api_key.to_string()),
        "Should create session with resolved API key"
    );
}
