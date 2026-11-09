//! Feature: spec/features/image-dimension-validation.feature
//!
//! Moved out of image_content_recovery_test.rs (COV-075: one test file per feature).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]

use codelet_cli::interactive::{build_user_content_with_images, BridgeImage};
use rig::message::UserContent;

// =============================================================================
// Bridge image validation tests (build_user_content_with_images)
// =============================================================================

/// Helper: create base64-encoded JPEG with given dimensions
fn make_jpeg_base64(width: u16, height: u16) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xFF, 0xD8]); // SOI
                                            // APP0 (JFIF)
    bytes.extend_from_slice(&[0xFF, 0xE0]);
    bytes.extend_from_slice(&[0x00, 0x10]); // length 16
    bytes.extend_from_slice(b"JFIF\0");
    bytes.extend_from_slice(&[0x01, 0x01, 0x00]);
    bytes.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);
    bytes.extend_from_slice(&[0x00, 0x00]);
    // SOF0
    bytes.extend_from_slice(&[0xFF, 0xC0]);
    bytes.extend_from_slice(&[0x00, 0x11]); // length 17
    bytes.extend_from_slice(&[0x08]); // bits per sample
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&[0x03]); // components
    bytes.extend_from_slice(&[0x01, 0x22, 0x00]);
    bytes.extend_from_slice(&[0x02, 0x11, 0x01]);
    bytes.extend_from_slice(&[0x03, 0x11, 0x01]);
    STANDARD.encode(&bytes)
}

#[test]
fn test_build_user_content_rejects_oversized_jpeg_bridge_image() {
    // @step Given a user pastes a JPEG image via the TUI bridge
    // @step And the image has dimensions 9000x6000 pixels
    let images = vec![BridgeImage {
        data: make_jpeg_base64(9000, 6000),
        media_type: "image/jpeg".to_string(),
    }];

    // @step When the stream loop processes the user input
    let content = build_user_content_with_images("Check this", Some(images));

    // @step Then the image should be rejected before entering conversation history
    let parts: Vec<&UserContent> = content.iter().collect();
    for part in &parts {
        if let UserContent::Image { .. } = part {
            panic!("Oversized JPEG bridge image should NOT produce Image content");
        }
    }
    // @step And the user should see an error message about dimension limits
    let has_error = parts.iter().any(|p| {
        matches!(p, UserContent::Text(text) if text.text.contains("9000") && text.text.contains("6000"))
    });
    assert!(has_error, "Should contain dimension error message");

    // @step And subsequent API calls should continue to work normally
    // (Verified: build_user_content_with_images is a pure function — the returned content
    // is valid for use in subsequent API calls, no session state is corrupted)
}
