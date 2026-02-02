/**
 * Test: Session creation with model validation
 *
 * Ensures that session creation properly validates model strings and rejects
 * invalid formats (e.g., "claude" instead of "anthropic/claude-opus-4-5").
 *
 * This prevents the bug where sessions were created without proper model
 * selection due to missing provider prefix in the model string.
 */

#[cfg(test)]
mod tests {
    use std::env;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_session_requires_model_with_provider_prefix() {
        // Setup: Set a dummy API key for testing
        env::set_var("ANTHROPIC_API_KEY", "sk-test-key");

        // Get SessionManager instance
        let manager = codelet_napi::session_manager::SessionManager::instance();

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
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("must be in 'provider/model-id' format"),
                "Valid format should not trigger format error, got: {}",
                error_msg
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
        assert!(
            invalid_result.is_err(),
            "Session creation with invalid model format should fail"
        );

        let error = invalid_result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("must be in 'provider/model-id' format"),
            "Error should mention required format, got: {}",
            error_msg
        );
        assert!(
            error_msg.contains("claude"),
            "Error should mention the invalid model string, got: {}",
            error_msg
        );
    }

    // Note: Watcher session testing removed as it requires SessionRole setup
    // The regular session test above covers the same validation logic

    #[tokio::test]
    async fn test_model_format_validation_examples() {
        env::set_var("ANTHROPIC_API_KEY", "sk-test-key");

        let manager = codelet_napi::session_manager::SessionManager::instance();

        // Test various invalid formats
        let invalid_formats = vec![
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
                    &format!("Test {}", description),
                )
                .await;

            assert!(
                result.is_err(),
                "Invalid format '{}' ({}) should fail",
                model_str,
                description
            );

            if let Err(e) = result {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("must be in 'provider/model-id' format"),
                    "Error for '{}' ({}) should mention format requirement, got: {}",
                    model_str,
                    description,
                    error_msg
                );
            }
        }

        // Test valid formats (may fail for other reasons, but not format)
        let valid_formats = vec![
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
                    &format!("Test {}", model_str),
                )
                .await;

            // If it fails, it should NOT be due to format validation
            if let Err(e) = result {
                let error_msg = e.to_string();
                assert!(
                    !error_msg.contains("must be in 'provider/model-id' format"),
                    "Valid format '{}' should not fail format validation, got: {}",
                    model_str,
                    error_msg
                );
            }
        }
    }
}
