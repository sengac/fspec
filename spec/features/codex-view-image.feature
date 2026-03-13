@BUG-112
Feature: Codex view_image tool

  """
  Follows the same standalone rig::tool::Tool pattern as ApplyPatchTool — struct with session_id, Args struct with JsonSchema derive, definition() and call() methods
  Reuses ReadTool's image validation logic (base64 size check, pixel dimension check) and file_type detection rather than duplicating it
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ViewImageTool must be a standalone rig::tool::Tool with NAME = "view_image" (not a facade, since no other provider has this tool)
  #   2. ViewImageTool must accept a single required 'path' parameter (String) matching the Codex CLI spec
  #   3. ViewImageTool must validate and resolve the path using validate_and_resolve_path for worktree isolation
  #   4. ViewImageTool must check the file path against the blocklist before any I/O
  #   5. ViewImageTool must only accept image files (PNG, JPEG, GIF, WEBP) — not SVG, PDF, text, or other types
  #   6. ViewImageTool must validate base64 size <= 5MB and pixel dimensions using the same limits as ReadTool
  #   7. ViewImageTool must return the same JSON format as ReadTool for images (ReadOutput::Image with data and media_type)
  #   8. ViewImageTool must be registered in CodexProvider::create_rig_agent() with .tool(ViewImageTool::new(session_id))
  #   9. ViewImageTool must be exported from codelet_tools lib.rs
  #
  # EXAMPLES:
  #   1. Model calls view_image with path to a PNG → returns base64-encoded PNG data with media_type image/png
  #   2. Model calls view_image with path to a JPEG → returns base64-encoded JPEG data with media_type image/jpeg
  #   3. Model calls view_image with path to a text file → error: not an image file
  #   4. Model calls view_image with path to an SVG file → error: SVG is text-based, not a binary image
  #   5. Model calls view_image with non-existent path → error: file not found
  #   6. Model calls view_image with oversized image (>5MB base64) → error: image too large
  #   7. Model calls view_image with blocklisted path → error: blocked
  #   8. ViewImageTool is registered in Codex agent → tool appears in agent's tool list as view_image
  #
  # ========================================

  Background: User Story
    As a Codex model
    I want to call view_image to view local image files
    So that I can view images the same way as the native Codex CLI

  @tool @codex
  Scenario: View a PNG image file
    Given a ViewImageTool instance with a valid session ID
    And a PNG image file exists at a known path
    When view_image is called with the path to the PNG file
    Then the result is a JSON object with type "image"
    And the media_type is "image/png"
    And the data field contains base64-encoded PNG data

  @tool @codex
  Scenario: View a JPEG image file
    Given a ViewImageTool instance with a valid session ID
    And a JPEG image file exists at a known path
    When view_image is called with the path to the JPEG file
    Then the result is a JSON object with type "image"
    And the media_type is "image/jpeg"
    And the data field contains base64-encoded JPEG data

  @tool @codex
  Scenario: Reject a text file as not an image
    Given a ViewImageTool instance with a valid session ID
    And a plain text file exists at a known path
    When view_image is called with the path to the text file
    Then the tool returns an error indicating the file is not a supported image

  @tool @codex
  Scenario: Reject an SVG file as not a binary image
    Given a ViewImageTool instance with a valid session ID
    And an SVG file exists at a known path
    When view_image is called with the path to the SVG file
    Then the tool returns an error indicating SVG is not a supported binary image format

  @tool @codex
  Scenario: Return error for non-existent file
    Given a ViewImageTool instance with a valid session ID
    When view_image is called with a path that does not exist
    Then the tool returns an error indicating the file was not found

  @tool @codex
  Scenario: Reject an oversized image
    Given a ViewImageTool instance with a valid session ID
    And an image file exists whose base64 encoding exceeds 5MB
    When view_image is called with the path to the oversized image
    Then the tool returns an error indicating the image is too large for LLM processing

  @tool @codex
  Scenario: Reject a blocklisted path
    Given a ViewImageTool instance with a valid session ID
    And a blocklist is initialized with a rule blocking the target path
    When view_image is called with the blocklisted path
    Then the tool returns a blocked error

  @tool @codex @integration
  Scenario: ViewImageTool is registered in Codex agent
    Given a CodexProvider with create_rig_agent configured
    When the agent is built with a session_id
    Then the agent's tool list includes a tool named "view_image"

  @tool @codex
  Scenario: Tool definition matches Codex CLI spec
    Given a ViewImageTool instance with a valid session ID
    When the tool definition is requested
    Then the tool name is "view_image"
    And the parameters schema has a required "path" property of type string
