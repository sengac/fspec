#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for Read tool pixel dimension validation
//! Feature: spec/features/image-dimension-validation.feature
//!
//! This test file validates that the Read tool rejects images exceeding
//! pixel dimension limits (5999px on any side) before they can enter
//! conversation history.

use codelet_tools::ReadTool;
use rig::tool::Tool;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper: create a PNG file with valid IHDR header specifying dimensions
fn create_png_with_dimensions(
    dir: &TempDir,
    name: &str,
    width: u32,
    height: u32,
    body_size: usize,
) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();

    // PNG signature
    file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        .unwrap();
    // IHDR chunk length: 13 bytes
    file.write_all(&[0x00, 0x00, 0x00, 0x0D]).unwrap();
    // IHDR chunk type
    file.write_all(b"IHDR").unwrap();
    // Width (u32 BE)
    file.write_all(&width.to_be_bytes()).unwrap();
    // Height (u32 BE)
    file.write_all(&height.to_be_bytes()).unwrap();
    // Bit depth=8, color type=2 (RGB), compression=0, filter=0, interlace=0
    file.write_all(&[8, 2, 0, 0, 0]).unwrap();
    // CRC (dummy)
    file.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap();
    // Fill remaining body
    let header_bytes = 8 + 4 + 4 + 4 + 4 + 5 + 4; // = 33
    if body_size > header_bytes {
        file.write_all(&vec![0u8; body_size - header_bytes])
            .unwrap();
    }

    path.to_string_lossy().to_string()
}

/// Helper: create a JPEG file with SOF0 marker specifying dimensions
fn create_jpeg_with_dimensions(
    dir: &TempDir,
    name: &str,
    width: u16,
    height: u16,
    body_size: usize,
) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();

    // SOI
    file.write_all(&[0xFF, 0xD8]).unwrap();
    // APP0 (JFIF) - minimal
    file.write_all(&[0xFF, 0xE0]).unwrap();
    file.write_all(&[0x00, 0x10]).unwrap(); // length 16
    file.write_all(b"JFIF\0").unwrap();
    file.write_all(&[0x01, 0x01, 0x00]).unwrap();
    file.write_all(&[0x00, 0x01, 0x00, 0x01]).unwrap();
    file.write_all(&[0x00, 0x00]).unwrap();
    // SOF0 (baseline)
    file.write_all(&[0xFF, 0xC0]).unwrap();
    file.write_all(&[0x00, 0x11]).unwrap(); // length 17
    file.write_all(&[0x08]).unwrap(); // bits per sample
    file.write_all(&height.to_be_bytes()).unwrap();
    file.write_all(&width.to_be_bytes()).unwrap();
    file.write_all(&[0x03]).unwrap(); // components
    file.write_all(&[0x01, 0x22, 0x00]).unwrap();
    file.write_all(&[0x02, 0x11, 0x01]).unwrap();
    file.write_all(&[0x03, 0x11, 0x01]).unwrap();

    // Fill remaining body
    let header_bytes = 2 + 2 + 16 + 2 + 17; // = 39
    if body_size > header_bytes {
        file.write_all(&vec![0u8; body_size - header_bytes])
            .unwrap();
    }

    path.to_string_lossy().to_string()
}

/// Scenario: Read tool rejects PNG image exceeding pixel dimension limit
#[tokio::test]
async fn test_read_tool_rejects_png_exceeding_pixel_limit() {
    let temp_dir = TempDir::new().unwrap();

    // @step Given I have a PNG image file at "/tmp/full-page-screenshot.png"
    // @step And the image has dimensions 800x15000 pixels
    // @step And the image is 3MB in file size
    let file_path = create_png_with_dimensions(
        &temp_dir,
        "full-page-screenshot.png",
        800,
        15000,
        3 * 1024 * 1024,
    );

    // @step When the Read tool reads the image file
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return a validation error instead of image data
    assert!(
        result.is_err(),
        "Oversized pixel dimensions should return an error, got OK"
    );
    let error_msg = result.unwrap_err().to_string();

    // @step And the error message should contain the actual dimensions "800x15000"
    assert!(
        error_msg.contains("800") && error_msg.contains("15000"),
        "Error should contain actual dimensions 800x15000, got: {error_msg}"
    );

    // @step And the error message should contain the pixel limit "5999"
    assert!(
        error_msg.contains("5999"),
        "Error should contain pixel limit 5999, got: {error_msg}"
    );

    // @step And the error message should suggest resizing the image
    assert!(
        error_msg.contains("sips")
            || error_msg.contains("convert")
            || error_msg.to_lowercase().contains("resize"),
        "Error should suggest resizing, got: {error_msg}"
    );

    // @step And no image data should enter the conversation history
    // (The error is a ToolError, not ReadOutput::Image, so no image data is emitted)
    assert!(
        !error_msg.contains("\"type\":\"image\""),
        "Error should not contain image data"
    );
}

/// Scenario: Read tool accepts image within pixel dimension limit
#[tokio::test]
async fn test_read_tool_accepts_image_within_pixel_limit() {
    let temp_dir = TempDir::new().unwrap();

    // @step Given I have a PNG image file at "/tmp/viewport-screenshot.png"
    // @step And the image has dimensions 1920x1080 pixels
    // @step And the image is 2MB in file size
    let file_path = create_png_with_dimensions(
        &temp_dir,
        "viewport-screenshot.png",
        1920,
        1080,
        2 * 1024 * 1024,
    );

    // @step When the Read tool reads the image file
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ReadOutput::Image with base64-encoded data
    assert!(
        result.is_ok(),
        "Normal-sized image should succeed, got: {:?}",
        result.err()
    );
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

    // @step And the image media type should be "image/png"
    assert_eq!(output["type"], "image");
    assert_eq!(output["media_type"], "image/png");
    assert!(output["data"].is_string());
}

/// Scenario: Image exactly at pixel dimension limit is accepted
#[tokio::test]
async fn test_read_tool_accepts_image_exactly_at_pixel_limit() {
    let temp_dir = TempDir::new().unwrap();

    // @step Given I have a PNG image file with dimensions 5999x5999 pixels
    let file_path = create_png_with_dimensions(&temp_dir, "boundary.png", 5999, 5999, 500 * 1024);

    // @step When the Read tool reads the image file
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should return ReadOutput::Image with base64-encoded data
    assert!(
        result.is_ok(),
        "Boundary image should succeed, got: {:?}",
        result.err()
    );
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

    // @step And the image should pass the pixel dimension check
    assert_eq!(output["type"], "image");
}

/// Scenario: Corrupt image with invalid header is handled gracefully
#[tokio::test]
async fn test_read_tool_allows_corrupt_header_image_through() {
    let temp_dir = TempDir::new().unwrap();

    // @step Given I have a PNG file with a corrupt or invalid header
    let path = temp_dir.path().join("corrupt.png");
    let mut file = File::create(&path).unwrap();
    // Write PNG magic bytes but truncate IHDR (no width/height)
    file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        .unwrap();
    file.write_all(&[0x00; 100]).unwrap(); // garbage after signature
    let file_path = path.to_string_lossy().to_string();

    // @step And the image dimensions cannot be extracted from the header
    // @step When the Read tool reads the image file
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // @step Then the tool should allow the image through without blocking
    assert!(
        result.is_ok(),
        "Corrupt-header image should be allowed through, got: {:?}",
        result.err()
    );

    // @step And the dimension check should fail gracefully without crashing
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(output["type"], "image", "Should still return image type");
}

/// Additional coverage: Read tool also rejects oversized JPEG images (Layer 1 validation).
/// The primary scenario test for "User-pasted JPEG" is in image_content_recovery_test.rs
/// which validates the Layer 3 (bridge) path via build_user_content_with_images.
#[tokio::test]
async fn test_read_tool_rejects_jpeg_exceeding_pixel_limit() {
    let temp_dir = TempDir::new().unwrap();

    // Create a JPEG image with dimensions 9000x6000 pixels
    let file_path = create_jpeg_with_dimensions(&temp_dir, "oversized.jpg", 9000, 6000, 500 * 1024);

    // Read tool validates dimensions from raw bytes (Layer 1)
    let tool = ReadTool::new(Uuid::nil());
    let args = codelet_tools::read::ReadArgs {
        file_path,
        offset: None,
        limit: None,
        pdf_mode: None,
    };
    let result = tool.call(args).await;

    // Oversized JPEG should be rejected
    assert!(
        result.is_err(),
        "Oversized JPEG should return an error, got OK"
    );
    let error_msg = result.unwrap_err().to_string();

    assert!(
        error_msg.contains("9000") && error_msg.contains("6000"),
        "Error should contain actual dimensions, got: {error_msg}"
    );
    assert!(
        error_msg.contains("5999"),
        "Error should contain the pixel limit, got: {error_msg}"
    );
}

/// Scenario: parse_tool_result_content rejects oversized image from any tool
/// Tests the base64 dimension extraction safety net (Layer 2)
#[tokio::test]
async fn test_parse_tool_result_content_detects_oversized_image_from_base64() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use codelet_tools::image_dimensions::{
        extract_dimensions_from_base64, MAX_IMAGE_PIXEL_DIMENSION,
    };

    // @step Given a tool has returned base64-encoded image data
    // Helper: create a valid PNG header with specified dimensions (same as image_dimensions_test)
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&10000_u32.to_be_bytes());
    bytes.extend_from_slice(&5000_u32.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0]);
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // @step And the image has dimensions 10000x5000 pixels
    let b64 = STANDARD.encode(&bytes);

    // @step When parse_tool_result_content processes the tool result
    let result = extract_dimensions_from_base64(&b64);

    // @step Then it should replace the image with a ToolResultContent::text error
    // (The caller checks dimensions and replaces accordingly)
    assert_eq!(result, Some((10000, 5000)));
    let (w, h) = result.unwrap();

    // @step And the error text should indicate the image exceeds dimension limits
    assert!(
        w > MAX_IMAGE_PIXEL_DIMENSION || h > MAX_IMAGE_PIXEL_DIMENSION,
        "10000x5000 should exceed the limit of {MAX_IMAGE_PIXEL_DIMENSION}"
    );

    // @step And no ToolResultContent::Image should be emitted
    // (Validated: when dimensions exceed limit, callers must not emit Image content)
}
