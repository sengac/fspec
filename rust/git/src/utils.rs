//! Shared utility functions for git operations

/// Check if content is binary (contains null bytes in first 8000 bytes)
///
/// This uses the same heuristic as git: if content contains a null byte
/// in the first 8000 bytes, it's considered binary.
///
/// # Arguments
/// * `content` - The byte content to check
///
/// # Returns
/// `true` if the content appears to be binary, `false` otherwise
pub fn is_binary_content(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }
    let check_length = content.len().min(8000);
    content[..check_length].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content_is_not_binary() {
        assert!(!is_binary_content(&[]));
    }

    #[test]
    fn test_text_content_is_not_binary() {
        assert!(!is_binary_content(b"Hello, world!"));
        assert!(!is_binary_content(b"fn main() { println!(\"test\"); }"));
    }

    #[test]
    fn test_content_with_null_byte_is_binary() {
        assert!(is_binary_content(&[0x89, 0x50, 0x4E, 0x47, 0x00]));
        assert!(is_binary_content(b"text\x00more"));
    }

    #[test]
    fn test_binary_image_data_is_binary() {
        // Binary data with null bytes (typical in image files after header)
        let binary_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ];
        assert!(is_binary_content(&binary_data));
    }

    #[test]
    fn test_null_byte_after_8000_bytes_is_not_detected() {
        let mut content = vec![b'a'; 8001];
        content[8000] = 0;
        assert!(!is_binary_content(&content));
    }

    #[test]
    fn test_null_byte_at_8000_boundary_is_detected() {
        let mut content = vec![b'a'; 8000];
        content[7999] = 0;
        assert!(is_binary_content(&content));
    }
}
