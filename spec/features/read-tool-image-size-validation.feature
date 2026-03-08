@EXT-014
Feature: Oversized images from Read tool crash the agent loop — pre-validate image size before sending to LLM

  """
  Validation goes in ReadTool::call() in codelet/tools/src/read.rs, in the FileType::Image branch, after base64 encoding but before constructing ReadOutput::Image
  Use a new ToolError::ImageSizeLimit variant (or reuse ToolError::Validation) with a human-readable error message — the error appears as a text tool result, never as image data in the conversation
  Provider-specific limits (Claude: 5MB base64, OpenAI: 20MB base64, Gemini: 20MB inline, Z.AI: 5MB) documented in the code as constants; initial implementation uses the strictest (5MB) as a universal safe default
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Read tool must validate image base64 size before returning ReadOutput::Image — if the base64 data exceeds 5MB (the strictest provider limit, shared by Claude and Z.AI), return a ToolError::Validation with an informative message instead of the image data
  #   2. The error message must include: the file path, the actual base64 size, the limit, and a suggestion to resize the image or read it as text using offset/limit
  #   3. The validation must happen BEFORE returning the image data, not after — once image data enters ToolResultContent and then conversation history, it cannot be removed and will poison all subsequent API calls
  #   4. The 5MB limit applies to the raw file bytes (not the base64 string), since base64 encoding inflates size by ~33% — a 3.75MB file becomes ~5MB base64
  #   5. SVG files should be exempt from image size validation — they are text-based XML and should be handled as text, not as binary image data
  #
  # EXAMPLES:
  #   1. Small PNG (500KB) → Read tool returns ReadOutput::Image normally, no validation error
  #   2. Large JPEG (8MB raw, ~10.7MB base64) → Read tool returns ToolError::Validation with message containing file path, size '10.7 MB', limit '5.0 MB', and suggestion to resize
  #   3. Image exactly at 3.75MB raw (~5MB base64) → Read tool returns ReadOutput::Image (boundary case, within limit)
  #   4. SVG file (10MB XML text) → Read tool treats it as text and returns ReadOutput::Text, NOT ReadOutput::Image — no image size validation applied
  #   5. Agent reads oversized image, gets error, tries again with different approach (e.g., bash to resize) → agent loop does NOT break, subsequent API calls succeed
  #
  # ========================================

  Background: User Story
    As a LLM agent
    I want to read image files without crashing the agent loop
    So that I can continue operating even when encountering oversized images

  @happy-path
  Scenario: Small image within size limit is returned normally
    Given I have a PNG image file at "/tmp/small-screenshot.png" that is 500KB
    When I use the Read tool to read "/tmp/small-screenshot.png"
    Then the tool should return image data with media type "image/png"
    And the result should be a ReadOutput::Image with base64-encoded data

  @error-handling
  Scenario: Oversized image returns a validation error instead of image data
    Given I have a JPEG image file at "/tmp/huge-photo.jpg" that is 8MB raw
    When I use the Read tool to read "/tmp/huge-photo.jpg"
    Then the tool should return a validation error, not image data
    And the error message should contain the file path "/tmp/huge-photo.jpg"
    And the error message should contain the actual base64 size
    And the error message should contain the limit "5.0 MB"
    And the error message should suggest resizing the image

  @boundary
  Scenario: Image at exactly the size limit is accepted
    Given I have an image file at "/tmp/boundary.png" that is exactly 3.75MB raw
    When I use the Read tool to read "/tmp/boundary.png"
    Then the tool should return image data with media type "image/png"
    And no validation error should occur

  @svg-exemption
  Scenario: SVG files are treated as text and bypass image size validation
    Given I have an SVG file at "/tmp/large-diagram.svg" that is 10MB of XML text
    When I use the Read tool to read "/tmp/large-diagram.svg"
    Then the tool should return text content, not image content
    And the result should be a ReadOutput::Text with line-numbered content

  @recovery
  Scenario: Agent loop continues after oversized image error
    Given I have an oversized image at "/tmp/massive.png" that is 15MB raw
    When I use the Read tool to read "/tmp/massive.png"
    Then the tool should return a validation error as text
    And the error should never enter the conversation as image data
    And subsequent Read tool calls for other files should succeed normally
