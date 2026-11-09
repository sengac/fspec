//! BUG-642: magic-byte sniffing drops only a character cut at the end of
//! its UTF-8 window.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use codelet_tools::file_type::{detect_by_magic_bytes, ExemptFileType, FileType, ImageMediaType};

/// `head` padded with ASCII to `cut - 1` bytes, then "€" (3 bytes), so the
/// window of `cut` bytes ends after the first byte of "€".
fn split_at_window(head: &str, cut: usize, tail: &str) -> Vec<u8> {
    let mut text = head.to_string();
    text.push_str(&" ".repeat(cut - 1 - head.len()));
    text.push('€');
    text.push_str(tail);
    text.into_bytes()
}

#[test]
fn file_type_sniffing_ignores_a_character_cut_at_the_window_end() {
    // @step Given an SVG document whose first 256 bytes end inside a multi-byte character
    let svg = split_at_window("<svg xmlns=\"http://www.w3.org/2000/svg\">", 256, "</svg>");
    // @step And an ipynb document whose first 1024 bytes end inside a multi-byte character
    let ipynb = split_at_window("{\"cells\": [], \"note\": \"", 1024, "\"}");
    // @step When their magic bytes are sniffed
    let svg_type = detect_by_magic_bytes(&svg);
    let ipynb_type = detect_by_magic_bytes(&ipynb);
    // @step Then they are detected as SVG and ipynb
    assert_eq!(svg_type, Some(FileType::Image(ImageMediaType::Svg)));
    assert_eq!(ipynb_type, Some(FileType::Exempt(ExemptFileType::Ipynb)));
    // @step And data that starts with "<svg" but holds an invalid UTF-8 byte inside the window is not detected as SVG
    let mut invalid = b"<svg xmlns=\"x\">".to_vec();
    invalid.push(0xFF);
    invalid.extend_from_slice(b" more </svg>");
    assert_eq!(detect_by_magic_bytes(&invalid), None);
}
