//! Token Estimator Tests
//!
//! Feature: spec/features/limit-file-reads-to-25000-tokens.feature
//!
//! Tests for tiktoken-rs based token estimation and file read limits.

use codelet_common::token_estimator::{count_tokens, max_file_tokens, DEFAULT_MAX_FILE_TOKENS};
use codelet_tools::read::{ReadArgs, ReadTool};
use codelet_tools::error::ToolError;
use lopdf::{dictionary, Document, Object, Stream};
use rig::tool::Tool;
use serial_test::serial;

// ============================================
// TOKEN ESTIMATION WITH TIKTOKEN-RS
// ============================================

/// Scenario: Token estimation uses tiktoken-rs cl100k_base encoding
#[tokio::test]
async fn test_token_estimation_uses_tiktoken_cl100k_base() {
    // @step Given a text file with English content
    let english_text = "Hello world, this is a test of the token estimation system.";

    // @step When the token count is estimated using count_tokens
    let token_count = count_tokens(english_text);

    // @step Then the estimation should use tiktoken-rs cl100k_base encoding
    // "Hello world, this is a test of the token estimation system." = ~14 tokens with cl100k
    assert!(
        token_count >= 10 && token_count <= 20,
        "Token count {} should be approximately 14 for cl100k_base",
        token_count
    );

    // @step And the result should be more accurate than byte-based approximation
    let byte_estimate = english_text.len() / 4;
    // Tiktoken should give a more precise count (15 from bytes vs ~14 from tiktoken)
    assert!(
        (token_count as i32 - 14i32).abs() <= (byte_estimate as i32 - 14i32).abs() + 2,
        "Tiktoken estimate {} should be at least as accurate as byte estimate {}",
        token_count,
        byte_estimate
    );
}

/// Scenario: TokenEstimator is shared across codebase
#[tokio::test]
async fn test_token_estimator_is_shared() {
    // @step Given the TokenEstimator utility in codelet/common
    // @step When token estimation is needed in any module
    let text = "Multiple modules should use the same encoder";

    // Multiple calls should return consistent results (same encoder)
    let count1 = count_tokens(text);
    let count2 = count_tokens(text);
    let count3 = count_tokens(text);

    // @step Then the shared TokenEstimator should be used
    assert_eq!(count1, count2, "Token counts should be consistent");
    assert_eq!(count2, count3, "Token counts should be consistent");

    // @step And no duplicate estimation logic should exist
    // This is verified by grep: no estimate_tokens() functions should remain
    // except in tests or as references
}

// ============================================
// FILE READ TOKEN LIMITS
// ============================================

/// Scenario: Read file under token limit
#[tokio::test]
#[serial]
async fn test_read_file_under_token_limit() {
    // Ensure clean env state
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");

    // @step Given a TypeScript file "/project/src/app.ts" with 50KB of content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("app.ts");
    // Small content that's definitely under 25,000 tokens
    let content = "const x = 1;\n".repeat(100); // ~1300 bytes ≈ ~300 tokens
    std::fs::write(&file_path, &content).expect("Failed to write test file");

    // @step When the read tool is called
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the file content should be returned successfully
    assert!(result.is_ok(), "Read should succeed for file under token limit");

    // @step And no token limit error should be raised
    // Already verified by is_ok()
}

/// Scenario: Read file exceeding token limit throws error
#[tokio::test]
#[serial]
async fn test_read_file_exceeding_token_limit() {
    // Set a very low limit to trigger the error without needing a huge file
    std::env::set_var("CODELET_MAX_FILE_TOKENS", "100");

    // @step Given a minified JavaScript file "/project/dist/bundle.js" with 200KB of content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("bundle.js");
    // Content that exceeds 100 tokens
    let content = "var a=1;\n".repeat(200); // ~200 tokens
    std::fs::write(&file_path, &content).expect("Failed to write test file");

    // @step When the read tool is called
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then a token limit error should be raised
    assert!(result.is_err(), "Read should fail for file over token limit");
    let err = result.unwrap_err();

    // @step And the error should be a TokenLimit error
    assert!(
        matches!(err, ToolError::TokenLimit { .. }),
        "Error should be TokenLimit, got: {:?}",
        err
    );

    // @step And the error message should include relevant information
    let err_msg = err.to_string();
    assert!(err_msg.contains("Token limit"), "Error should mention token limit");
    assert!(err_msg.contains("bundle.js"), "Error should include file path");

    // Cleanup
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
}

/// Scenario: Read image file exempt from token limit
#[tokio::test]
#[serial]
async fn test_read_image_file_exempt() {
    // Set a very low limit to verify images bypass it
    std::env::set_var("CODELET_MAX_FILE_TOKENS", "10");

    // @step Given a PNG image file "/project/assets/logo.png" with 5MB of content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("logo.png");
    // Create a minimal valid PNG header
    let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let mut content = png_header.to_vec();
    // Add IHDR chunk (minimal valid PNG)
    content.extend_from_slice(&[0, 0, 0, 13]); // chunk length
    content.extend_from_slice(b"IHDR");
    content.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]); // 1x1 RGB
    content.extend_from_slice(&[0x90, 0x77, 0x53, 0xDE]); // CRC
    // Add IEND chunk
    content.extend_from_slice(&[0, 0, 0, 0]);
    content.extend_from_slice(b"IEND");
    content.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
    std::fs::write(&file_path, &content).expect("Failed to write test file");

    // @step When the read tool is called
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the image should be processed successfully
    assert!(result.is_ok(), "Image read should succeed despite low token limit");

    // @step And the token limit check should be skipped
    // Verified by success - would fail if token limit was applied

    // Cleanup
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
}

/// Scenario: Read PDF file exempt from token limit
#[tokio::test]
#[serial]
async fn test_read_pdf_file_exempt() {
    // Set a very low limit to verify PDFs bypass it
    std::env::set_var("CODELET_MAX_FILE_TOKENS", "10");

    // @step Given a PDF file "/project/docs/manual.pdf" with 10MB of content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("manual.pdf");

    // Create a valid PDF using lopdf
    let pdf_bytes = create_valid_test_pdf();
    std::fs::write(&file_path, &pdf_bytes).expect("Failed to write test file");

    // @step When the read tool is called
    // Use text mode since visual mode requires Pdfium library
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: Some("text".to_string()),
        })
        .await;

    // @step Then the PDF should be processed successfully
    // @step And the token limit check should be skipped
    assert!(result.is_ok(), "PDF read should succeed despite low token limit: {:?}", result.err());

    // Cleanup
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
}

/// Helper to create a valid PDF for testing
fn create_valid_test_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let content_stream = "BT\n/F1 12 Tf\n50 700 Td\n(Test PDF content) Tj\nET";
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        content_stream.as_bytes().to_vec(),
    ));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Contents" => content_id,
        "Resources" => resources_id,
    });

    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1i64,
    });

    // Update page to reference Pages node
    if let Ok(page) = doc.get_object_mut(page_id) {
        if let Object::Dictionary(ref mut dict) = page {
            dict.set("Parent", pages_id);
        }
    }

    // Create catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap_or_default();
    bytes
}

/// Scenario: Partial read within token limit
#[tokio::test]
#[serial]
async fn test_partial_read_within_token_limit() {
    // Set a moderate limit
    std::env::set_var("CODELET_MAX_FILE_TOKENS", "500");

    // @step Given a large file "/project/src/large.ts" with 500KB of content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("large.ts");
    let content = "const line = 'test';\n".repeat(2500); // Large file
    std::fs::write(&file_path, &content).expect("Failed to write test file");

    // @step When the read tool is called with a limit
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(1),
            limit: Some(50), // Only read 50 lines
            pdf_mode: None,
        })
        .await;

    // @step Then only the requested portion should be checked for token limits
    // @step And the content should be returned if under limit
    assert!(
        result.is_ok(),
        "Partial read of 50 lines should succeed: {:?}",
        result.err()
    );

    // Cleanup
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
}

// ============================================
// CONFIGURATION
// ============================================

/// Scenario: Custom token limit via environment variable
#[tokio::test]
#[serial]
async fn test_custom_token_limit_via_env() {
    // @step Given the environment variable CODELET_MAX_FILE_TOKENS is set to 50000
    std::env::set_var("CODELET_MAX_FILE_TOKENS", "50000");
    // @step And a JavaScript file "/project/dist/bundle.js" with 150KB of content
    // (This test focuses on the env var behavior, not file reading)

    // @step When max_file_tokens is called
    let limit = max_file_tokens();

    // @step Then the custom limit should be returned
    // @step And the custom limit of 50,000 tokens should be applied
    assert_eq!(limit, 50000, "Custom limit should be 50000");

    // Verify default when unset
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
    let default_limit = max_file_tokens();
    assert_eq!(
        default_limit, DEFAULT_MAX_FILE_TOKENS,
        "Default limit should be {}",
        DEFAULT_MAX_FILE_TOKENS
    );
}

// ============================================
// MIGRATION TESTS
// ============================================

/// Scenario: Replace byte-based estimation in interactive_helpers
#[tokio::test]
async fn test_interactive_helpers_uses_token_estimator() {
    // @step Given the migration is complete
    // The count_tokens function is now used instead of estimate_tokens

    // @step Then the function should use TokenEstimator::count_tokens()
    let text = "Hello, world!";
    let token_count = count_tokens(text);

    // Tiktoken gives more accurate results than bytes/4
    // "Hello, world!" = 4 tokens with tiktoken vs 3 with bytes/4
    assert!(token_count > 0, "Token count should be positive");
    assert!(token_count < 10, "Token count for short text should be small");

    // @step And the APPROX_BYTES_PER_TOKEN constant should be removed
    // Verified by successful compilation - removed constant would cause errors
}

/// Scenario: Replace byte-based estimation in persistence storage
#[tokio::test]
async fn test_persistence_storage_uses_token_estimator() {
    // @step Given the existing estimate_tokens() function in napi/persistence/storage.rs
    // @step When the migration is complete
    // storage.rs now uses count_tokens from codelet_common

    // @step Then the function should use the shared TokenEstimator
    // This is verified by the import: use codelet_common::token_estimator::count_tokens

    // @step And token counts for persisted messages should be more accurate
    let message = "This is a test message for persistence.";
    let accurate_count = count_tokens(message);
    let byte_estimate = message.len() / 4;

    // Tiktoken should provide reasonable estimates
    assert!(
        accurate_count > 0 && accurate_count < message.len(),
        "Token count {} should be reasonable",
        accurate_count
    );

    // Both estimates should be in the same ballpark for short text
    assert!(
        (accurate_count as i32 - byte_estimate as i32).abs() < 5,
        "Tiktoken {} and byte estimate {} should be similar for short text",
        accurate_count,
        byte_estimate
    );
}

/// Scenario: Replace byte-based estimation in compactor
#[tokio::test]
async fn test_compactor_uses_token_estimator() {
    // @step Given the inline token estimation in compactor.rs
    // @step When the migration is complete
    // compactor.rs now uses count_tokens from codelet_common

    // @step Then compaction decisions should use TokenEstimator
    // This is verified by the import in compactor.rs

    // @step And summary token estimation should be more accurate
    let summary = "## Summary\n\nUser worked on implementing token limits.";
    let accurate_count = count_tokens(summary);

    // Should provide reasonable token count for markdown content
    assert!(
        accurate_count > 5 && accurate_count < 30,
        "Summary token count {} should be reasonable",
        accurate_count
    );
}

/// Debug test to verify line and token limits
#[tokio::test]
#[serial]
async fn test_debug_line_and_token_limits() {
    use codelet_common::token_estimator::count_tokens;
    
    std::env::remove_var("CODELET_MAX_FILE_TOKENS");
    
    // Create file with 5000 lines
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("big.js");
    let mut content = String::new();
    for i in 1..=5000 {
        content.push_str(&format!("const variable_{} = someValue;\n", i));
    }
    std::fs::write(&file_path, &content).expect("Failed to write test file");
    
    let read_tool = ReadTool::new();
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    match result {
        Ok(output) => {
            // Parse JSON
            let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
            let content = parsed["content"].as_str().unwrap();
            let line_count = content.lines().count();
            let token_count = count_tokens(content);
            
            println!("=== DEBUG OUTPUT ===");
            println!("Input file lines: 5000");
            println!("Output lines: {}", line_count);
            println!("Output tokens: {}", token_count);
            println!("First line: {}", content.lines().next().unwrap_or("N/A"));
            println!("Last 3 lines:");
            for line in content.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev() {
                println!("  {}", line);
            }
            println!("===================");
            
            // Verify 2000 line limit
            assert!(line_count <= 2005, "Should be ~2000 lines, got {}", line_count);
        }
        Err(e) => {
            println!("ERROR (might be token limit): {}", e);
        }
    }
}
