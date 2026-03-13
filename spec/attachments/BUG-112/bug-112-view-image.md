# BUG-112: Codex agent missing view_image tool

## Problem

The Codex CLI native tool set includes a `view_image` tool for viewing local image files. This tool is missing from the Codex facade and agent registration.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "view_image"
params:
  - path: String (required) - "Local filesystem path to an image file"
```

This is a `ToolSpec::Function` type tool.

## Current State

No `view_image` tool exists in:
- `codelet/tools/src/facade/codex.rs`
- `codelet/providers/src/codex/mod.rs` tool registration

The existing `ReadTool` already handles image files (PNG, JPG, GIF, WEBP) — it detects image extensions and returns base64-encoded image data with media type. This logic lives in `codelet/tools/src/read.rs`.

## Impact

When the model attempts to call `view_image` (a tool it was trained on), it gets an unknown tool error and may fall back to trying to read the image through other means or shell commands.

## Recommended Fix

Create a `ViewImageTool` as a standalone `rig::tool::Tool` that:

1. Takes a single `path` parameter
2. Validates and resolves the path (using `validate_and_resolve_path` for worktree isolation)
3. Delegates to the same image reading logic used by `ReadTool` (detecting file type, reading as base64, validating dimensions)
4. Returns the image data in the same format as `ReadTool` does for images

Alternatively, create a thin `CodexViewImageFacade` that maps `view_image { path }` to `InternalFileParams::Read { file_path: path }` and reuses the `FileToolFacadeWrapper`.

Register it in `CodexProvider::create_rig_agent()`.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- ReadTool image handling: `codelet/tools/src/read.rs`
- Image dimension validation: `codelet/tools/src/image_dimensions.rs`
