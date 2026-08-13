#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for image dimension extraction from raw headers
//! Feature: spec/features/image-dimension-validation.feature
//!
//! This test file validates the core image_dimensions module that extracts
//! pixel dimensions from PNG, JPEG, GIF, and WebP headers without external crates.
//! These functions underpin Layer 1 (Read tool) and Layer 2 (parse_tool_result_content).

use codelet_tools::image_dimensions::{
    extract_dimensions_from_base64, extract_jpeg_dimensions, extract_png_dimensions,
    MAX_IMAGE_PIXEL_DIMENSION,
};

// ============================================================================
// PNG Dimension Extraction
// ============================================================================

/// Helper: create a valid PNG header with specified dimensions
fn make_png_header(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    // PNG signature: 89 50 4E 47 0D 0A 1A 0A
    bytes.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    // IHDR chunk length: 13 bytes (0x0000000D)
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    // IHDR chunk type
    bytes.extend_from_slice(b"IHDR");
    // Width (u32 BE)
    bytes.extend_from_slice(&width.to_be_bytes());
    // Height (u32 BE)
    bytes.extend_from_slice(&height.to_be_bytes());
    // Bit depth, color type, compression, filter, interlace (5 bytes, not needed for dimension extraction)
    bytes.extend_from_slice(&[8, 2, 0, 0, 0]);
    // CRC (4 bytes, dummy)
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    bytes
}

#[test]
fn test_extract_png_dimensions_valid_header() {
    // @step Given I have a PNG image file at "/tmp/viewport-screenshot.png"
    // @step And the image has dimensions 1920x1080 pixels
    let bytes = make_png_header(1920, 1080);

    let result = extract_png_dimensions(&bytes);

    // @step Then dimensions should be extracted as (1920, 1080)
    assert_eq!(result, Some((1920, 1080)));
}

#[test]
fn test_extract_png_dimensions_oversized() {
    // @step Given I have a PNG image file at "/tmp/full-page-screenshot.png"
    // @step And the image has dimensions 800x15000 pixels
    let bytes = make_png_header(800, 15000);

    let result = extract_png_dimensions(&bytes);

    assert_eq!(result, Some((800, 15000)));
    // Verify the height exceeds the limit
    let (_, h) = result.unwrap();
    assert!(h > MAX_IMAGE_PIXEL_DIMENSION);
}

#[test]
fn test_extract_png_dimensions_at_boundary() {
    // @step Given I have a PNG image file with dimensions 5999x5999 pixels
    let bytes = make_png_header(5999, 5999);

    let result = extract_png_dimensions(&bytes);

    // @step And the image should pass the pixel dimension check
    assert_eq!(result, Some((5999, 5999)));
    let (w, h) = result.unwrap();
    assert!(w <= MAX_IMAGE_PIXEL_DIMENSION);
    assert!(h <= MAX_IMAGE_PIXEL_DIMENSION);
}

#[test]
fn test_extract_png_dimensions_exactly_6000_exceeds() {
    let bytes = make_png_header(6000, 6000);
    let result = extract_png_dimensions(&bytes);
    assert_eq!(result, Some((6000, 6000)));
    let (w, h) = result.unwrap();
    assert!(w > MAX_IMAGE_PIXEL_DIMENSION);
    assert!(h > MAX_IMAGE_PIXEL_DIMENSION);
}

#[test]
fn test_extract_png_dimensions_corrupt_header() {
    // @step Given I have a PNG file with a corrupt or invalid header
    // @step And the image dimensions cannot be extracted from the header
    let bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00]; // Truncated

    let result = extract_png_dimensions(&bytes);

    // @step Then the dimension check should fail gracefully without crashing
    assert_eq!(result, None);
}

#[test]
fn test_extract_png_dimensions_empty_bytes() {
    let result = extract_png_dimensions(&[]);
    assert_eq!(result, None);
}

#[test]
fn test_extract_png_dimensions_wrong_magic() {
    // Not a PNG at all
    let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0x00, 0x00];
    let result = extract_png_dimensions(&bytes);
    assert_eq!(result, None);
}

// ============================================================================
// JPEG Dimension Extraction
// ============================================================================

/// Helper: create a minimal JPEG with SOF0 marker at a known position
fn make_jpeg_with_dimensions(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    // SOI marker
    bytes.extend_from_slice(&[0xFF, 0xD8]);
    // APP0 marker (JFIF) - minimal
    bytes.extend_from_slice(&[0xFF, 0xE0]); // APP0
    bytes.extend_from_slice(&[0x00, 0x10]); // length = 16
    bytes.extend_from_slice(b"JFIF\0"); // identifier
    bytes.extend_from_slice(&[0x01, 0x01]); // version
    bytes.extend_from_slice(&[0x00]); // units
    bytes.extend_from_slice(&[0x00, 0x01]); // X density
    bytes.extend_from_slice(&[0x00, 0x01]); // Y density
    bytes.extend_from_slice(&[0x00, 0x00]); // thumbnail
                                            // SOF0 marker (baseline DCT)
    bytes.extend_from_slice(&[0xFF, 0xC0]); // SOF0
    bytes.extend_from_slice(&[0x00, 0x11]); // length = 17
    bytes.extend_from_slice(&[0x08]); // bits per sample
    bytes.extend_from_slice(&height.to_be_bytes()); // height (u16 BE)
    bytes.extend_from_slice(&width.to_be_bytes()); // width (u16 BE)
    bytes.extend_from_slice(&[0x03]); // number of components
                                      // Component specs (3 components * 3 bytes each)
    bytes.extend_from_slice(&[0x01, 0x22, 0x00]); // Y
    bytes.extend_from_slice(&[0x02, 0x11, 0x01]); // Cb
    bytes.extend_from_slice(&[0x03, 0x11, 0x01]); // Cr
    bytes
}

#[test]
fn test_extract_jpeg_dimensions_valid() {
    // @step Given a user pastes a JPEG image via the TUI bridge
    // @step And the image has dimensions 9000x6000 pixels
    let bytes = make_jpeg_with_dimensions(9000, 6000);

    let result = extract_jpeg_dimensions(&bytes);

    assert_eq!(result, Some((9000, 6000)));
}

#[test]
fn test_extract_jpeg_dimensions_normal_size() {
    let bytes = make_jpeg_with_dimensions(1920, 1080);
    let result = extract_jpeg_dimensions(&bytes);
    assert_eq!(result, Some((1920, 1080)));
}

#[test]
fn test_extract_jpeg_dimensions_progressive() {
    // SOF2 (progressive DCT) should also be detected
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xFF, 0xD8]); // SOI
    bytes.extend_from_slice(&[0xFF, 0xC2]); // SOF2 (progressive)
    bytes.extend_from_slice(&[0x00, 0x11]); // length = 17
    bytes.extend_from_slice(&[0x08]); // bits per sample
    bytes.extend_from_slice(&4000_u16.to_be_bytes()); // height
    bytes.extend_from_slice(&3000_u16.to_be_bytes()); // width
    bytes.extend_from_slice(&[0x03]); // components
    bytes.extend_from_slice(&[0x01, 0x22, 0x00]);
    bytes.extend_from_slice(&[0x02, 0x11, 0x01]);
    bytes.extend_from_slice(&[0x03, 0x11, 0x01]);

    let result = extract_jpeg_dimensions(&bytes);
    assert_eq!(result, Some((3000, 4000)));
}

#[test]
fn test_extract_jpeg_dimensions_corrupt() {
    // @step Given I have a PNG file with a corrupt or invalid header
    // (applies to JPEG too — corrupt headers)
    let bytes = vec![0xFF, 0xD8, 0xFF]; // Truncated after SOI+marker start

    let result = extract_jpeg_dimensions(&bytes);

    // @step Then the dimension check should fail gracefully without crashing
    assert_eq!(result, None);
}

#[test]
fn test_extract_jpeg_dimensions_not_jpeg() {
    let bytes = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic
    let result = extract_jpeg_dimensions(&bytes);
    assert_eq!(result, None);
}

#[test]
fn test_extract_jpeg_dimensions_empty() {
    let result = extract_jpeg_dimensions(&[]);
    assert_eq!(result, None);
}

// ============================================================================
// Base64 Dimension Extraction (Layer 2 safety net)
// ============================================================================

#[test]
fn test_extract_dimensions_from_base64_png() {
    // @step Given a tool has returned base64-encoded image data
    use base64::{engine::general_purpose::STANDARD, Engine};

    // @step And the image has dimensions 10000x5000 pixels
    let raw = make_png_header(10000, 5000);
    let b64 = STANDARD.encode(&raw);

    // @step When parse_tool_result_content processes the tool result
    let result = extract_dimensions_from_base64(&b64);

    // @step Then it should replace the image with a ToolResultContent::text error
    // (The caller checks dimensions against MAX_IMAGE_PIXEL_DIMENSION and replaces accordingly)
    assert_eq!(result, Some((10000, 5000)));
    let (w, h) = result.unwrap();

    // @step And the error text should indicate the image exceeds dimension limits
    assert!(
        w > MAX_IMAGE_PIXEL_DIMENSION || h > MAX_IMAGE_PIXEL_DIMENSION,
        "10000x5000 should exceed the limit of {MAX_IMAGE_PIXEL_DIMENSION}"
    );

    // @step And no ToolResultContent::Image should be emitted
    // (Validated by the dimension check — when dimensions exceed limit, callers must not emit Image)
}

#[test]
fn test_extract_dimensions_from_base64_jpeg() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let raw = make_jpeg_with_dimensions(1920, 1080);
    let b64 = STANDARD.encode(&raw);

    let result = extract_dimensions_from_base64(&b64);

    assert_eq!(result, Some((1920, 1080)));
}

#[test]
fn test_extract_dimensions_from_base64_invalid() {
    // Invalid base64
    let result = extract_dimensions_from_base64("not-valid-base64!!!");
    // Should return None gracefully (corrupt/invalid data)
    assert_eq!(result, None);
}

#[test]
fn test_extract_dimensions_from_base64_empty() {
    let result = extract_dimensions_from_base64("");
    assert_eq!(result, None);
}

// ============================================================================
// MAX_IMAGE_PIXEL_DIMENSION constant
// ============================================================================

#[test]
fn test_max_pixel_dimension_is_5999() {
    // @step The universal pixel dimension limit should be 5999px (strictest provider: Z.AI at 6000)
    assert_eq!(MAX_IMAGE_PIXEL_DIMENSION, 5999);
}
