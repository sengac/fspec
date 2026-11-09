// Test: Session creation with model validation
//
// Ensures that session creation properly validates model strings and rejects
// invalid formats (e.g., "claude" instead of "anthropic/claude-opus-4-5").
//
// This prevents the bug where sessions were created without proper model
// selection due to missing provider prefix in the model string.
//
// Relocated from the retired codelet-napi crate, whose
// `session_bindings::SessionManager` was a re-export of
// `codelet_sessions::session_manager::SessionManager`.

use codelet_sessions::SessionManager;
use serial_test::serial;
use std::env;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FORMAT_ERROR: &str = "must be in 'provider/model-id' format";

/// Point the process-global data directory at a fresh temp dir so sessions
/// created by these tests never touch the real user data directory.
fn isolated_data_dir() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let data_dir = tempfile::tempdir()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    Ok(data_dir)
}

#[tokio::test]
#[serial]
async fn test_session_requires_model_with_provider_prefix() -> TestResult {
    let _data_dir = isolated_data_dir()?;
    // Setup: Set a dummy API key for testing
    env::set_var("ANTHROPIC_API_KEY", "sk-test-key");

    // BUG-405: a manager the test owns (there is no singleton)
    let manager = SessionManager::new();

    // Test 1: Valid model string with provider/model-id format should work
    let valid_id = Uuid::new_v4();
    let valid_result = manager
        .create_session_with_id(
            &valid_id.to_string(),
            "anthropic/claude-sonnet-4",
            "test-project",
            "Test Session Valid",
        )
        .await;

    // Should succeed (or fail for other reasons, but not format validation)
    // We're not checking success here because it requires full provider setup
    // Just checking that the error (if any) is NOT about format
    if let Err(e) = valid_result {
        let error_msg = e;
        assert!(
            !error_msg.contains(FORMAT_ERROR),
            "Valid format should not trigger format error, got: {error_msg}"
        );
    }

    // Test 2: Invalid model string without provider prefix should fail
    let invalid_id = Uuid::new_v4();
    let invalid_result = manager
        .create_session_with_id(
            &invalid_id.to_string(),
            "claude",
            "test-project",
            "Test Session Invalid",
        )
        .await;

    // Should fail with format validation error
    let Err(error) = invalid_result else {
        return Err("Session creation with invalid model format should fail".into());
    };
    let error_msg = error;
    assert!(
        error_msg.contains(FORMAT_ERROR),
        "Error should mention required format, got: {error_msg}"
    );
    assert!(
        error_msg.contains("claude"),
        "Error should mention the invalid model string, got: {error_msg}"
    );
    Ok(())
}

// Note: Watcher session testing removed as it requires SupervisorRole setup
// The regular session test above covers the same validation logic

#[tokio::test]
#[serial]
async fn test_model_format_validation_examples() -> TestResult {
    let _data_dir = isolated_data_dir()?;
    env::set_var("ANTHROPIC_API_KEY", "sk-test-key");

    let manager = SessionManager::new();

    // Test various invalid formats
    let invalid_formats = [
        ("", "Empty string"),
        ("anthropic", "Provider only"),
        ("claude-sonnet-4", "Model only"),
        ("/claude-sonnet-4", "Leading slash"),
        ("anthropic/", "Trailing slash"),
    ];

    for (model_str, description) in invalid_formats {
        let session_id = Uuid::new_v4();
        let result = manager
            .create_session_with_id(
                &session_id.to_string(),
                model_str,
                "test-project",
                &format!("Test {description}"),
            )
            .await;

        let Err(e) = result else {
            return Err(format!("Invalid format '{model_str}' ({description}) should fail").into());
        };
        let error_msg = e.to_string();
        assert!(
            error_msg.contains(FORMAT_ERROR),
            "Error for '{model_str}' ({description}) should mention format requirement, got: {error_msg}"
        );
    }

    // Test valid formats (may fail for other reasons, but not format)
    let valid_formats = [
        "anthropic/claude-opus-4-5",
        "google/gemini-2.0-flash",
        "openai/gpt-4o",
        "zai/deepseek-v3",
    ];

    for model_str in valid_formats {
        let session_id = Uuid::new_v4();
        let result = manager
            .create_session_with_id(
                &session_id.to_string(),
                model_str,
                "test-project",
                &format!("Test {model_str}"),
            )
            .await;

        // If it fails, it should NOT be due to format validation
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains(FORMAT_ERROR),
                "Valid format '{model_str}' should not fail format validation, got: {error_msg}"
            );
        }
    }
    Ok(())
}
