//! Read tool implementation
//!
//! Reads file contents with line numbers, supporting offset and limit.
//! Supports multimodal content: images are returned as base64-encoded data.
//!
//! PROV-002: Text files are checked against token limits before being returned.
//! Images, PDFs, and Jupyter notebooks are exempt from token limits (processed differently).

use super::error::ToolError;
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

/// Structured output for the Read tool supporting multimodal content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReadOutput {
    /// Text content with line numbers
    Text { content: String },
    /// Image content as base64-encoded data
    Image { data: String, media_type: String },
}

/// Read tool for reading file contents
pub struct ReadTool;

impl ReadTool {
    /// Create a new Read tool instance
    pub fn new() -> Self {
        Self
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
        // CRITICAL: Bound start_idx to prevent panic when offset > total_lines
        // This can happen when a file is shortened between sessions and the old
        // offset is cached or restored from history.
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
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
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
                - This tool can read images (PNG, JPG, GIF, WEBP, SVG). When reading an image file the contents are presented visually as base64-encoded data with media type.\n\
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
        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&args.file_path).map_err(|e| ToolError::Validation {
            tool: "read",
            message: e.content,
        })?;

        // Check file exists (async)
        require_file_exists(path, &args.file_path)
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
            FileType::Image(media_type) => {
                // For images, base64 encode and return structured output
                let base64_data = BASE64.encode(&binary_content);
                ReadOutput::Image {
                    data: base64_data,
                    media_type: media_type.as_mime().to_string(),
                }
            }
            FileType::Exempt(exempt_type) => {
                // PROV-002: PDF and IPYNB files are exempt from token limits
                // They are processed differently (as structured documents)
                match exempt_type {
                    ExemptFileType::Pdf => {
                        // TOOLS-002: Support three PDF reading modes
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
                            PdfError::PdfiumNotAvailable(msg) => ToolError::File {
                                tool: "read",
                                message: msg,
                            },
                        };

                        match mode {
                            "text" => {
                                // TEXT MODE: Extract text page by page
                                let pdf_content = read_pdf_from_bytes(&binary_content, &args.file_path)
                                    .map_err(map_pdf_error)?;
                                ReadOutput::Text {
                                    content: pdf_content.format_display(),
                                }
                            }
                            "images" => {
                                // IMAGES MODE: Extract embedded images
                                let images = super::pdf::extract_pdf_images(&binary_content, &args.file_path)
                                    .map_err(map_pdf_error)?;
                                ReadOutput::Text {
                                    content: serde_json::to_string_pretty(&images)
                                        .unwrap_or_else(|_| "[]".to_string()),
                                }
                            }
                            _ => {
                                // VISUAL MODE (default): Render pages as images
                                let pages = super::pdf::render_pdf_pages(&binary_content, &args.file_path)
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
                // For text files, use existing line-numbered format
                let text_content =
                    String::from_utf8(binary_content).map_err(|e| ToolError::File {
                        tool: "read",
                        message: format!("Error reading file: {e}"),
                    })?;

                // PROV-002: Check token limit on the raw content BEFORE applying line limits
                // This ensures large files are rejected even if they would be truncated
                // If user provides offset/limit, check only the requested portion
                let has_custom_range = args.offset.is_some() || args.limit.is_some();

                if !has_custom_range {
                    // No custom range - check full file first
                    if let Some((estimated_tokens, max_tokens)) = check_token_limit(&text_content) {
                        return Err(ToolError::TokenLimit {
                            tool: "read",
                            file_path: args.file_path.clone(),
                            estimated_tokens,
                            max_tokens,
                        });
                    }
                }

                let offset = args.offset.unwrap_or(1);
                let limit = args.limit.unwrap_or(OutputLimits::MAX_LINES);
                let formatted = Self::format_text_with_line_numbers(&text_content, offset, limit);

                // For partial reads, check the extracted portion
                if has_custom_range {
                    if let Some((estimated_tokens, max_tokens)) = check_token_limit(&formatted) {
                        return Err(ToolError::TokenLimit {
                            tool: "read",
                            file_path: args.file_path.clone(),
                            estimated_tokens,
                            max_tokens,
                        });
                    }
                }

                ReadOutput::Text { content: formatted }
            }
        };

        // Serialize to JSON string for the tool output
        serde_json::to_string(&output).map_err(|e| ToolError::File {
            tool: "read",
            message: format!("Error serializing output: {e}"),
        })
    }
}
