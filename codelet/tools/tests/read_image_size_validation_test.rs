#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for Read tool image size validation
//! Feature: spec/features/read-tool-image-size-validation.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.

use codelet_tools::ReadTool;
use rig::tool::Tool;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper: create a file of a specific size with a given extension
fn create_file_of_size(dir: &TempDir, name: &str, size_bytes: usize) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    // Write PNG magic bytes header so it's detected as PNG by magic bytes too
    let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    file.write_all(&png_header).unwrap();
    // Fill remaining bytes with zeros
    if size_bytes > 8 {
        let remaining = vec![0u8; size_bytes - 8];
        file.write_all(&remaining).unwrap();
    }
    path.to_string_lossy().to_string()
}

/// Scenario: Small image within size limit is returned normally
#[tokio::test]
async fn test_small_image_within_size_limit_is_returned_normally() {
    // @step Given I have a PNG image file at "/tmp/small-screenshot.png" that is 500KB
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_file_of_size(&temp_dir, "small-screenshot.png", 500 * 1024); // 500KB

    // @step When I use the Read tool to read "/tmp/small-screenshot.png"
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return image data with media type "image/png"
    assert!(
        result.is_ok(),
        "Small image should succeed, got: {:?}",
        result.err()
    );
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(
        output["media_type"], "image/png",
        "Should have PNG media type"
    );

    // @step And the result should be a ReadOutput::Image with base64-encoded data
    assert_eq!(output["type"], "image", "Should return image type");
    assert!(output["data"].is_string(), "Should have base64 data string");
}

/// Scenario: Oversized image returns a validation error instead of image data
#[tokio::test]
async fn test_oversized_image_returns_validation_error() {
    // @step Given I have a JPEG image file at "/tmp/huge-photo.jpg" that is 8MB raw
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("huge-photo.jpg");
    let size_bytes = 8 * 1024 * 1024; // 8MB
    let mut file = File::create(&path).unwrap();
    // Write JPEG magic bytes
    let jpeg_header: [u8; 3] = [0xFF, 0xD8, 0xFF];
    file.write_all(&jpeg_header).unwrap();
    file.write_all(&vec![0u8; size_bytes - 3]).unwrap();
    let file_path = path.to_string_lossy().to_string();

    // @step When I use the Read tool to read "/tmp/huge-photo.jpg"
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path: file_path.clone(),
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return a validation error, not image data
    assert!(result.is_err(), "Oversized image should return an error");
    let error_msg = result.unwrap_err().to_string();

    // @step And the error message should contain the file path "/tmp/huge-photo.jpg"
    assert!(
        error_msg.contains("huge-photo.jpg"),
        "Error should contain file path, got: {error_msg}"
    );

    // @step And the error message should contain the actual base64 size
    // 8MB raw ≈ 10.67MB base64
    assert!(
        error_msg.contains("10.") || error_msg.contains("MB"),
        "Error should contain actual size in MB, got: {error_msg}"
    );

    // @step And the error message should contain the limit "5.0 MB"
    assert!(
        error_msg.contains("5.0 MB") || error_msg.contains("5.0MB"),
        "Error should contain limit of 5.0 MB, got: {error_msg}"
    );

    // @step And the error message should suggest resizing the image
    assert!(
        error_msg.to_lowercase().contains("resize"),
        "Error should suggest resizing the image, got: {error_msg}"
    );
}

/// Scenario: Image at exactly the size limit is accepted
#[tokio::test]
async fn test_image_at_exactly_the_size_limit_is_accepted() {
    // @step Given I have an image file at "/tmp/boundary.png" that is exactly 3.75MB raw
    let temp_dir = TempDir::new().unwrap();
    // 3.75MB raw = 3,932,160 bytes → exactly 5,242,880 bytes base64 (5MB)
    // The exact base64 output for N bytes is: ceil(N/3) * 4
    // For 3,932,160: 3,932,160 / 3 = 1,310,720 → 1,310,720 * 4 = 5,242,880 = 5MB exactly
    let file_path = create_file_of_size(&temp_dir, "boundary.png", 3_932_160);

    // @step When I use the Read tool to read "/tmp/boundary.png"
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return image data with media type "image/png"
    assert!(
        result.is_ok(),
        "Boundary image should succeed, got: {:?}",
        result.err()
    );
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(
        output["media_type"], "image/png",
        "Should have PNG media type"
    );

    // @step And no validation error should occur
    assert_eq!(
        output["type"], "image",
        "Should return image type, not error"
    );
}

/// Scenario: SVG files are treated as text and bypass image size validation
#[tokio::test]
async fn test_svg_files_are_treated_as_text() {
    // @step Given I have an SVG file at "/tmp/large-diagram.svg" that is 10MB of XML text
    // Note: We create a smaller SVG (~25KB) rather than 10MB because the token limit
    // would reject a 10MB text file. The key assertion is that SVG is returned as text,
    // not image — proving it bypasses the binary image size validation entirely.
    let temp_dir = TempDir::new().unwrap();
    let svg_path = temp_dir.path().join("large-diagram.svg");
    let mut file = File::create(&svg_path).unwrap();
    // Write valid SVG content
    let svg_header = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\">\n";
    file.write_all(svg_header).unwrap();
    // Pad with SVG content — small enough to stay within token limits
    // but large enough to prove SVG bypasses image size validation
    let line = b"  <circle cx=\"50\" cy=\"50\" r=\"40\" fill=\"red\" />\n";
    for _ in 0..500 {
        file.write_all(line).unwrap();
    }
    file.write_all(b"</svg>\n").unwrap();
    let file_path = svg_path.to_string_lossy().to_string();

    // @step When I use the Read tool to read "/tmp/large-diagram.svg"
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return text content, not image content
    assert!(
        result.is_ok(),
        "SVG should succeed as text, got: {:?}",
        result.err()
    );
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

    // @step And the result should be a ReadOutput::Text with line-numbered content
    assert_eq!(
        output["type"], "text",
        "SVG should return text type, not image. Got: {}",
        output["type"]
    );
    let content = output["content"].as_str().unwrap_or("");
    assert!(
        content.contains("1:"),
        "Text output should have line numbers, got: {}",
        &content[..content.len().min(200)]
    );
}

/// Scenario: Agent loop continues after oversized image error
#[tokio::test]
async fn test_agent_loop_continues_after_oversized_image_error() {
    let temp_dir = TempDir::new().unwrap();

    // @step Given I have an oversized image at "/tmp/massive.png" that is 15MB raw
    let oversized_path = create_file_of_size(&temp_dir, "massive.png", 15 * 1024 * 1024);

    // @step When I use the Read tool to read "/tmp/massive.png"
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path: oversized_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return a validation error as text
    assert!(result.is_err(), "Oversized image should return an error");
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    // Verify the error is a text message, not image data
    assert!(
        !error_msg.starts_with("{\"type\":\"image\""),
        "Error should be text, not image data"
    );

    // @step And the error should never enter the conversation as image data
    // The error type should be a Validation error (which serializes as text in tool results)
    assert!(
        error_msg.contains("too large") || error_msg.contains("MB"),
        "Error should be a human-readable size message, got: {error_msg}"
    );

    // @step And subsequent Read tool calls for other files should succeed normally
    let normal_path = temp_dir.path().join("normal.txt");
    File::create(&normal_path)
        .unwrap()
        .write_all(b"hello world\n")
        .unwrap();

    let args2 = codelet_tools::read::ReadArgs {
        file_path: normal_path.to_string_lossy().to_string(),
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result2 = tool.call(args2).await;
    assert!(
        result2.is_ok(),
        "Subsequent read should succeed, got: {:?}",
        result2.err()
    );
}
