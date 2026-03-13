//! Read tool implementation
//!
//! Reads file contents with line numbers, supporting offset and limit.
//! Supports multimodal content: images are returned as base64-encoded data.
//!
//! Text files are checked against token limits before being returned.
//! Images, PDFs, and Jupyter notebooks are exempt from token limits (processed differently).
//!
//! For isolated sessions, file paths are validated and resolved to the worktree
//! to ensure the session cannot access files outside its isolated environment.

use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::file_type::{detect_file_type, ExemptFileType, FileType};
use super::limits::OutputLimits;
use super::pdf::{read_pdf_from_bytes, PdfError};
use super::truncation::{format_truncation_warning, truncate_line_default};
use super::validation::{require_absolute_path, require_file_exists};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use codelet_common::token_estimator::check_token_limit;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

use super::file_type::ImageMediaType;
use crate::image_dimensions::{
    exceeds_pixel_limit, extract_jpeg_dimensions, extract_png_dimensions, format_dimension_error,
};

/// Maximum base64 size (in bytes) for images sent to LLM providers.
///
/// 5MB is the strictest limit across all supported providers:
///   - Claude (Anthropic): 5MB base64 per image
///   - Z.AI (GLM-4V): 5MB per image
///   - OpenAI (GPT-4o): ~20MB base64 per image
///   - Gemini (Google): 20MB inline request
///
/// We use the strictest limit as a universal safe default.
pub const MAX_IMAGE_BASE64_BYTES: usize = 5 * 1024 * 1024; // 5MB

/// Structured output for the Read tool supporting multimodal content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReadOutput {
    /// Text content with line numbers
    Text { content: String },
    /// Image content as base64-encoded data
    Image { data: String, media_type: String },
}

/// Validate binary image data (size + pixel dimensions) and encode to [`ReadOutput::Image`].
///
/// Shared by `ReadTool` and `CodexViewImageFacade` (via `FileToolFacadeWrapper`) —
/// single source of truth for the
/// base64 size limit, pixel dimension limit, and encoding step.
pub fn validate_and_encode_image(
    binary_content: &[u8],
    media_type: ImageMediaType,
    file_path_str: &str,
    tool_name: &'static str,
) -> Result<ReadOutput, ToolError> {
    // Calculate exact base64 output size without encoding: ceil(n/3) * 4
    let raw_size = binary_content.len();
    let base64_size = raw_size.div_ceil(3) * 4;

    if base64_size > MAX_IMAGE_BASE64_BYTES {
        let actual_mb = base64_size as f64 / (1024.0 * 1024.0);
        let limit_mb = MAX_IMAGE_BASE64_BYTES as f64 / (1024.0 * 1024.0);
        return Err(ToolError::Validation {
            tool: tool_name,
            message: format!(
                "Image file is too large for LLM processing: {file_path_str}\n\
                 Base64 size: {actual_mb:.1} MB (limit: {limit_mb:.1} MB)\n\
                 Suggestions:\n\
                 - Resize the image to reduce file size (e.g., use `convert` or `sips` in Bash)\n\
                 - Use offset/limit parameters with the Read tool to view the file as text instead"
            ),
        });
    }

    // Validate pixel dimensions (PNG IHDR or JPEG SOF marker)
    let dimensions =
        extract_png_dimensions(binary_content).or_else(|| extract_jpeg_dimensions(binary_content));

    if let Some((width, height)) = dimensions {
        if exceeds_pixel_limit(width, height) {
            return Err(ToolError::Validation {
                tool: tool_name,
                message: format_dimension_error(Some(file_path_str), width, height),
            });
        }
    }
    // If dimensions can't be extracted (corrupt header, unsupported format),
    // allow the image through — don't block valid images due to parsing failure

    let base64_data = BASE64.encode(binary_content);
    Ok(ReadOutput::Image {
        data: base64_data,
        media_type: media_type.as_mime().to_string(),
    })
}

/// Read tool for reading file contents.
///
/// Requires a session ID for path resolution. In isolated sessions, paths are
/// resolved relative to the session's worktree directory.
pub struct ReadTool {
    session_id: Uuid,
}

impl ReadTool {
    /// Create a new Read tool instance.
    ///
    /// # Arguments
    /// * `session_id` - The session ID used for path resolution in isolated sessions
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }

    /// Read file as binary and return raw bytes
    async fn read_binary(path: &Path) -> Result<Vec<u8>, ToolError> {
        fs::read(path).await.map_err(|e| ToolError::File {
            tool: "read",
            message: format!("Error reading file: {e}"),
        })
    }

    /// Read file as text with line numbers (existing behavior)
    fn format_text_with_line_numbers(content: &str, offset: usize, limit: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Calculate range (offset is 1-based)
        // Bound start_idx to prevent panic when offset > total_lines
        let start_idx = offset.saturating_sub(1).min(total_lines);
        let effective_limit = limit.min(OutputLimits::MAX_LINES);
        let end_idx = (start_idx + effective_limit).min(total_lines);

        // Format lines with numbers and truncate long lines
        let mut output_lines: Vec<String> = Vec::new();
        for (idx, line) in lines[start_idx..end_idx].iter().enumerate() {
            let line_num = start_idx + idx + 1;
            let truncated_line = truncate_line_default(line);
            output_lines.push(format!("{line_num}: {truncated_line}"));
        }

        // Check if we need to truncate due to line limit
        let lines_after_range = total_lines.saturating_sub(end_idx);
        let was_truncated = end_idx < total_lines && lines_after_range > 0;

        let mut output = output_lines.join("\n");

        if was_truncated {
            let remaining = total_lines - end_idx;
            let warning =
                format_truncation_warning(remaining, "lines", true, OutputLimits::MAX_OUTPUT_CHARS);
            output.push('\n');
            output.push_str(&warning);
        }

        output
    }

    /// Process binary content as text with line numbers and token limit validation.
    ///
    /// Used by both SVG (text-based XML) and plain text file branches to avoid
    /// duplicating the token limit checking and line formatting logic.
    fn process_as_text(
        binary_content: Vec<u8>,
        file_path_str: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<ReadOutput, ToolError> {
        let text_content =
            String::from_utf8(binary_content).map_err(|e| ToolError::File {
                tool: "read",
                message: format!("Error reading file as text: {e}"),
            })?;

        let has_custom_range = offset.is_some() || limit.is_some();

        // Check token limit on full content before applying line limits.
        // This ensures large files are rejected even if they would be truncated.
        if !has_custom_range {
            if let Some((estimated_tokens, max_tokens)) = check_token_limit(&text_content) {
                return Err(ToolError::TokenLimit {
                    tool: "read",
                    file_path: file_path_str.to_string(),
                    estimated_tokens,
                    max_tokens,
                });
            }
        }

        let effective_offset = offset.unwrap_or(1);
        let effective_limit = limit.unwrap_or(OutputLimits::MAX_LINES);
        let formatted =
            Self::format_text_with_line_numbers(&text_content, effective_offset, effective_limit);

        // For partial reads, check the extracted portion
        if has_custom_range {
            if let Some((estimated_tokens, max_tokens)) = check_token_limit(&formatted) {
                return Err(ToolError::TokenLimit {
                    tool: "read",
                    file_path: file_path_str.to_string(),
                    estimated_tokens,
                    max_tokens,
                });
            }
        }

        Ok(ReadOutput::Text { content: formatted })
    }
}

// rig::tool::Tool implementation

/// Arguments for Read tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadArgs {
    /// Absolute path to the file to read
    pub file_path: String,
    /// 1-based line number to start reading from (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Number of lines to read (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// PDF reading mode: "visual" (default), "text", or "images"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_mode: Option<String>,
}

impl rig::tool::Tool for ReadTool {
    const NAME: &'static str = "Read";

    type Error = ToolError;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Read".to_string(),
            description: "Reads a file from the local filesystem. You can access any file directly by using this tool.\n\n\
                Usage:\n\
                - The file_path parameter must be an absolute path, not a relative path\n\
                - By default, it reads up to 2000 lines starting from the beginning of the file\n\
                - You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters\n\
                - Any lines longer than 2000 characters will be truncated\n\
                - Results are returned using cat -n format, with line numbers starting at 1\n\
                - Text files exceeding 25,000 tokens will return an error - use offset/limit for large files\n\
                - This tool can read images (PNG, JPG, GIF, WEBP). When reading an image file the contents are presented visually as base64-encoded data with media type.\n\
                - SVG files are always read as text (line-numbered XML source), not as image data.\n\
                - PDFs support three modes via the pdf_mode parameter:\n\
                  * 'visual' (default): Renders each page as a PNG image for full visual understanding of diagrams, charts, and layouts\n\
                  * 'text': Extracts text content page by page with page numbers (use for searchable text from text-heavy documents)\n\
                  * 'images': Extracts all embedded images from the PDF (use for catalogs, presentations with photos)\n\
                - Use visual mode for PDFs with diagrams, flowcharts, or complex layouts\n\
                - PDFs and Jupyter notebooks (.ipynb) are exempt from the token limit\n\
                - If the user provides a path to a screenshot or image, use this tool to view the file at that path.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ReadArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate and resolve path (handles worktree isolation for isolated sessions)
        let resolved_path = validate_and_resolve_path(self.session_id, &args.file_path, "read")?;
        let file_path_str = resolved_path.to_string_lossy().to_string();

        // Check file path against blocklist before any I/O
        if let Err(blocked) = check_file_path(&file_path_str) {
            return Err(ToolError::Blocked {
                tool: "read",
                message: blocked.to_string(),
            });
        }

        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&file_path_str).map_err(|e| ToolError::Validation {
            tool: "read",
            message: e.content,
        })?;

        // Check file exists (async)
        require_file_exists(path, &file_path_str)
            .await
            .map_err(|e| ToolError::Validation {
                tool: "read",
                message: e.content,
            })?;

        // Read file as binary first to detect type
        let binary_content = Self::read_binary(path).await?;

        // Detect file type by extension and magic bytes
        let file_type = detect_file_type(path, &binary_content);

        let output = match file_type {
            FileType::Image(ImageMediaType::Svg) => {
                // SVG files are text-based XML — treat as text, not binary image (EXT-014 rule [4])
                Self::process_as_text(binary_content, &file_path_str, args.offset, args.limit)?
            }
            FileType::Image(media_type) => {
                // EXT-014/EXT-016: Validate size + dimensions, then encode
                validate_and_encode_image(&binary_content, media_type, &file_path_str, "read")?
            }
            FileType::Exempt(exempt_type) => {
                // PDF and IPYNB files are exempt from token limits
                // They are processed differently (as structured documents)
                match exempt_type {
                    ExemptFileType::Pdf => {
                        // Support three PDF reading modes
                        let mode = args.pdf_mode.as_deref().unwrap_or("visual");

                        // Map errors for all modes
                        let map_pdf_error = |e: PdfError| match e {
                            PdfError::Encrypted(path) => ToolError::File {
                                tool: "read",
                                message: format!("Cannot read password-protected PDF: {path}"),
                            },
                            PdfError::LoadError(msg) => ToolError::File {
                                tool: "read",
                                message: format!("Error loading PDF: {msg}"),
                            },
                            PdfError::ExtractionError { page, message } => ToolError::File {
                                tool: "read",
                                message: format!("Error extracting text from page {page}: {message}"),
                            },
                            PdfError::RenderError { page, message } => ToolError::File {
                                tool: "read",
                                message: format!("Error rendering page {page}: {message}"),
                            },
                        };

                        match mode {
                            "text" => {
                                // TEXT MODE: Extract text page by page
                                let pdf_content = read_pdf_from_bytes(&binary_content, &file_path_str)
                                    .map_err(map_pdf_error)?;
                                ReadOutput::Text {
                                    content: pdf_content.format_display(),
                                }
                            }
                            "images" => {
                                // IMAGES MODE: Extract embedded images
                                let images = super::pdf::extract_pdf_images(&binary_content, &file_path_str)
                                    .map_err(map_pdf_error)?;
                                ReadOutput::Text {
                                    content: serde_json::to_string_pretty(&images)
                                        .unwrap_or_else(|_| "[]".to_string()),
                                }
                            }
                            _ => {
                                // VISUAL MODE (default): Render pages as images
                                let pages = super::pdf::render_pdf_pages(&binary_content, &file_path_str)
                                    .map_err(map_pdf_error)?;
                                ReadOutput::Text {
                                    content: serde_json::to_string_pretty(&pages)
                                        .unwrap_or_else(|_| "[]".to_string()),
                                }
                            }
                        }
                    }
                    ExemptFileType::Ipynb => {
                        // IPYNB files are JSON and can be read as text
                        let text_content =
                            String::from_utf8(binary_content).map_err(|e| ToolError::File {
                                tool: "read",
                                message: format!("Error reading file: {e}"),
                            })?;

                        let offset = args.offset.unwrap_or(1);
                        let limit = args.limit.unwrap_or(OutputLimits::MAX_LINES);
                        let formatted =
                            Self::format_text_with_line_numbers(&text_content, offset, limit);

                        ReadOutput::Text { content: formatted }
                    }
                }
            }
            FileType::Text => {
                // For text files, use existing line-numbered format with token limits
                Self::process_as_text(binary_content, &file_path_str, args.offset, args.limit)?
            }
        };

        // Serialize to JSON string for the tool output
        serde_json::to_string(&output).map_err(|e| ToolError::File {
            tool: "read",
            message: format!("Error serializing output: {e}"),
        })
    }
}
