//! PDF reading module with three modes:
//! - TEXT: Extract text page by page using lopdf
//! - IMAGES: Extract embedded images using lopdf XObject iteration
//! - VISUAL: Render pages as PNG images using hayro (pure Rust)

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::{LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::RenderSettings;
use lopdf::{Document, Error as LopdfError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

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
    /// Truncation notice naming the next offset to continue reading (BUG-168)
    pub notice: Option<String>,
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

        if let Some(notice) = &self.notice {
            output.push('\n');
            output.push_str(notice);
        }

        output
    }
}

/// Read and extract text from a PDF file, honoring `offset` (1-based) and
/// `limit` (max pages). When the range covers fewer pages than the document
/// has, a truncation notice naming the next offset is attached (BUG-168).
pub fn read_pdf_from_bytes(
    bytes: &[u8],
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<PdfContent, PdfError> {
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

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
    let start = offset.max(1);
    let end = start
        .saturating_add(limit.saturating_sub(1))
        .min(page_count);
    let mut pages = Vec::new();

    for page_num in start..=end {
        let page_num_u32 = page_num as u32;
        let text = doc
            .extract_text(&[page_num_u32])
            .unwrap_or_else(|e| format!("[Error extracting text: {e}]"));

        pages.push(PageContent {
            page_number: page_num_u32,
            text,
        });
    }

    Ok(PdfContent {
        path: path.to_string(),
        total_pages: page_count,
        pages,
        notice: pdf_pagination_notice(start, end, page_count, "pages"),
    })
}

/// Read PDF from file path
pub fn read_pdf_from_path(path: &Path) -> Result<PdfContent, PdfError> {
    let bytes = std::fs::read(path).map_err(|e| PdfError::LoadError(e.to_string()))?;
    read_pdf_from_bytes(&bytes, &path.to_string_lossy(), 1, usize::MAX)
}

fn is_encryption_error(error: &LopdfError) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("encrypt")
        || error_str.contains("password")
        || error_str.contains("protected")
        || error_str.contains("decrypt")
}

/// Check if the raw PDF bytes contain encryption markers
pub fn has_encryption_markers(bytes: &[u8]) -> bool {
    let content = String::from_utf8_lossy(bytes);
    content.contains("/Encrypt")
        || content.contains("/Standard")
        || content.contains("/Filter /Standard")
}

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
    /// Truncation notice naming the next offset to continue rendering (BUG-168)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// Build a truncation notice for a bounded PDF read (BUG-168).
///
/// `start`/`end` are the 1-based inclusive range that was returned;
/// `total` is the full document size in `unit`s (pages or images). Returns
/// `None` when the whole document was returned, otherwise a notice that names
/// the total and — when more `unit`s remain — the exact offset to call Read
/// again with.
pub fn pdf_pagination_notice(start: usize, end: usize, total: usize, unit: &str) -> Option<String> {
    if start > total {
        return Some(format!(
            "Offset {start} is past the end of the document ({total} {unit}); no content returned."
        ));
    }
    if end >= total {
        return None;
    }
    let next = end + 1;
    Some(format!(
        "Returned {} of {total} {unit} (from offset {start}). Continue with offset={next}.",
        end - start + 1
    ))
}

/// Scale factor for 150 DPI rendering (150 / 72 standard PDF points per inch)
const RENDER_SCALE: f32 = 150.0 / 72.0;

/// Render PDF pages as PNG images at 150 DPI using hayro (pure Rust),
/// honoring `offset` (1-based) and `limit` (max pages). When the range covers
/// fewer pages than the document has, a truncation notice naming the next
/// offset is attached (BUG-168).
pub fn render_pdf_pages(
    bytes: &[u8],
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<RenderedPdfPages, PdfError> {
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

    let pdf = Pdf::new(Arc::new(bytes.to_vec())).map_err(|e| match e {
        LoadPdfError::Decryption(_) => PdfError::Encrypted(path.to_string()),
        LoadPdfError::Invalid => PdfError::LoadError("Invalid or corrupted PDF".to_string()),
    })?;

    let pdf_pages = pdf.pages();
    let page_count = pdf_pages.len();
    let start = offset.max(1);
    let end = start
        .saturating_add(limit.saturating_sub(1))
        .min(page_count);
    let mut pages = Vec::new();

    let interpreter_settings = InterpreterSettings::default();
    let render_settings = RenderSettings {
        x_scale: RENDER_SCALE,
        y_scale: RENDER_SCALE,
        width: None,
        height: None,
        bg_color: WHITE,
    };

    for (index, page) in pdf_pages.iter().enumerate() {
        let page_num = index + 1;
        if page_num < start || page_num > end {
            continue;
        }

        let page_number = page_num as u32;
        let pixmap = hayro::render(page, &interpreter_settings, &render_settings);

        let png_bytes = pixmap.into_png().map_err(|e| PdfError::RenderError {
            page: page_number,
            message: format!("Failed to encode PNG: {e:?}"),
        })?;

        pages.push(RenderedPage {
            page_number,
            data: BASE64.encode(&png_bytes),
            media_type: "image/png".to_string(),
        });
    }

    Ok(RenderedPdfPages {
        path: path.to_string(),
        total_pages: page_count,
        notice: pdf_pagination_notice(start, end, page_count, "pages"),
        pages,
    })
}

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
    /// Number of images found (across the whole document)
    pub image_count: usize,
    /// Extracted images
    pub images: Vec<ExtractedImage>,
    /// Truncation notice naming the next image offset to continue with (BUG-168)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// Extract embedded images from a PDF, honoring `offset` (1-based image
/// index) and `limit` (max images). `image_count` always reports the total
/// images in the document; when the range covers fewer, a truncation notice
/// naming the next offset is attached (BUG-168).
pub fn extract_pdf_images(
    bytes: &[u8],
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<ExtractedPdfImages, PdfError> {
    if has_encryption_markers(bytes) {
        return Err(PdfError::Encrypted(path.to_string()));
    }

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
    let start = offset.max(1);
    let end = start.saturating_add(limit.saturating_sub(1));
    let mut images = Vec::new();
    let mut image_index = 0u32;
    let mut total_images = 0usize;

    for (_object_id, object) in doc.objects.iter() {
        if let Ok(stream) = object.as_stream() {
            let dict = &stream.dict;

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
            total_images += 1;
            let index = image_index;
            if index < start as u32 || index > end as u32 {
                continue;
            }

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

            let filter = dict
                .get(b"Filter")
                .ok()
                .and_then(|f| f.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).to_string());

            let media_type = match filter.as_deref() {
                Some("DCTDecode") => "image/jpeg",
                Some("JPXDecode") => "image/jp2",
                Some("CCITTFaxDecode") => "image/tiff",
                _ => "application/octet-stream",
            };

            let content = stream.content.clone();

            images.push(ExtractedImage {
                index,
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
        image_count: total_images,
        notice: pdf_pagination_notice(start, end, total_images, "images"),
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
            notice: None,
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
