//! File type detection module
//!
//! Detects file types by extension and magic bytes for multimodal content support.

use std::path::Path;

/// Supported image media types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageMediaType {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
}

/// File types that are exempt from token limits (PROV-002)
/// These are processed differently and don't consume context window tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExemptFileType {
    /// PDF documents - processed as document content
    Pdf,
    /// Jupyter notebooks - processed as structured notebooks
    Ipynb,
}

impl ImageMediaType {
    /// Get the MIME type string
    pub fn as_mime(&self) -> &'static str {
        match self {
            ImageMediaType::Png => "image/png",
            ImageMediaType::Jpeg => "image/jpeg",
            ImageMediaType::Gif => "image/gif",
            ImageMediaType::Webp => "image/webp",
            ImageMediaType::Svg => "image/svg+xml",
        }
    }
}

/// Detected file type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    /// Image file with detected media type
    Image(ImageMediaType),
    /// Token-limit exempt file type (PROV-002)
    Exempt(ExemptFileType),
    /// Text file (default for unknown types)
    Text,
}

/// Detect file type by extension
pub fn detect_by_extension(path: &Path) -> Option<FileType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        // Image types
        "png" => Some(FileType::Image(ImageMediaType::Png)),
        "jpg" | "jpeg" => Some(FileType::Image(ImageMediaType::Jpeg)),
        "gif" => Some(FileType::Image(ImageMediaType::Gif)),
        "webp" => Some(FileType::Image(ImageMediaType::Webp)),
        "svg" => Some(FileType::Image(ImageMediaType::Svg)),
        // Token-limit exempt types (PROV-002)
        "pdf" => Some(FileType::Exempt(ExemptFileType::Pdf)),
        "ipynb" => Some(FileType::Exempt(ExemptFileType::Ipynb)),
        _ => None,
    }
}

/// Detect file type by magic bytes (file signature)
pub fn detect_by_magic_bytes(data: &[u8]) -> Option<FileType> {
    if data.len() < 4 {
        return None;
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if data.len() >= 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some(FileType::Image(ImageMediaType::Png));
    }

    // JPEG: FF D8 FF
    if data.len() >= 3 && data[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some(FileType::Image(ImageMediaType::Jpeg));
    }

    // GIF: 47 49 46 38 (GIF8)
    if data.len() >= 4 && data[0..4] == [0x47, 0x49, 0x46, 0x38] {
        return Some(FileType::Image(ImageMediaType::Gif));
    }

    // WebP: 52 49 46 46 ... 57 45 42 50 (RIFF....WEBP)
    if data.len() >= 12
        && data[0..4] == [0x52, 0x49, 0x46, 0x46]
        && data[8..12] == [0x57, 0x45, 0x42, 0x50]
    {
        return Some(FileType::Image(ImageMediaType::Webp));
    }

    // SVG: Check for XML declaration or <svg tag
    if let Ok(text) = std::str::from_utf8(&data[..data.len().min(256)]) {
        let text = text.trim_start();
        if text.starts_with("<?xml") || text.starts_with("<svg") {
            return Some(FileType::Image(ImageMediaType::Svg));
        }
    }

    // PDF: %PDF- (25 50 44 46 2D) - PROV-002
    if data.len() >= 5 && &data[0..5] == b"%PDF-" {
        return Some(FileType::Exempt(ExemptFileType::Pdf));
    }

    // IPYNB: JSON starting with { and containing "cells" key - PROV-002
    // Note: IPYNB files are JSON, so we check for the opening brace and the characteristic key
    if let Ok(text) = std::str::from_utf8(&data[..data.len().min(1024)]) {
        let text = text.trim_start();
        if text.starts_with('{') && text.contains("\"cells\"") {
            return Some(FileType::Exempt(ExemptFileType::Ipynb));
        }
    }

    None
}

/// Detect file type by extension first, then magic bytes as fallback
pub fn detect_file_type(path: &Path, data: &[u8]) -> FileType {
    // Try extension first
    if let Some(file_type) = detect_by_extension(path) {
        return file_type;
    }

    // Fall back to magic bytes
    if let Some(file_type) = detect_by_magic_bytes(data) {
        return file_type;
    }

    // Default to text
    FileType::Text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_png_by_extension() {
        let path = PathBuf::from("/path/to/image.png");
        assert_eq!(
            detect_by_extension(&path),
            Some(FileType::Image(ImageMediaType::Png))
        );
    }

    #[test]
    fn test_detect_jpeg_by_extension() {
        let path = PathBuf::from("/path/to/photo.jpg");
        assert_eq!(
            detect_by_extension(&path),
            Some(FileType::Image(ImageMediaType::Jpeg))
        );

        let path = PathBuf::from("/path/to/photo.jpeg");
        assert_eq!(
            detect_by_extension(&path),
            Some(FileType::Image(ImageMediaType::Jpeg))
        );
    }

    #[test]
    fn test_detect_png_by_magic_bytes() {
        let png_signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(
            detect_by_magic_bytes(&png_signature),
            Some(FileType::Image(ImageMediaType::Png))
        );
    }

    #[test]
    fn test_detect_jpeg_by_magic_bytes() {
        let jpeg_signature = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(
            detect_by_magic_bytes(&jpeg_signature),
            Some(FileType::Image(ImageMediaType::Jpeg))
        );
    }

    #[test]
    fn test_no_extension_falls_back_to_magic_bytes() {
        let path = PathBuf::from("/path/to/image-no-ext");
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(
            detect_file_type(&path, &png_data),
            FileType::Image(ImageMediaType::Png)
        );
    }

    #[test]
    fn test_unknown_file_type_defaults_to_text() {
        let path = PathBuf::from("/path/to/unknown");
        let data = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_file_type(&path, &data), FileType::Text);
    }

    // PROV-002: PDF and IPYNB exemption tests

    #[test]
    fn test_detect_pdf_by_extension() {
        let path = PathBuf::from("/path/to/document.pdf");
        assert_eq!(
            detect_by_extension(&path),
            Some(FileType::Exempt(ExemptFileType::Pdf))
        );
    }

    #[test]
    fn test_detect_pdf_by_magic_bytes() {
        let pdf_signature = b"%PDF-1.4\n%more content";
        assert_eq!(
            detect_by_magic_bytes(pdf_signature),
            Some(FileType::Exempt(ExemptFileType::Pdf))
        );
    }

    #[test]
    fn test_detect_ipynb_by_extension() {
        let path = PathBuf::from("/path/to/notebook.ipynb");
        assert_eq!(
            detect_by_extension(&path),
            Some(FileType::Exempt(ExemptFileType::Ipynb))
        );
    }

    #[test]
    fn test_detect_ipynb_by_magic_bytes() {
        let ipynb_content = br#"{"cells": [], "metadata": {}, "nbformat": 4}"#;
        assert_eq!(
            detect_by_magic_bytes(ipynb_content),
            Some(FileType::Exempt(ExemptFileType::Ipynb))
        );
    }

    #[test]
    fn test_pdf_falls_back_to_magic_bytes() {
        let path = PathBuf::from("/path/to/file-without-extension");
        let pdf_data = b"%PDF-1.7\n";
        assert_eq!(
            detect_file_type(&path, pdf_data),
            FileType::Exempt(ExemptFileType::Pdf)
        );
    }

    #[test]
    fn test_ipynb_falls_back_to_magic_bytes() {
        let path = PathBuf::from("/path/to/file-without-extension");
        let ipynb_data = br#"{"cells": [{"cell_type": "code"}]}"#;
        assert_eq!(
            detect_file_type(&path, ipynb_data),
            FileType::Exempt(ExemptFileType::Ipynb)
        );
    }
}
