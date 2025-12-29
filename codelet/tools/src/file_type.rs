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
    /// Text file (default for unknown types)
    Text,
}

/// Detect file type by extension
pub fn detect_by_extension(path: &Path) -> Option<FileType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "png" => Some(FileType::Image(ImageMediaType::Png)),
        "jpg" | "jpeg" => Some(FileType::Image(ImageMediaType::Jpeg)),
        "gif" => Some(FileType::Image(ImageMediaType::Gif)),
        "webp" => Some(FileType::Image(ImageMediaType::Webp)),
        "svg" => Some(FileType::Image(ImageMediaType::Svg)),
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
    if data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46] && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
        return Some(FileType::Image(ImageMediaType::Webp));
    }

    // SVG: Check for XML declaration or <svg tag
    if let Ok(text) = std::str::from_utf8(&data[..data.len().min(256)]) {
        let text = text.trim_start();
        if text.starts_with("<?xml") || text.starts_with("<svg") {
            return Some(FileType::Image(ImageMediaType::Svg));
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
        assert_eq!(detect_by_extension(&path), Some(FileType::Image(ImageMediaType::Png)));
    }

    #[test]
    fn test_detect_jpeg_by_extension() {
        let path = PathBuf::from("/path/to/photo.jpg");
        assert_eq!(detect_by_extension(&path), Some(FileType::Image(ImageMediaType::Jpeg)));

        let path = PathBuf::from("/path/to/photo.jpeg");
        assert_eq!(detect_by_extension(&path), Some(FileType::Image(ImageMediaType::Jpeg)));
    }

    #[test]
    fn test_detect_png_by_magic_bytes() {
        let png_signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(detect_by_magic_bytes(&png_signature), Some(FileType::Image(ImageMediaType::Png)));
    }

    #[test]
    fn test_detect_jpeg_by_magic_bytes() {
        let jpeg_signature = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_by_magic_bytes(&jpeg_signature), Some(FileType::Image(ImageMediaType::Jpeg)));
    }

    #[test]
    fn test_no_extension_falls_back_to_magic_bytes() {
        let path = PathBuf::from("/path/to/image-no-ext");
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(detect_file_type(&path, &png_data), FileType::Image(ImageMediaType::Png));
    }

    #[test]
    fn test_unknown_file_type_defaults_to_text() {
        let path = PathBuf::from("/path/to/unknown");
        let data = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_file_type(&path, &data), FileType::Text);
    }
}
