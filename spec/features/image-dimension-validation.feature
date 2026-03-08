@EXT-016
Feature: Oversized image pixel dimensions still crash agent loop — EXT-014 only validated byte size, not pixel dimension limit

  """
  Create a shared image_dimensions module in codelet/tools/src/ with functions: extract_png_dimensions(bytes) -> Option<(u32,u32)> and extract_jpeg_dimensions(bytes) -> Option<(u32,u32)> — reusable by both Read tool and parse_tool_result_content
  For parse_tool_result_content in rig-core patch: decode first 32 bytes of base64 (about 24 raw bytes), enough to read PNG IHDR or detect JPEG SOI marker — then scan for SOF if JPEG. This minimal decode avoids decoding the entire image.
  Three validation layers (defense-in-depth): Layer 1: Read tool (has raw bytes, most informative error). Layer 2: parse_tool_result_content (has base64, safety net for all tools). Layer 3: stream_loop user images (has base64 from bridge).
  Provider pixel limits (verified from official docs):
    Z.AI (GLM-4V): 6000×6000px — strictest hard limit (aisharenet.com GLM-4V-Flash docs)
    Claude (Anthropic): 8000×8000px (platform.claude.com/docs/en/build-with-claude/vision)
    OpenAI (GPT-5.4): 6000px max dimension in "original" detail (developers.openai.com/api/docs/guides/images-vision)
    Gemini: No documented pixel hard reject limit (auto-tiles)
  Universal safe limit: 5999px (just under the strictest provider limit of 6000px)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Images must be validated for pixel dimensions (max 5999px on any side — lowest common denominator across all providers) BEFORE being returned as ReadOutput::Image or converted to ToolResultContent::Image — once in conversation history, they cannot be removed
  #   2. parse_tool_result_content() in rig-core streaming.rs must validate image dimensions before calling ToolResultContent::image_base64() — this is the safety net that catches ALL image sources (Read tool, MCP tools, future tools), replacing oversized images with a text error
  #   3. Read tool must extract pixel dimensions from PNG headers (IHDR chunk at bytes 16-23: width/height as u32 BE) and JPEG SOF markers — no external image crate needed, just raw header parsing
  #   4. User-pasted images in stream_loop.rs (line 438, UserContent::image_base64) must also be validated for dimensions before entering conversation history
  #   5. The error message for oversized dimensions must include: file path (if known), actual dimensions (WxH), the limit (5999px), and a suggestion to resize
  #   6. The universal pixel dimension limit must be 5999px (just under 6000) — the strictest provider limit is Z.AI GLM-4V at 6000×6000px
  #
  # EXAMPLES:
  #   1. Full-page screenshot of news.com.au (800×15000px, 3MB) → Read tool extracts dimensions from PNG header, sees 15000 > 5999, returns ToolError::Validation with dimensions and resize suggestion — image NEVER enters conversation
  #   2. Normal viewport screenshot (1920×1080px, 2MB) → Read tool checks dimensions, both under 5999px, returns ReadOutput::Image normally
  #   3. MCP tool returns base64 image data with 10000px width → parse_tool_result_content() decodes first 32 bytes of base64 to read PNG header, detects oversized, replaces with ToolResultContent::text() error — conversation not poisoned
  #   4. User pastes a 9000×6000px JPEG screenshot in the TUI → stream_loop.rs validates dimensions from JPEG SOF marker before calling UserContent::image_base64(), rejects with error message
  #   5. Image exactly 5999×5999px → passes dimension check, proceeds to normal base64 size check (boundary case)
  #   6. Corrupt PNG with invalid header → dimension extraction fails gracefully, falls back to allowing the image (don't block valid images just because header parsing failed)
  #   7. API returns 400 for unknown reason → error is shown to the LLM, session returns to idle, user can send next message — no automatic retry
  #   8. API returns 400 with 'image dimensions exceed max allowed size' → stream_loop detects it's a content-related 400, sanitizes history, emits error, session returns to idle
  #
  # SPEC REFERENCES:
  #   PNG: libpng.org/pub/png/spec/1.2/PNG-Chunks.html (IHDR section)
  #   JPEG: ITU T.81 / ISO 10918-1, confirmed via disktuna.com marker list and wikibooks JPEG header docs
  #   Claude API: platform.claude.com/docs/en/build-with-claude/vision
  #   Z.AI: aisharenet.com GLM-4V-Flash docs ("not more than 6000*6000")
  #   OpenAI: developers.openai.com/api/docs/guides/images-vision
  #
  # ========================================

  Background: User Story
    As a AI agent user
    I want to have oversized images rejected before they enter conversation history
    So that my session never gets irrecoverably broken by a single bad image

  @read-tool @png
  Scenario: Read tool rejects PNG image exceeding pixel dimension limit
    Given I have a PNG image file at "/tmp/full-page-screenshot.png"
    And the image has dimensions 800x15000 pixels
    And the image is 3MB in file size
    When the Read tool reads the image file
    Then the tool should return a validation error instead of image data
    And the error message should contain the actual dimensions "800x15000"
    And the error message should contain the pixel limit "5999"
    And the error message should suggest resizing the image
    And no image data should enter the conversation history

  @read-tool @happy-path
  Scenario: Read tool accepts image within pixel dimension limit
    Given I have a PNG image file at "/tmp/viewport-screenshot.png"
    And the image has dimensions 1920x1080 pixels
    And the image is 2MB in file size
    When the Read tool reads the image file
    Then the tool should return ReadOutput::Image with base64-encoded data
    And the image media type should be "image/png"

  @safety-net @parse-tool-result
  Scenario: parse_tool_result_content rejects oversized image from any tool
    Given a tool has returned base64-encoded image data
    And the image has dimensions 10000x5000 pixels
    When parse_tool_result_content processes the tool result
    Then it should replace the image with a ToolResultContent::text error
    And the error text should indicate the image exceeds dimension limits
    And no ToolResultContent::Image should be emitted

  @user-input @jpeg
  Scenario: User-pasted JPEG image exceeding dimensions is rejected
    Given a user pastes a JPEG image via the TUI bridge
    And the image has dimensions 9000x6000 pixels
    When the stream loop processes the user input
    Then the image should be rejected before entering conversation history
    And the user should see an error message about dimension limits
    And subsequent API calls should continue to work normally

  @boundary
  Scenario: Image exactly at pixel dimension limit is accepted
    Given I have a PNG image file with dimensions 5999x5999 pixels
    When the Read tool reads the image file
    Then the tool should return ReadOutput::Image with base64-encoded data
    And the image should pass the pixel dimension check

  @error-handling
  Scenario: Corrupt image with invalid header is handled gracefully
    Given I have a PNG file with a corrupt or invalid header
    And the image dimensions cannot be extracted from the header
    When the Read tool reads the image file
    Then the tool should allow the image through without blocking
    And the dimension check should fail gracefully without crashing
