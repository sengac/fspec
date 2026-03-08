# EXT-016 Codebase Research: Image Pixel Dimension Validation

## Key File Locations

### Layer 1: Read Tool (has raw bytes — most informative errors)

**File:** `codelet/tools/src/read.rs`

- `ReadOutput::Image` enum variant at line 49 — the only output type for binary images
- `FileType::Image(media_type)` match arm at lines 253-279 — the single construction site for `ReadOutput::Image`
- Existing EXT-014 byte-size validation at lines 256-272 — checks `MAX_IMAGE_BASE64_BYTES` (5MB)
- **Insertion point for pixel validation:** After line 257 (base64_size calculation), before the size check at line 259
- The Read tool has the raw `binary_content: Vec<u8>` at this point, so full header parsing is possible

### Layer 2: parse_tool_result_content (has base64 — safety net for ALL tools)

**File:** `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs`

- `parse_tool_result_content()` function at lines 27-125
- **Image creation points that need dimension validation:**
  - Line 65: `ToolResultContent::image_base64(data, ...)` — PDF pages
  - Line 86: `ToolResultContent::image_base64(data, ...)` — Read tool images
  - Line 108: `ToolResultContent::image_base64(data, ...)` — nested PDF pages
- **Approach:** Before each `image_base64()` call, decode enough base64 to extract header dimensions. For PNG, 32 base64 chars = 24 raw bytes (enough for IHDR). For JPEG, need to decode ~8KB base64 to scan for SOF marker.
- This function also adds tool results to chat history at lines 562-579 via `parse_tool_result_content()` again — these must also go through the same validated path.

### Layer 3: User-Pasted Images via Bridge (has base64 from bridge)

**File:** `codelet/cli/src/interactive/stream_loop.rs`

- `BridgeImage` struct at line 186 — has `data: String` (base64) and `media_type: String`
- User image injection at lines 424-446 — iterates `bridge_images` and calls `UserContent::image_base64(img.data, media_type, None)`
- **No validation currently** — images go directly into `session.messages` as `Message::User`
- **Insertion point:** Before line 438 (`content_parts.push(UserContent::image_base64(...))`)

### Layer 4: API Error Recovery (last resort)

**File:** `codelet/cli/src/interactive/stream_loop.rs`

- Error handling at lines 1183-1248
- `is_prompt_too_long_error()` function at lines 49-65 — already handles prompt-too-long recovery
- **Current behavior for non-prompt-too-long errors:** Logs and returns `Err(anyhow::anyhow!("Agent error: {e}"))` at line 1247
- **Insertion point:** Between prompt-too-long check (line 1191) and the generic error handler (line 1214)
- Recovery model: Similar to prompt-too-long recovery — strip offending content, set flag, break from loop

## Shared Module Location

**New file:** `codelet/tools/src/image_dimensions.rs`

- Must be added to `codelet/tools/src/lib.rs` as `pub mod image_dimensions;`
- Must export:
  - `extract_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)>`
  - `extract_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)>`
  - `extract_dimensions_from_base64(base64_data: &str) -> Option<(u32, u32)>`
  - `const MAX_IMAGE_PIXEL_DIMENSION: u32 = 5999;`

## PNG Header Format (for dimension extraction)

```
Offset  Bytes  Description
0       8      PNG signature: 89 50 4E 47 0D 0A 1A 0A
8       4      IHDR chunk length (always 13 = 0x0000000D)
12      4      IHDR chunk type: "IHDR"
16      4      Width (u32 Big Endian)
20      4      Height (u32 Big Endian)
```

Total bytes needed: 24 (= 32 base64 chars)

## JPEG Header Format (for dimension extraction)

```
Offset  Bytes  Description
0       2      SOI marker: FF D8
2+      var    Variable-length APP markers (APP0=FFE0, APP1=FFE1, etc.)
...     ...    DQT, DHT, and other markers
???     2      SOF marker: FF C0 (baseline), FF C1, FF C2 (progressive)
???+2   2      Frame length
???+4   1      Bits per sample
???+5   2      Height (u16 Big Endian)
???+7   2      Width (u16 Big Endian)
```

The SOF marker can be at any offset. Must scan marker-by-marker (NOT byte-by-byte) through the JPEG header structure:
1. After SOI (FF D8), each segment starts with FF XX.
2. If XX is a SOF marker (C0-CF excluding C4=DHT, C8=JPG reserved, CC=DAC): read dimensions, done.
3. If XX is SOS (DA): stop scanning — SOF always precedes SOS per ITU T.81. Image data follows after SOS and must not be scanned.
4. If XX is a standalone marker (D0-D9 RST/SOI/EOI, 01 TEM): advance 2 bytes.
5. Otherwise: read 2-byte length at offset+2, advance 2+length bytes to next marker.

This marker-by-marker approach is safe because SOF must appear before SOS in a valid JPEG, and we never enter entropy-coded data.

**For base64 safety net:** Decode first 8KB of base64 (~6KB raw). This covers virtually all JPEG headers. If SOF not found in 6KB, let the image through (graceful fallback per Rule [5]).

**Spec references verified:**
- PNG: libpng.org/pub/png/spec/1.2/PNG-Chunks.html (IHDR section)
- JPEG: ITU T.81 / ISO 10918-1, confirmed via disktuna.com marker list and wikibooks JPEG header docs
- Claude API: platform.claude.com/docs/en/build-with-claude/vision ("larger than 8000x8000 px, it is rejected")

## GIF and WebP Dimension Extraction

**GIF:**
```
Offset  Bytes  Description
0       6      GIF signature: "GIF89a" or "GIF87a"
6       2      Width (u16 Little Endian)
8       2      Height (u16 Little Endian)
```
Total bytes needed: 10

**WebP:**
```
Offset  Bytes  Description
0       4      RIFF marker
4       4      File size
8       4      WEBP marker
12      4      Chunk type (VP8, VP8L, or VP8X)
```
VP8: Width/Height at offset 26-29 (after parsing VP8 header)
VP8L: Width 14-bit and Height 14-bit packed in bytes 21-24
VP8X: Width 24-bit at offset 24-26, Height 24-bit at offset 27-29

**For simplicity:** Focus on PNG and JPEG first (most common screenshot formats). GIF and WebP can fall through to graceful handling.

## Existing Test Patterns

- `codelet/tools/tests/read_image_size_validation_test.rs` — EXT-014 tests for byte size validation
- `codelet/tools/tests/read_multimodal_test.rs` — multimodal image read tests
- `codelet/cli/tests/prompt_too_long_recovery_test.rs` — pattern for API error recovery tests
- Helper `create_file_of_size()` creates dummy images with PNG headers

## Dependencies

- `base64` crate already in codelet/tools for encoding — can also be used for decoding in safety net
- No external image crate needed — raw header parsing only
- `codelet_tools` lib already re-exports `ReadTool` and `ReadOutput`

## Error Message Format (Rule [4])

```
Image pixel dimensions exceed limit: /path/to/screenshot.png
Dimensions: 800x15000 (limit: 5999px on any side)
Suggestions:
  macOS: sips -Z 4000 /path/to/screenshot.png
  Linux: convert -resize 4000x4000 /path/to/screenshot.png
```

## Provider Pixel Dimension Limits (verified from official docs)

| Provider | Max Pixel Dimension | Source |
|----------|-------------------|--------|
| Z.AI (GLM-4V) | 6000×6000px (strictest) | aisharenet.com GLM-4V-Flash docs |
| Claude (Anthropic) | 8000×8000px | platform.claude.com/docs/en/build-with-claude/vision |
| OpenAI (GPT-5.4) | 6000px max in "original" | developers.openai.com/api/docs/guides/images-vision |
| OpenAI (GPT-4o) | 2048px (auto-resizes) | developers.openai.com/api/docs/guides/images-vision |
| Gemini | No documented hard limit | ai.google.dev/gemini-api/docs/image-understanding |

**Universal safe limit:** MAX_IMAGE_PIXEL_DIMENSION = 5999 (just under Z.AI's 6000)

## Recovery Flow (Rules [6] & [7])

```
1. API returns 400 invalid_request_error (NOT prompt-too-long)
2. Check if error message mentions "image" or "dimensions" or "size"
3. If yes (image-related 400):
   a. Pop last user message from session.messages
   b. Walk session.messages backward, find Image content
   c. Replace Image with text placeholder:
      "[Image removed: exceeded 8000px dimension limit — 800x15000px PNG]"
   d. Re-add user message
   e. Set retry flag, break from loop to retry
4. If no (unknown 400):
   a. Show error to user
   b. Session remains Idle (don't return Err)
   c. If SAME error recurs on NEXT turn:
      - Strip non-text content from last few messages
      - Retry
```
