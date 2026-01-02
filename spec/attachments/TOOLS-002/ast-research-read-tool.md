# AST Research: Read Tool Implementation

## Async Functions in codelet/tools/src/read.rs

```
codelet/tools/src/read.rs:42:5:async fn read_binary(path: &Path) -> Result<Vec<u8>, ToolError>
codelet/tools/src/read.rs:113:5:async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition
codelet/tools/src/read.rs:132:5:async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>
```

## Integration Points

1. **read_binary** (line 42): Reads file as raw bytes - will be used to load PDF content
2. **call** (line 132): Main entry point where file type detection happens
   - Uses `detect_file_type()` from file_type.rs
   - Matches on `FileType::Exempt(ExemptFileType::Pdf)` - currently reads as UTF-8 (broken for binary PDFs)
   - Need to add lopdf processing here

## Required Changes

1. Add `lopdf` dependency to Cargo.toml
2. Modify the `FileType::Exempt(ExemptFileType::Pdf)` branch in `call()` to:
   - Use `lopdf::Document::load_mem()` to parse PDF bytes
   - Extract text page by page using `page_iter()` and `extract_text()`
   - Handle password-protected PDFs by catching lopdf errors
   - Return structured output with page numbers
