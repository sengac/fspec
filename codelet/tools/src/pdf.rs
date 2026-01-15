//! PDF reading module
//!
//! TOOLS-002: Provides PDF reading with three modes:
//! - TEXT: Extract text page by page using lopdf
//! - IMAGES: Extract embedded images using lopdf XObject iteration
//! - VISUAL: Render pages as PNG images at 150 DPI using pdfium-render
//!
//! Handles encrypted PDFs with clear errors.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::ImageFormat;
use lopdf::{Document, Error as LopdfError};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

/// Error types for PDF reading
#[derive(Debug)]
pub enum PdfError {
    /// PDF is password-protected/encrypted
    Encrypted(String),
    /// Error loading or parsing PDF
    LoadError(String),
    /// Error extracting text from page
    ExtractionError { page: u32, message: String },
    /// Error rendering PDF page
    RenderError { page: u32, message: String },
    /// Pdfium library not available
    PdfiumNotAvailable(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Encrypted(path) => {
                write!(f, "Cannot read password-protected PDF: {path}")
            }
            PdfError::LoadError(msg) => {
                write!(f, "Error loading PDF: {msg}")
            }
            PdfError::ExtractionError { page, message } => {
                write!(f, "Error extracting text from page {page}: {message}")
            }
            PdfError::RenderError { page, message } => {
                write!(f, "Error rendering page {page}: {message}")
            }
            PdfError::PdfiumNotAvailable(msg) => {
                write!(f, "Pdfium library not available: {msg}")
            }
        }
    }
}

impl std::error::Error for PdfError {}

/// Extracted PDF content with page-by-page text
#[derive(Debug, Clone)]
pub struct PdfContent {
    /// Path to the PDF file
    pub path: String,
    /// Total number of pages
    pub total_pages: usize,
    /// Text content from each page (0-indexed)
    pub pages: Vec<PageContent>,
}

/// Content from a single PDF page
#[derive(Debug, Clone)]
pub struct PageContent {
    /// Page number (1-indexed for display)
    pub page_number: u32,
    /// Extracted text content
    pub text: String,
}

impl PdfContent {
    /// Format the PDF content for display
    pub fn format_display(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "PDF: {} ({} pages)\n\n",
            self.path, self.total_pages
        ));

        for page in &self.pages {
            output.push_str(&format!("--- Page {} ---\n", page.page_number));
            output.push_str(&page.text);
            if !page.text.ends_with('\n') {
                output.push('\n');
            }
            output.push('\n');
        }

        output
    }
}

/// Read and extract text from a PDF file
pub fn read_pdf_from_bytes(bytes: &[u8], path: &str) -> Result<PdfContent, PdfError> {
    // Check for encryption markers in raw bytes first
    // This catches encrypted PDFs that lopdf can't parse
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    // Load PDF from bytes
    let doc = Document::load_mem(bytes).map_err(|e| {
        // Check if the error is due to encryption
        if is_encryption_error(&e) {
            PdfError::Encrypted(path.to_string())
        } else {
            PdfError::LoadError(e.to_string())
        }
    })?;

    // Check if document is encrypted
    if doc.is_encrypted() {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    // Get page count
    let page_count = doc.get_pages().len();

    // Extract text from each page
    let mut pages = Vec::with_capacity(page_count);

    for page_num in 1..=page_count as u32 {
        let text = doc
            .extract_text(&[page_num])
            .unwrap_or_else(|e| format!("[Error extracting text: {e}]"));

        pages.push(PageContent {
            page_number: page_num,
            text,
        });
    }

    Ok(PdfContent {
        path: path.to_string(),
        total_pages: page_count,
        pages,
    })
}

/// Read PDF from file path
pub fn read_pdf_from_path(path: &Path) -> Result<PdfContent, PdfError> {
    let bytes = std::fs::read(path).map_err(|e| PdfError::LoadError(e.to_string()))?;
    read_pdf_from_bytes(&bytes, &path.to_string_lossy())
}

/// Check if a lopdf error indicates encryption or an invalid/corrupted PDF
/// (which could be due to encryption that we can't parse)
fn is_encryption_error(error: &LopdfError) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("encrypt")
        || error_str.contains("password")
        || error_str.contains("protected")
        || error_str.contains("decrypt")
}

/// Check if the raw PDF bytes contain encryption markers
pub fn has_encryption_markers(bytes: &[u8]) -> bool {
    // Look for common encryption-related strings in the PDF header/trailer
    let content = String::from_utf8_lossy(bytes);
    content.contains("/Encrypt")
        || content.contains("/Standard")
        || content.contains("/Filter /Standard")
}

// ============================================
// VISUAL MODE: Render pages as images
// ============================================

/// A rendered PDF page as an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedPage {
    /// Page number (1-indexed)
    pub page_number: u32,
    /// Base64-encoded PNG image data
    pub data: String,
    /// Media type (always "image/png" for rendered pages)
    pub media_type: String,
}

/// Result of rendering PDF pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedPdfPages {
    /// Path to the PDF file
    pub path: String,
    /// Total number of pages
    pub total_pages: usize,
    /// Rendered page images
    pub pages: Vec<RenderedPage>,
}

/// DPI for rendering PDF pages (150 DPI as per TOOLS-002 spec)
const RENDER_DPI: f32 = 150.0;

/// Standard PDF DPI (72 points per inch)
const PDF_POINTS_PER_INCH: f32 = 72.0;

/// Render PDF pages as PNG images at 150 DPI
///
/// TOOLS-002: Visual mode renders each page as a PNG image at 150 DPI.
/// Uses pdfium-render for high-quality page rendering.
pub fn render_pdf_pages(bytes: &[u8], path: &str) -> Result<RenderedPdfPages, PdfError> {
    // Check for encryption markers first
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    // Try to bind to the Pdfium library
    let pdfium = Pdfium::new(
        Pdfium::bind_to_system_library()
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")))
            .map_err(|e| PdfError::PdfiumNotAvailable(format!(
                "Could not load Pdfium library: {e}. Install libpdfium for your platform."
            )))?
    );

    // Load the PDF document
    let document = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(|e| {
        let err_str = e.to_string().to_lowercase();
        if err_str.contains("password") || err_str.contains("encrypt") {
            PdfError::Encrypted(path.to_string())
        } else {
            PdfError::LoadError(e.to_string())
        }
    })?;

    let page_count = document.pages().len() as usize;
    let mut pages = Vec::with_capacity(page_count);

    // Calculate scale factor for 150 DPI rendering
    let scale = RENDER_DPI / PDF_POINTS_PER_INCH;

    // Render each page
    for (index, page) in document.pages().iter().enumerate() {
        let page_num = (index + 1) as u32;

        // Get page dimensions and calculate render size
        let width = page.width();
        let height = page.height();
        let render_width = (width.value * scale) as i32;
        let render_height = (height.value * scale) as i32;

        // Render the page to a bitmap
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(render_width)
                    .set_target_height(render_height)
                    .render_form_data(true)
                    .render_annotations(true),
            )
            .map_err(|e| PdfError::RenderError {
                page: page_num,
                message: e.to_string(),
            })?;

        // Convert to image::DynamicImage and encode as PNG
        let dynamic_image = bitmap.as_image();
        let mut png_bytes = Cursor::new(Vec::new());
        dynamic_image
            .write_to(&mut png_bytes, ImageFormat::Png)
            .map_err(|e| PdfError::RenderError {
                page: page_num,
                message: format!("Failed to encode PNG: {e}"),
            })?;

        pages.push(RenderedPage {
            page_number: page_num,
            data: BASE64.encode(png_bytes.into_inner()),
            media_type: "image/png".to_string(),
        });
    }

    Ok(RenderedPdfPages {
        path: path.to_string(),
        total_pages: page_count,
        pages,
    })
}

// ============================================
// IMAGES MODE: Extract embedded images
// ============================================

/// An extracted embedded image from a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    /// Image index (1-indexed)
    pub index: u32,
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
    /// Image width in pixels (if available)
    pub width: Option<u32>,
    /// Image height in pixels (if available)
    pub height: Option<u32>,
}

/// Result of extracting images from a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPdfImages {
    /// Path to the PDF file
    pub path: String,
    /// Total number of pages
    pub total_pages: usize,
    /// Number of images found
    pub image_count: usize,
    /// Extracted images
    pub images: Vec<ExtractedImage>,
}

/// Extract embedded images from a PDF
///
/// TOOLS-002: Images mode extracts all embedded XObject images.
/// Determines format from /Filter (DCTDecode=JPEG, FlateDecode=PNG).
pub fn extract_pdf_images(bytes: &[u8], path: &str) -> Result<ExtractedPdfImages, PdfError> {
    // Check for encryption markers first
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    // Load PDF
    let doc = Document::load_mem(bytes).map_err(|e| {
        if is_encryption_error(&e) {
            PdfError::Encrypted(path.to_string())
        } else {
            PdfError::LoadError(e.to_string())
        }
    })?;

    if doc.is_encrypted() {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    let page_count = doc.get_pages().len();
    let mut images = Vec::new();
    let mut image_index = 0u32;

    // Iterate through all objects looking for image XObjects
    for (_object_id, object) in doc.objects.iter() {
        if let Ok(stream) = object.as_stream() {
            let dict = &stream.dict;

            // Check if this is an image XObject
            let is_image = dict
                .get(b"Subtype")
                .ok()
                .and_then(|s| s.as_name().ok())
                .map(|n| n == b"Image")
                .unwrap_or(false);

            if !is_image {
                continue;
            }

            image_index += 1;

            // Get image dimensions
            let width = dict
                .get(b"Width")
                .ok()
                .and_then(|w| w.as_i64().ok())
                .map(|w| w as u32);
            let height = dict
                .get(b"Height")
                .ok()
                .and_then(|h| h.as_i64().ok())
                .map(|h| h as u32);

            // Determine media type from filter
            let filter = dict
                .get(b"Filter")
                .ok()
                .and_then(|f| f.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).to_string());

            let media_type = match filter.as_deref() {
                Some("DCTDecode") => "image/jpeg",
                Some("JPXDecode") => "image/jp2",
                Some("CCITTFaxDecode") => "image/tiff",
                _ => "application/octet-stream", // Unknown or raw image data
            };

            // Get the raw image content
            let content = stream.content.clone();

            images.push(ExtractedImage {
                index: image_index,
                data: BASE64.encode(&content),
                media_type: media_type.to_string(),
                width,
                height,
            });
        }
    }

    Ok(ExtractedPdfImages {
        path: path.to_string(),
        total_pages: page_count,
        image_count: images.len(),
        images,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_error_display() {
        let err = PdfError::Encrypted("/path/to/file.pdf".to_string());
        assert!(err.to_string().contains("password-protected"));
        assert!(err.to_string().contains("/path/to/file.pdf"));
    }

    #[test]
    fn test_page_content_format() {
        let content = PdfContent {
            path: "test.pdf".to_string(),
            total_pages: 2,
            pages: vec![
                PageContent {
                    page_number: 1,
                    text: "Page 1 content".to_string(),
                },
                PageContent {
                    page_number: 2,
                    text: "Page 2 content".to_string(),
                },
            ],
        };

        let output = content.format_display();
        assert!(output.contains("test.pdf"));
        assert!(output.contains("2 pages"));
        assert!(output.contains("Page 1"));
        assert!(output.contains("Page 1 content"));
        assert!(output.contains("Page 2"));
        assert!(output.contains("Page 2 content"));
    }
}
