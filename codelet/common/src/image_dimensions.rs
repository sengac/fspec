//! Image dimension extraction from raw headers
//!
//! Extracts pixel dimensions from PNG and JPEG headers without external image crates.
//! Used by the Read tool (Layer 1), parse_tool_result_content (Layer 2), and
//! stream_loop bridge images (Layer 3) for defense-in-depth pixel dimension validation.
//!
//! Feature: spec/features/image-dimension-validation.feature

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Maximum pixel dimension allowed on any side of an image.
///
/// 5999px is just under the strictest provider limit:
///   - Z.AI (GLM-4V): 6000×6000px (strictest)
///   - OpenAI (GPT-5.4): 6000px max in "original" detail
///   - Claude (Anthropic): 8000×8000px
///   - Gemini: No documented hard limit
///
/// We use 5999 as the universal safe default across all providers.
pub const MAX_IMAGE_PIXEL_DIMENSION: u32 = 5999;

/// Extract width and height from a PNG IHDR header.
///
/// PNG format (first 24 bytes):
/// ```text
/// Offset  Bytes  Description
/// 0       8      PNG signature: 89 50 4E 47 0D 0A 1A 0A
/// 8       4      IHDR chunk length (always 13 = 0x0000000D)
/// 12      4      IHDR chunk type: "IHDR"
/// 16      4      Width (u32 Big Endian)
/// 20      4      Height (u32 Big Endian)
/// ```
///
/// Returns `None` if the header is invalid, truncated, or not PNG.
pub fn extract_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // Need at least 24 bytes for PNG signature + IHDR width + height
    if bytes.len() < 24 {
        return None;
    }

    // Verify PNG signature
    let png_sig: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes[..8] != png_sig {
        return None;
    }

    // Verify IHDR chunk type at bytes 12-15
    if &bytes[12..16] != b"IHDR" {
        return None;
    }

    // Read width (u32 BE) at bytes 16-19
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    // Read height (u32 BE) at bytes 20-23
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

    Some((width, height))
}

/// Extract width and height from a JPEG SOF marker.
///
/// JPEG format:
/// - Starts with SOI marker: FF D8
/// - Followed by variable-length segments
/// - SOF markers (FF C0-CF, excluding C4/C8/CC) contain dimensions:
///   - 2 bytes: marker (FF Cx)
///   - 2 bytes: segment length
///   - 1 byte:  bits per sample
///   - 2 bytes: height (u16 BE)
///   - 2 bytes: width (u16 BE)
///
/// Scans marker-by-marker through the header structure (not byte-by-byte).
/// Stops at SOS (FF DA) marker since SOF always precedes SOS in valid JPEG.
///
/// Returns `None` if SOF not found, header is corrupt, or not JPEG.
pub fn extract_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // Need at least 2 bytes for SOI
    if bytes.len() < 2 {
        return None;
    }

    // Verify JPEG SOI marker
    if bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut pos = 2;

    // Scan marker-by-marker
    while pos + 1 < bytes.len() {
        // Each marker starts with 0xFF
        if bytes[pos] != 0xFF {
            return None; // Invalid marker structure
        }

        let marker = bytes[pos + 1];

        // Skip padding bytes (0xFF followed by more 0xFF)
        if marker == 0xFF {
            pos += 1;
            continue;
        }

        // Check for SOF markers: C0-CF excluding C4 (DHT), C8 (JPG reserved), CC (DAC)
        let is_sof = matches!(marker, 0xC0..=0xCF)
            && marker != 0xC4
            && marker != 0xC8
            && marker != 0xCC;

        if is_sof {
            // SOF marker found — read dimensions
            // Need at least 9 bytes after marker: 2 (length) + 1 (bits) + 2 (height) + 2 (width)
            if pos + 9 >= bytes.len() {
                return None; // Truncated
            }

            let height = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
            let width = u16::from_be_bytes([bytes[pos + 7], bytes[pos + 8]]) as u32;

            return Some((width, height));
        }

        // SOS marker (FF DA) — stop scanning, SOF must appear before SOS
        if marker == 0xDA {
            return None;
        }

        // Standalone markers (D0-D9 RST/SOI/EOI, 01 TEM) — no length field
        if matches!(marker, 0xD0..=0xD9 | 0x01) {
            pos += 2;
            continue;
        }

        // All other markers have a 2-byte length field
        if pos + 3 >= bytes.len() {
            return None; // Truncated
        }

        let seg_len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        pos += 2 + seg_len; // Advance past marker + segment
    }

    None
}

/// Extract dimensions from base64-encoded image data.
///
/// Decodes enough base64 to read the image header:
/// - PNG: 44 base64 chars → ~33 raw bytes (sufficient for IHDR)
/// - JPEG: ~11000 base64 chars → ~8KB raw bytes (sufficient for most SOF markers)
///
/// Returns `None` if dimensions cannot be extracted (invalid base64, unsupported format,
/// or corrupt header). Callers should allow the image through if `None` is returned
/// (graceful fallback — don't block valid images due to parsing failure).
pub fn extract_dimensions_from_base64(base64_data: &str) -> Option<(u32, u32)> {
    if base64_data.is_empty() {
        return None;
    }

    // For PNG, we only need 24 raw bytes = ~44 base64 chars
    // For JPEG, we need up to ~8KB to find SOF marker = ~11000 base64 chars
    // Decode the minimum needed for PNG first, then try JPEG with more data
    let png_chars = base64_data.len().min(44); // 33 raw bytes covers IHDR fully
    if let Ok(raw) = BASE64.decode(&base64_data[..png_chars]) {
        if let Some(dims) = extract_png_dimensions(&raw) {
            return Some(dims);
        }
    }

    // Try JPEG — need more data for SOF marker scanning
    let jpeg_chars = base64_data.len().min(11000); // ~8KB raw
    if let Ok(raw) = BASE64.decode(&base64_data[..jpeg_chars]) {
        if let Some(dims) = extract_jpeg_dimensions(&raw) {
            return Some(dims);
        }
    }

    None
}

/// Format a human-readable error message for oversized image dimensions.
///
/// Includes file path (if known), actual dimensions, limit, and resize suggestions.
pub fn format_dimension_error(file_path: Option<&str>, width: u32, height: u32) -> String {
    let path_line = match file_path {
        Some(path) => format!("Image pixel dimensions exceed limit: {path}\n"),
        None => "Image pixel dimensions exceed limit\n".to_string(),
    };

    format!(
        "{path_line}\
         Dimensions: {width}x{height} (limit: {MAX_IMAGE_PIXEL_DIMENSION}px on any side)\n\
         Suggestions:\n  \
           macOS: sips -Z 4000 {file_ref}\n  \
           Linux: convert -resize 4000x4000 {file_ref}",
        file_ref = file_path.unwrap_or("image.png"),
    )
}

/// Check if image dimensions exceed the maximum allowed pixel dimension.
///
/// Returns `true` if either width or height exceeds `MAX_IMAGE_PIXEL_DIMENSION`.
pub fn exceeds_pixel_limit(width: u32, height: u32) -> bool {
    width > MAX_IMAGE_PIXEL_DIMENSION || height > MAX_IMAGE_PIXEL_DIMENSION
}

/// Check base64 image data for oversized dimensions and return an error message if exceeded.
///
/// Combines `extract_dimensions_from_base64` + `exceeds_pixel_limit` + `format_dimension_error`
/// into a single convenience function. Used by parse_tool_result_content (Layer 2).
///
/// Returns `Some(error_message)` if dimensions exceed the limit, `None` if OK or undetectable.
pub fn check_image_dimensions(base64_data: &str, file_path: Option<&str>) -> Option<String> {
    if let Some((w, h)) = extract_dimensions_from_base64(base64_data) {
        if exceeds_pixel_limit(w, h) {
            return Some(format_dimension_error(file_path, w, h));
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    // ============================================================================
    // Helpers
    // ============================================================================

    /// Build a valid PNG header with specified dimensions
    fn make_png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 2, 0, 0, 0]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes
    }

    /// Build a minimal JPEG with SOF0 marker
    fn make_jpeg_header(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xFF, 0xD8]); // SOI
        bytes.extend_from_slice(&[0xFF, 0xE0]); // APP0
        bytes.extend_from_slice(&[0x00, 0x10]); // length 16
        bytes.extend_from_slice(b"JFIF\0");
        bytes.extend_from_slice(&[0x01, 0x01, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes.extend_from_slice(&[0xFF, 0xC0]); // SOF0
        bytes.extend_from_slice(&[0x00, 0x11]); // length 17
        bytes.extend_from_slice(&[0x08]); // bits per sample
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[0x03]);
        bytes.extend_from_slice(&[0x01, 0x22, 0x00]);
        bytes.extend_from_slice(&[0x02, 0x11, 0x01]);
        bytes.extend_from_slice(&[0x03, 0x11, 0x01]);
        bytes
    }

    // ============================================================================
    // PNG Dimension Extraction
    // ============================================================================

    #[test]
    fn test_extract_png_dimensions_valid() {
        let bytes = make_png_header(1920, 1080);
        assert_eq!(extract_png_dimensions(&bytes), Some((1920, 1080)));
    }

    #[test]
    fn test_extract_png_dimensions_oversized() {
        let bytes = make_png_header(800, 15000);
        assert_eq!(extract_png_dimensions(&bytes), Some((800, 15000)));
    }

    #[test]
    fn test_extract_png_dimensions_at_boundary() {
        let bytes = make_png_header(5999, 5999);
        let (w, h) = extract_png_dimensions(&bytes).expect("PNG dimensions should be extractable");
        assert!(w <= MAX_IMAGE_PIXEL_DIMENSION);
        assert!(h <= MAX_IMAGE_PIXEL_DIMENSION);
    }

    #[test]
    fn test_extract_png_dimensions_exactly_6000_exceeds() {
        let bytes = make_png_header(6000, 6000);
        let (w, h) = extract_png_dimensions(&bytes).expect("PNG dimensions should be extractable");
        assert!(w > MAX_IMAGE_PIXEL_DIMENSION);
        assert!(h > MAX_IMAGE_PIXEL_DIMENSION);
    }

    #[test]
    fn test_extract_png_dimensions_corrupt() {
        let bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        assert_eq!(extract_png_dimensions(&bytes), None);
    }

    #[test]
    fn test_extract_png_dimensions_empty() {
        assert_eq!(extract_png_dimensions(&[]), None);
    }

    #[test]
    fn test_extract_png_dimensions_wrong_magic() {
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(extract_png_dimensions(&bytes), None);
    }

    // ============================================================================
    // JPEG Dimension Extraction
    // ============================================================================

    #[test]
    fn test_extract_jpeg_dimensions_valid() {
        let bytes = make_jpeg_header(9000, 6000);
        assert_eq!(extract_jpeg_dimensions(&bytes), Some((9000, 6000)));
    }

    #[test]
    fn test_extract_jpeg_dimensions_normal() {
        let bytes = make_jpeg_header(1920, 1080);
        assert_eq!(extract_jpeg_dimensions(&bytes), Some((1920, 1080)));
    }

    #[test]
    fn test_extract_jpeg_dimensions_progressive() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xFF, 0xD8]); // SOI
        bytes.extend_from_slice(&[0xFF, 0xC2]); // SOF2 (progressive)
        bytes.extend_from_slice(&[0x00, 0x11]);
        bytes.extend_from_slice(&[0x08]);
        bytes.extend_from_slice(&4000_u16.to_be_bytes());
        bytes.extend_from_slice(&3000_u16.to_be_bytes());
        bytes.extend_from_slice(&[0x03]);
        bytes.extend_from_slice(&[0x01, 0x22, 0x00]);
        bytes.extend_from_slice(&[0x02, 0x11, 0x01]);
        bytes.extend_from_slice(&[0x03, 0x11, 0x01]);
        assert_eq!(extract_jpeg_dimensions(&bytes), Some((3000, 4000)));
    }

    #[test]
    fn test_extract_jpeg_dimensions_corrupt() {
        assert_eq!(extract_jpeg_dimensions(&[0xFF, 0xD8, 0xFF]), None);
    }

    #[test]
    fn test_extract_jpeg_dimensions_not_jpeg() {
        assert_eq!(extract_jpeg_dimensions(&[0x89, 0x50, 0x4E, 0x47]), None);
    }

    #[test]
    fn test_extract_jpeg_dimensions_empty() {
        assert_eq!(extract_jpeg_dimensions(&[]), None);
    }

    // ============================================================================
    // Base64 Dimension Extraction
    // ============================================================================

    #[test]
    fn test_base64_png() {
        let raw = make_png_header(10000, 5000);
        let b64 = STANDARD.encode(&raw);
        assert_eq!(extract_dimensions_from_base64(&b64), Some((10000, 5000)));
    }

    #[test]
    fn test_base64_jpeg() {
        let raw = make_jpeg_header(1920, 1080);
        let b64 = STANDARD.encode(&raw);
        assert_eq!(extract_dimensions_from_base64(&b64), Some((1920, 1080)));
    }

    #[test]
    fn test_base64_invalid() {
        assert_eq!(extract_dimensions_from_base64("not-valid-base64!!!"), None);
    }

    #[test]
    fn test_base64_empty() {
        assert_eq!(extract_dimensions_from_base64(""), None);
    }

    // ============================================================================
    // check_image_dimensions (Layer 2 safety net logic)
    //
    // These tests validate the exact decision path used by parse_tool_result_content
    // in rig-core streaming.rs. parse_tool_result_content delegates to this function
    // to decide whether to emit ToolResultContent::Image or ToolResultContent::text.
    // ============================================================================

    /// Oversized PNG via base64 → returns error message with dimensions and limit
    /// This tests the exact decision function used by parse_tool_result_content (Layer 2).
    #[test]
    fn test_check_image_dimensions_rejects_oversized_png() {
        // @step Given a tool has returned base64-encoded image data
        // @step And the image has dimensions 10000x5000 pixels
        let raw = make_png_header(10000, 5000);
        let b64 = STANDARD.encode(&raw);

        // @step When parse_tool_result_content processes the tool result
        let result = check_image_dimensions(&b64, None);

        // @step Then it should replace the image with a ToolResultContent::text error
        assert!(result.is_some(), "Oversized image should produce error message");
        let msg = result.expect("Error message should be present for oversized image with file path");

        // @step And the error text should indicate the image exceeds dimension limits
        assert!(msg.contains("10000"), "Should contain actual width");
        assert!(msg.contains("5000"), "Should contain actual height");
        assert!(msg.contains("5999"), "Should contain the limit");
        assert!(msg.contains("sips") || msg.contains("convert"), "Should suggest resize");

        // @step And no ToolResultContent::Image should be emitted
        // (Verified: check_image_dimensions returns Some(error_msg), so the caller
        // must emit text instead of image — this is the contract tested here)
    }

    /// Normal-sized PNG via base64 → returns None (allow through)
    #[test]
    fn test_check_image_dimensions_allows_normal_png() {
        let raw = make_png_header(1920, 1080);
        let b64 = STANDARD.encode(&raw);

        let result = check_image_dimensions(&b64, None);

        assert!(result.is_none(), "Normal image should return None (allow through)");
    }

    /// Oversized PDF page render (PNG) via base64 → returns error message
    #[test]
    fn test_check_image_dimensions_rejects_oversized_pdf_page() {
        let raw = make_png_header(7000, 9000);
        let b64 = STANDARD.encode(&raw);

        let result = check_image_dimensions(&b64, None);

        assert!(result.is_some(), "Oversized PDF page should produce error");
        let msg = result.expect("Error message should be present for oversized PDF page");
        assert!(
            msg.contains("7000") || msg.contains("9000"),
            "Should contain actual dimensions"
        );
        assert!(msg.contains("5999"), "Should contain the limit");
    }

    /// Boundary case: exactly 5999×5999 → returns None (allow through)
    #[test]
    fn test_check_image_dimensions_allows_boundary() {
        let raw = make_png_header(5999, 5999);
        let b64 = STANDARD.encode(&raw);

        assert!(
            check_image_dimensions(&b64, None).is_none(),
            "5999×5999 should be allowed"
        );
    }

    /// Boundary case: exactly 6000×6000 → returns error
    #[test]
    fn test_check_image_dimensions_rejects_just_over_boundary() {
        let raw = make_png_header(6000, 6000);
        let b64 = STANDARD.encode(&raw);

        assert!(
            check_image_dimensions(&b64, None).is_some(),
            "6000×6000 should be rejected"
        );
    }

    /// Invalid base64 → returns None (graceful fallback, don't block)
    #[test]
    fn test_check_image_dimensions_graceful_on_invalid() {
        assert!(
            check_image_dimensions("totally-garbage-data!!!", None).is_none(),
            "Invalid data should return None (graceful fallback)"
        );
    }

    /// With file path → error message includes the path
    #[test]
    fn test_check_image_dimensions_includes_file_path() {
        let raw = make_png_header(10000, 5000);
        let b64 = STANDARD.encode(&raw);

        let result = check_image_dimensions(&b64, Some("/tmp/huge.png"));

        assert!(result.is_some());
        let msg = result.expect("Error message should be present for oversized image");
        assert!(msg.contains("/tmp/huge.png"), "Should contain file path");
    }

    // ============================================================================
    // Constants
    // ============================================================================

    #[test]
    fn test_max_pixel_dimension_is_5999() {
        assert_eq!(MAX_IMAGE_PIXEL_DIMENSION, 5999);
    }

    #[test]
    fn test_exceeds_pixel_limit() {
        assert!(!exceeds_pixel_limit(5999, 5999));
        assert!(exceeds_pixel_limit(6000, 5999));
        assert!(exceeds_pixel_limit(5999, 6000));
        assert!(exceeds_pixel_limit(6000, 6000));
        assert!(!exceeds_pixel_limit(1920, 1080));
        assert!(exceeds_pixel_limit(10000, 5000));
    }
}
