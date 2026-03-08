# AST Research: Read Tool Image Handling

## ReadOutput::Image construction sites

Only one location constructs `ReadOutput::Image`:
- `codelet/tools/src/read.rs:187` — in the `FileType::Image(media_type)` match arm

This is the exact insertion point for size validation: after base64 encoding, before returning the Image variant.

## ImageMediaType::Svg references

Three locations reference `ImageMediaType::Svg`:
- `codelet/tools/src/file_type.rs:35` — `as_mime()` returns "image/svg+xml"
- `codelet/tools/src/file_type.rs:60` — `detect_by_extension()` maps ".svg" to Image(Svg)
- `codelet/tools/src/file_type.rs:101` — `detect_by_magic_bytes()` detects XML/SVG patterns

SVG files are currently classified as `FileType::Image(ImageMediaType::Svg)`, meaning they flow through the same base64 encoding path as binary images. For EXT-014, SVGs must be rerouted to the text path instead.

## ToolError variants available

- `ToolError::Validation { tool, message }` — suitable for image size limit errors
- `ToolError::TokenLimit { tool, file_path, estimated_tokens, max_tokens }` — existing pattern for size-based rejections

## Key finding

The `call()` method in read.rs at line 183 has a `match file_type` block. The `FileType::Image(media_type)` arm (line 184-191) is where:
1. SVG exemption check needs to be added (redirect to text path)
2. Base64 size validation needs to be inserted after encoding
