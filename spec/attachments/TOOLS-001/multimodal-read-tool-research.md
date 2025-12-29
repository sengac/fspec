# Multimodal Read Tool Research

## Overview

This document details the research conducted to understand how to add multimodal content support (images and PDFs) to the codelet Read tool, modeled after Claude Code's Read tool functionality.

## Claude Code Read Tool Definition (Reference)

From the Claude Code system prompt, the Read tool supports:

```
- This tool allows Claude Code to read images (eg PNG, JPG, etc). When reading an image file the contents are presented visually as Claude Code is a multimodal LLM.
- This tool can read PDF files (.pdf). PDFs are processed page by page, extracting both text and visual content for analysis.
- You will regularly be asked to read screenshots. If the user provides a path to a screenshot, ALWAYS use this tool to view the file at the path.
```

**Key behaviors:**
1. Images are "presented visually" - the LLM sees the actual image
2. PDFs are processed "page by page" with both text and visual content
3. Screenshots are a common use case

## Current Codelet Read Tool Implementation

**Location:** `codelet/tools/src/read.rs`

**Current behavior:**
- Reads files as text only
- Returns content with line numbers (cat -n format)
- Supports offset and limit for pagination
- Truncates long lines

**Key code structure:**
```rust
impl rig::tool::Tool for ReadTool {
    const NAME: &'static str = "read";
    type Error = ToolError;
    type Args = ReadArgs;
    type Output = String;  // Text only!

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Reads file as text, formats with line numbers
        let content = read_file_contents(path).await...;
        let lines: Vec<&str> = content.lines().collect();
        // Returns formatted text
    }
}
```

## Rig Framework Multimodal Support

### Message Types (rig-core/src/completion/message.rs)

Rig has comprehensive multimodal support:

**ToolResultContent enum:**
```rust
pub enum ToolResultContent {
    Text(Text),
    Image(Image),  // Supports images in tool results!
}
```

**Image struct:**
```rust
pub struct Image {
    pub data: DocumentSourceKind,
    pub media_type: Option<ImageMediaType>,
    pub detail: Option<ImageDetail>,
    pub additional_params: Option<serde_json::Value>,
}
```

**DocumentSourceKind enum:**
```rust
pub enum DocumentSourceKind {
    Url(String),      // URL reference
    Base64(String),   // Base64-encoded data
    Raw(Vec<u8>),     // Raw bytes
    String(String),   // Plain string
    Unknown,
}
```

**ImageMediaType enum:**
```rust
pub enum ImageMediaType {
    JPEG,
    PNG,
    GIF,
    WEBP,
    HEIC,
    HEIF,
    SVG,
}
```

**DocumentMediaType enum (for PDFs):**
```rust
pub enum DocumentMediaType {
    PDF,
    TXT,
    RTF,
    HTML,
    CSS,
    MARKDOWN,
    CSV,
    XML,
    Javascript,
    Python,
}
```

### Helper Methods for Tool Results

```rust
impl ToolResultContent {
    // Create text result
    pub fn text(text: impl Into<String>) -> Self

    // Create image from base64
    pub fn image_base64(
        data: impl Into<String>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self

    // Create image from raw bytes
    pub fn image_raw(
        data: impl Into<Vec<u8>>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self

    // Create image from URL
    pub fn image_url(
        url: impl Into<String>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self
}
```

### Provider Support (Anthropic)

**Location:** `rig-core/src/providers/anthropic/completion.rs:474-489`

Anthropic provider fully supports images in tool results:

```rust
message::ToolResultContent::Image(image) => {
    let DocumentSourceKind::Base64(data) = image.data else {
        return Err(MessageError::ConversionError(
            "Only base64 strings can be used with the Anthropic API"
                .to_string(),
        ));
    };
    let media_type = image.media_type.ok_or(MessageError::ConversionError(
        "Image media type is required".to_owned(),
    ))?;
    Ok(ToolResultContent::Image(ImageSource {
        data: ImageSourceData::Base64(data),
        media_type: media_type.try_into()?,
        r#type: SourceType::BASE64,
    }))
}
```

**Key requirements for Anthropic:**
- Base64 encoding required (not URLs or raw bytes)
- Media type is required

## The Challenge: Tool Trait Architecture

### Current Tool Trait

```rust
pub trait Tool: Sized + WasmCompatSend + WasmCompatSync {
    const NAME: &'static str;
    type Error: std::error::Error + WasmCompatSend + WasmCompatSync + 'static;
    type Args: for<'a> Deserialize<'a> + WasmCompatSend + WasmCompatSync;
    type Output: Serialize;  // Must be serializable

    fn call(&self, args: Self::Args) -> impl Future<Output = Result<Self::Output, Self::Error>>;
}
```

### ToolDyn Wrapper

```rust
impl<T: Tool> ToolDyn for T {
    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            match serde_json::from_str(&args) {
                Ok(args) => <Self as Tool>::call(self, args)
                    .await
                    .map_err(|e| ToolError::ToolCallError(Box::new(e)))
                    .and_then(|output| {
                        serde_json::to_string(&output).map_err(ToolError::JsonError)
                    }),  // OUTPUT IS SERIALIZED TO STRING
                Err(e) => Err(ToolError::JsonError(e)),
            }
        })
    }
}
```

### Agent Loop Handling

**Location:** `rig-core/src/agent/prompt_request/streaming.rs:343`

```rust
let tr = ToolResult {
    id: tool_call.id,
    call_id: tool_call.call_id,
    content: OneOrMany::one(ToolResultContent::Text(Text { text }))  // ALWAYS TEXT!
};
```

**The problem:** The agent loop always wraps tool outputs as `ToolResultContent::Text`, even though the type supports `Image`.

## Implementation Approaches

### Approach 1: Special Output Format (Recommended)

The Read tool returns a specially formatted JSON when reading binary files:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ReadOutput {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "image")]
    Image {
        data: String,  // base64
        media_type: String,
    },
    #[serde(rename = "pdf")]
    Pdf {
        pages: Vec<PdfPage>,  // Could be text + images per page
    },
}
```

**Agent loop modification:**
1. Parse tool output as JSON
2. If it matches `ReadOutput::Image` or `ReadOutput::Pdf`, convert to `ToolResultContent::Image`
3. Otherwise, use `ToolResultContent::Text` as before

**Pros:**
- Minimal changes to rig (only agent loop)
- Backward compatible
- Works with existing Tool trait

**Cons:**
- Requires parsing tool output as JSON
- Convention-based, not type-safe

### Approach 2: New MultimodalTool Trait

Add a new trait for tools that can return multimodal content:

```rust
pub trait MultimodalTool: Sized + WasmCompatSend + WasmCompatSync {
    const NAME: &'static str;
    type Error: std::error::Error + WasmCompatSend + WasmCompatSync + 'static;
    type Args: for<'a> Deserialize<'a> + WasmCompatSend + WasmCompatSync;

    fn call(&self, args: Self::Args)
        -> impl Future<Output = Result<OneOrMany<ToolResultContent>, Self::Error>>;
}
```

**Pros:**
- Type-safe multimodal returns
- Clean separation of concerns
- Explicit intent

**Cons:**
- Requires rig modifications
- Two different tool traits to maintain
- Migration path needed for existing tools

### Approach 3: MCP-Style Data URI Detection

MCP tools already return images as data URIs:

```rust
// From rig-core/src/tool/mod.rs:311-312
rmcp::model::RawContent::Image(raw) => {
    format!("data:{};base64,{}", raw.mime_type, raw.data)
}
```

**Agent loop modification:**
1. Check if tool output matches pattern `data:{mime_type};base64,{data}`
2. If yes, parse and convert to `ToolResultContent::Image`
3. Otherwise, use `ToolResultContent::Text`

**Pros:**
- Uses existing convention from MCP
- Simple pattern matching
- No changes to Tool trait

**Cons:**
- Fragile pattern matching
- Doesn't handle PDFs naturally
- Magic string detection

## Recommended Implementation Plan

### Phase 1: Read Tool Enhancement (codelet)

1. **Detect file type** by extension and/or magic bytes
2. **For images** (png, jpg, gif, webp, svg):
   - Read file as binary
   - Base64 encode
   - Determine media type from extension
   - Return as `ReadOutput::Image`
3. **For PDFs**:
   - Use a PDF library (e.g., `pdf-extract` or `lopdf`)
   - Extract text per page
   - Optionally render pages as images for visual content
   - Return as `ReadOutput::Pdf` with mixed content
4. **For text files** (default):
   - Keep existing behavior with line numbers

### Phase 2: Agent Loop Modification (codelet or rig)

1. **In the tool result handling** (streaming.rs equivalent):
   ```rust
   let tool_result_content = if let Ok(read_output) = serde_json::from_str::<ReadOutput>(&text) {
       match read_output {
           ReadOutput::Text { content } => ToolResultContent::text(content),
           ReadOutput::Image { data, media_type } => {
               let media_type = parse_media_type(&media_type);
               ToolResultContent::image_base64(data, Some(media_type), None)
           },
           ReadOutput::Pdf { pages } => {
               // Return as multiple content items
               OneOrMany::many(pages.into_iter().map(|p| p.to_content()).collect())
           }
       }
   } else {
       ToolResultContent::text(text)
   };
   ```

### Phase 3: PDF Support Libraries

Options for PDF processing:
- `pdf-extract`: Text extraction
- `lopdf`: Low-level PDF manipulation
- `pdf`: Another option
- Consider using an external process (poppler-utils, etc.)

For page-as-image rendering:
- `pdfium-render`: Render PDF pages to images
- `pdf2image` via subprocess

## File Type Detection

### By Extension
```rust
fn get_file_type(path: &Path) -> FileType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => FileType::Image(ImageMediaType::PNG),
        Some("jpg" | "jpeg") => FileType::Image(ImageMediaType::JPEG),
        Some("gif") => FileType::Image(ImageMediaType::GIF),
        Some("webp") => FileType::Image(ImageMediaType::WEBP),
        Some("svg") => FileType::Image(ImageMediaType::SVG),
        Some("pdf") => FileType::Pdf,
        _ => FileType::Text,
    }
}
```

### By Magic Bytes (more robust)
```rust
fn detect_by_magic(data: &[u8]) -> Option<FileType> {
    match data {
        [0x89, 0x50, 0x4E, 0x47, ..] => Some(FileType::Image(ImageMediaType::PNG)),
        [0xFF, 0xD8, 0xFF, ..] => Some(FileType::Image(ImageMediaType::JPEG)),
        [0x47, 0x49, 0x46, 0x38, ..] => Some(FileType::Image(ImageMediaType::GIF)),
        [0x52, 0x49, 0x46, 0x46, ..] if &data[8..12] == b"WEBP" =>
            Some(FileType::Image(ImageMediaType::WEBP)),
        [0x25, 0x50, 0x44, 0x46, ..] => Some(FileType::Pdf), // %PDF
        _ => None,
    }
}
```

## Testing Considerations

1. **Unit tests** for file type detection
2. **Integration tests** with actual image/PDF files
3. **Provider compatibility** tests (Anthropic image limits, etc.)
4. **Error handling** for corrupt/unreadable files
5. **Size limits** - Large images may need resizing or error handling

## Dependencies to Add

```toml
# For base64 encoding
base64 = "0.21"

# For PDF text extraction (choose one)
pdf-extract = "0.6"
# or
lopdf = "0.31"

# For PDF to image (optional, for visual PDF content)
pdfium-render = "0.8"

# For image processing/resizing (optional)
image = "0.24"
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to encode image: {0}")]
    EncodingError(String),

    #[error("Failed to process PDF: {0}")]
    PdfError(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedType(String),

    #[error("File too large: {size} bytes (max: {max})")]
    FileTooLarge { size: u64, max: u64 },
}
```

## Summary

Rig has full multimodal support at the message level (`ToolResultContent::Image`), but the current tool architecture always serializes outputs to strings and wraps them as text. The recommended approach is:

1. **Modify the Read tool** to return a structured JSON format for images/PDFs
2. **Modify the agent loop** to detect and parse this format
3. **Convert to appropriate `ToolResultContent`** variant

This approach requires minimal changes to rig while enabling full multimodal tool support.
