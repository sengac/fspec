//! Binary-output guard for the Bash tool (BUG-142).
//!
//! Feature: spec/features/bash-tool-binary-output-guard.feature
//!
//! When a bash command prints binary content to stdout (e.g. `cat /tmp/icon.png`),
//! forwarding those raw bytes to the model is useless at best and actively
//! corrupts the context at worst. This module provides a pure detector that
//! inspects the final buffered stdout and decides whether it must be suppressed
//! in favour of a structured error that instructs the agent to use the Read
//! tool instead.
//!
//! Design:
//! - Detection is content-based, not command-based — any command that emits
//!   PNG / JPEG / GIF / WebP / PDF / ELF / ZIP / gzip magic bytes, or any
//!   embedded NUL byte, is treated as binary output.
//! - Known-type detection (image, PDF) takes priority over generic binary so
//!   the error message can name the format and tell the agent which Read mode
//!   to use.
//! - UTF-8 text with emoji, accented characters or CJK is NOT misclassified —
//!   we only look for NUL bytes or explicit magic signatures.

use crate::file_type::ImageMediaType;

/// Classification of detected binary content on stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryKind {
    /// A recognised image format (PNG/JPEG/GIF/WebP).
    Image(ImageMediaType),
    /// A PDF document.
    Pdf,
    /// Any other binary content — ELF, ZIP, gzip, raw NUL-bearing bytes, etc.
    Other,
}

/// Maximum number of leading bytes scanned for NUL presence. We don't need to
/// scan the entire buffer — binary output typically contains NULs within the
/// first few KB, and scanning more wastes cycles on truncated strings.
const NUL_SCAN_LIMIT: usize = 8192;

/// Detect whether `stdout_bytes` is binary content that must be suppressed.
///
/// Returns `None` when the output appears to be text and should be forwarded
/// to the model unchanged; returns `Some(kind)` when it should be replaced
/// with a structured binary-guard error.
///
/// Priority rules:
/// 1. Specific magic signatures (PNG/JPEG/GIF/WebP/PDF) win — we can name the
///    format and tell the agent which Read mode to use.
/// 2. Generic binary magics (ELF / ZIP / gzip) → `Other`.
/// 3. Embedded NUL byte anywhere in the first `NUL_SCAN_LIMIT` bytes → `Other`.
/// 4. Otherwise → `None` (treat as text).
pub fn detect_bash_binary_output(stdout_bytes: &[u8]) -> Option<BinaryKind> {
    if stdout_bytes.is_empty() {
        return None;
    }

    // 1. Specific image/PDF magic signatures — named types first.
    if stdout_bytes.len() >= 8
        && stdout_bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    {
        return Some(BinaryKind::Image(ImageMediaType::Png));
    }
    if stdout_bytes.len() >= 3 && stdout_bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some(BinaryKind::Image(ImageMediaType::Jpeg));
    }
    if stdout_bytes.len() >= 4 && stdout_bytes[0..4] == [0x47, 0x49, 0x46, 0x38] {
        return Some(BinaryKind::Image(ImageMediaType::Gif));
    }
    if stdout_bytes.len() >= 12
        && stdout_bytes[0..4] == [0x52, 0x49, 0x46, 0x46]
        && stdout_bytes[8..12] == [0x57, 0x45, 0x42, 0x50]
    {
        return Some(BinaryKind::Image(ImageMediaType::Webp));
    }
    if stdout_bytes.len() >= 5 && &stdout_bytes[0..5] == b"%PDF-" {
        return Some(BinaryKind::Pdf);
    }

    // 2. Mixed text-prefix + known binary payload. A command like
    //    `{ printf 'header\n'; cat /tmp/icon.png; }` emits text bytes followed
    //    by a PNG header. Scan a bounded window for embedded magic signatures.
    if let Some(kind) = scan_for_embedded_magic(stdout_bytes) {
        return Some(kind);
    }

    // 3. Generic binary magics — unnamed types.
    if stdout_bytes.len() >= 2 && stdout_bytes[0..2] == [0x1F, 0x8B] {
        return Some(BinaryKind::Other); // gzip
    }
    if stdout_bytes.len() >= 4 && stdout_bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        return Some(BinaryKind::Other); // zip
    }
    if stdout_bytes.len() >= 4 && stdout_bytes[0..4] == [0x7F, 0x45, 0x4C, 0x46] {
        return Some(BinaryKind::Other); // ELF
    }

    // 4. Generic NUL-byte presence anywhere in the first NUL_SCAN_LIMIT bytes.
    let scan_end = stdout_bytes.len().min(NUL_SCAN_LIMIT);
    if stdout_bytes[..scan_end].contains(&0x00) {
        return Some(BinaryKind::Other);
    }

    None
}

/// Look for a named magic signature embedded in the first few KB of output.
///
/// This handles `{ printf 'header\n'; cat /tmp/icon.png; }` — a text prefix
/// followed by a PNG payload. We only scan for named image/PDF magics because
/// those are the ones worth naming in the error message; unnamed binaries
/// will be caught by the subsequent NUL-byte sweep.
fn scan_for_embedded_magic(bytes: &[u8]) -> Option<BinaryKind> {
    let scan_end = bytes.len().min(NUL_SCAN_LIMIT);
    let haystack = &bytes[..scan_end];

    // PNG
    let png = [0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if find_subsequence(haystack, &png).is_some() {
        return Some(BinaryKind::Image(ImageMediaType::Png));
    }
    // JPEG
    let jpeg = [0xFFu8, 0xD8, 0xFF];
    if find_subsequence(haystack, &jpeg).is_some() {
        return Some(BinaryKind::Image(ImageMediaType::Jpeg));
    }
    // PDF
    if find_subsequence(haystack, b"%PDF-").is_some() {
        return Some(BinaryKind::Pdf);
    }
    None
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Format the deterministic, testable error message shown to the model when
/// binary output is suppressed.
pub fn format_binary_guard_message(kind: BinaryKind) -> String {
    let detected = format_detected(kind);
    format!(
        "Bash output suppressed: {detected}. Use the Read tool on the file instead; \
         the Bash tool does not return binary bytes to the model."
    )
}

/// Return just the "detected X" fragment used inside longer guard messages.
///
/// Shared between the Bash guard and the file-reading surface guards (Edit,
/// apply_patch) so every guard message names the format identically.
pub fn format_detected(kind: BinaryKind) -> String {
    match kind {
        BinaryKind::Image(ImageMediaType::Png) => "detected PNG image".to_string(),
        BinaryKind::Image(ImageMediaType::Jpeg) => "detected JPEG image".to_string(),
        BinaryKind::Image(ImageMediaType::Gif) => "detected GIF image".to_string(),
        BinaryKind::Image(ImageMediaType::Webp) => "detected WebP image".to_string(),
        BinaryKind::Image(ImageMediaType::Svg) => "detected SVG image".to_string(),
        BinaryKind::Pdf => "detected PDF document".to_string(),
        BinaryKind::Other => "detected binary content".to_string(),
    }
}

/// Format a binary-guard error message for a file-reading tool (Edit, apply_patch).
///
/// The resulting message names the detected format and directs the agent to
/// use the Read tool instead of retrying the same text-oriented operation.
pub fn format_file_tool_guard_message(tool_display: &str, kind: BinaryKind) -> String {
    let detected = format_detected(kind);
    format!(
        "{tool_display} target is a binary file ({detected}). \
         Use the Read tool to view binary/image content; \
         {tool_display} only accepts UTF-8 text files."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stdout_is_not_binary() {
        assert_eq!(detect_bash_binary_output(b""), None);
    }

    #[test]
    fn plain_ascii_text_is_not_binary() {
        assert_eq!(detect_bash_binary_output(b"hello world\n"), None);
    }

    #[test]
    fn utf8_text_with_emoji_is_not_binary() {
        let s = "hello 👋 world — café résumé\n".as_bytes();
        assert_eq!(detect_bash_binary_output(s), None);
    }

    #[test]
    fn utf8_text_with_cjk_is_not_binary() {
        let s = "中文测试 日本語 한국어\n".as_bytes();
        assert_eq!(detect_bash_binary_output(s), None);
    }

    #[test]
    fn hexdump_text_is_not_binary() {
        // `hexdump -C` output is plain ASCII; the source file being binary is irrelevant.
        let hexdump = b"00000000  89 50 4e 47 0d 0a 1a 0a  |.PNG....|\n\
                        00000010  00 00 00 0d 49 48 44 52  |....IHDR|\n";
        assert_eq!(detect_bash_binary_output(hexdump), None);
    }

    #[test]
    fn png_magic_bytes_detected_as_png() {
        let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52]);
        assert_eq!(
            detect_bash_binary_output(&data),
            Some(BinaryKind::Image(ImageMediaType::Png))
        );
    }

    #[test]
    fn jpeg_magic_bytes_detected_as_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(
            detect_bash_binary_output(&data),
            Some(BinaryKind::Image(ImageMediaType::Jpeg))
        );
    }

    #[test]
    fn gif_magic_bytes_detected_as_gif() {
        let data = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00];
        assert_eq!(
            detect_bash_binary_output(&data),
            Some(BinaryKind::Image(ImageMediaType::Gif))
        );
    }

    #[test]
    fn webp_magic_bytes_detected_as_webp() {
        let mut data = vec![0x52, 0x49, 0x46, 0x46]; // RIFF
        data.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]); // size
        data.extend_from_slice(b"WEBP");
        assert_eq!(
            detect_bash_binary_output(&data),
            Some(BinaryKind::Image(ImageMediaType::Webp))
        );
    }

    #[test]
    fn pdf_magic_bytes_detected_as_pdf() {
        let data = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        assert_eq!(detect_bash_binary_output(data), Some(BinaryKind::Pdf));
    }

    #[test]
    fn elf_magic_bytes_detected_as_other() {
        let data = [0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00];
        assert_eq!(detect_bash_binary_output(&data), Some(BinaryKind::Other));
    }

    #[test]
    fn gzip_magic_bytes_detected_as_other() {
        let data = [0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_bash_binary_output(&data), Some(BinaryKind::Other));
    }

    #[test]
    fn zip_magic_bytes_detected_as_other() {
        let data = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];
        assert_eq!(detect_bash_binary_output(&data), Some(BinaryKind::Other));
    }

    #[test]
    fn raw_nul_bytes_detected_as_other() {
        let data = b"\x00\x01\x02\x03hello";
        assert_eq!(detect_bash_binary_output(data), Some(BinaryKind::Other));
    }

    #[test]
    fn nul_byte_anywhere_in_scan_window_is_detected() {
        let mut data = vec![b'a'; 100];
        data.push(0x00);
        data.extend_from_slice(b"more text");
        assert_eq!(detect_bash_binary_output(&data), Some(BinaryKind::Other));
    }

    #[test]
    fn text_prefix_followed_by_png_is_detected_as_png() {
        let mut data = b"header\n".to_vec();
        data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        assert_eq!(
            detect_bash_binary_output(&data),
            Some(BinaryKind::Image(ImageMediaType::Png))
        );
    }

    #[test]
    fn text_prefix_followed_by_pdf_is_detected_as_pdf() {
        let mut data = b"some text\n".to_vec();
        data.extend_from_slice(b"%PDF-1.7\n");
        assert_eq!(detect_bash_binary_output(&data), Some(BinaryKind::Pdf));
    }

    #[test]
    fn format_message_names_png() {
        let msg = format_binary_guard_message(BinaryKind::Image(ImageMediaType::Png));
        assert!(msg.contains("detected PNG image"), "msg = {msg}");
        assert!(msg.contains("Use the Read tool"), "msg = {msg}");
    }

    #[test]
    fn format_message_names_pdf() {
        let msg = format_binary_guard_message(BinaryKind::Pdf);
        assert!(msg.contains("detected PDF document"), "msg = {msg}");
        assert!(msg.contains("Use the Read tool"), "msg = {msg}");
    }

    #[test]
    fn format_message_for_other_is_generic() {
        let msg = format_binary_guard_message(BinaryKind::Other);
        assert!(msg.contains("detected binary content"), "msg = {msg}");
        assert!(msg.contains("Use the Read tool"), "msg = {msg}");
        assert!(!msg.contains("PNG"), "msg = {msg}");
        assert!(!msg.contains("PDF"), "msg = {msg}");
    }
}
