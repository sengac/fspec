//! Base64 image detection and metadata extraction for the Trimmer.
//!
//! Handles detection of base64-encoded image data in content strings,
//! extraction of PNG dimensions from headers, and byte count estimation.

/// Detect if content contains base64 image data.
///
/// Checks for the data URI scheme pattern: `data:image/...;base64,...`
/// Only matches structured data URIs, not casual mentions of base64.
pub fn is_base64_image(content: &str) -> bool {
    // Only match if the data URI pattern appears at a word boundary or start of content,
    // not as a substring of prose discussion about base64.
    if let Some(pos) = content.find("data:image/") {
        // Must be preceded by nothing, whitespace, quote, or common delimiters
        if pos == 0 {
            return content.contains(";base64,");
        }
        let prev = content.as_bytes()[pos - 1];
        if prev == b'"' || prev == b'\'' || prev == b' ' || prev == b'\n' || prev == b'\t' {
            return content.contains(";base64,");
        }
    }
    false
}

/// Trim base64 image data to a compact placeholder.
pub fn trim_base64_image(content: &str, path: &str) -> String {
    let (width, height) = extract_image_dimensions(content);
    let byte_count = estimate_image_bytes(content);

    format!("[image: {width}x{height}, {byte_count} bytes, from {path}]",)
}

/// Extract image dimensions from base64-encoded PNG data.
/// Returns (width, height) or (0, 0) if unparseable.
fn extract_image_dimensions(content: &str) -> (u32, u32) {
    if let Some(b64_start) = content.find(";base64,") {
        let b64_data = &content[b64_start + 8..];
        // Take enough base64 to get the PNG header (~24 bytes = ~32 base64 chars)
        let header_b64: String = b64_data.chars().take(100).collect();

        if let Some(bytes) = decode_base64_prefix(&header_b64) {
            // PNG: bytes 16-19 = width, 20-23 = height (big-endian)
            if bytes.len() >= 24 && bytes[0..4] == [0x89, 0x50, 0x4E, 0x47] {
                let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
                let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
                return (width, height);
            }
        }
    }

    (0, 0)
}

/// Estimate the byte count of base64-encoded image data.
fn estimate_image_bytes(content: &str) -> usize {
    if let Some(b64_start) = content.find(";base64,") {
        let b64_data = &content[b64_start + 8..];
        // Base64 encodes 3 bytes into 4 chars, so byte_count ≈ len * 3/4
        let clean_len = b64_data
            .bytes()
            .filter(|&b| b != b'\n' && b != b'\r' && b != b' ' && b != b'=')
            .count();
        clean_len * 3 / 4
    } else {
        content.len()
    }
}

/// Simple base64 decoder for prefix parsing (first ~30 bytes).
///
/// Only decodes enough to read a PNG IHDR chunk. Does not require
/// external crate — intentionally minimal for this single use case.
fn decode_base64_prefix(b64: &str) -> Option<Vec<u8>> {
    let table: [i8; 128] = {
        let mut t = [-1i8; 128];
        for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            .iter()
            .enumerate()
        {
            t[c as usize] = i as i8;
        }
        t
    };

    let clean: Vec<u8> = b64
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r' && b != b' ')
        .take(96) // 96 base64 chars = 72 bytes max
        .collect();

    let mut output = Vec::new();
    for chunk in clean.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let vals: Vec<i8> = chunk
            .iter()
            .map(|&b| {
                if b == b'=' {
                    0
                } else if (b as usize) < 128 {
                    table[b as usize]
                } else {
                    -1
                }
            })
            .collect();
        if vals.iter().any(|&v| v < 0) {
            break;
        }
        output.push(((vals[0] as u8) << 2) | ((vals[1] as u8) >> 4));
        if chunk.len() > 2 && chunk[2] != b'=' {
            output.push(((vals[1] as u8) << 4) | ((vals[2] as u8) >> 2));
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            output.push(((vals[2] as u8) << 6) | (vals[3] as u8));
        }
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_base64_image_detects_data_uri() {
        let content = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==";
        assert!(is_base64_image(content));
    }

    #[test]
    fn test_is_base64_image_with_quoted_uri() {
        let content = r#"src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==""#;
        assert!(is_base64_image(content));
    }

    #[test]
    fn test_is_base64_image_rejects_prose_mention() {
        // A user discussing base64 in prose should NOT trigger image detection
        // when "data:image/" is glued to preceding text without a delimiter.
        assert!(!is_base64_image(
            "I was reading aboutdata:image/png;base64,something in the docs"
        ));
    }

    #[test]
    fn test_is_base64_image_rejects_no_base64() {
        let content = "data:image/png without base64 marker";
        assert!(!is_base64_image(content));
    }

    #[test]
    fn test_estimate_bytes_from_base64() {
        // 100 base64 chars ≈ 75 bytes
        let b64_chars = "A".repeat(100);
        let content = format!("data:image/png;base64,{b64_chars}");
        let bytes = estimate_image_bytes(&content);
        assert_eq!(bytes, 75);
    }

    #[test]
    fn test_decode_base64_prefix_valid() {
        // "AQID" = [1, 2, 3]
        let result = decode_base64_prefix("AQID");
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_decode_base64_prefix_empty() {
        assert_eq!(decode_base64_prefix(""), None);
    }

    #[test]
    fn test_dimensions_unknown_for_non_png() {
        let content = "data:image/jpeg;base64,/9j/4AAQ";
        let (w, h) = extract_image_dimensions(content);
        assert_eq!((w, h), (0, 0));
    }
}
