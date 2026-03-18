@BUG-112
Feature: Codex view_image tool
  """
  Uses the FileToolFacade pattern — CodexViewImageFacade maps Codex-native view_image params
  to InternalFileParams::Read, which delegates to ReadTool's existing image validation logic
  (base64 size check, pixel dimension check, file type detection).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexViewImageFacade must implement FileToolFacade with tool_name "view_image"
  #   2. CodexViewImageFacade must accept a required 'path' parameter (String) matching the Codex CLI spec
  #   3. CodexViewImageFacade must map view_image params to InternalFileParams::Read for ReadTool delegation
  #   4. ReadTool validates and resolves the path using validate_and_resolve_path for worktree isolation
  #   5. ReadTool checks the file path against the blocklist before any I/O
  #   6. ReadTool only accepts image files (PNG, JPEG, GIF, WEBP) — not SVG, PDF, text, or other types
  #   7. ReadTool validates base64 size <= 5MB and pixel dimensions
  #   8. The facade must be registered in CodexProvider::create_rig_agent() via FileToolFacadeWrapper
  #   9. CodexViewImageFacade must be exported from codelet_tools facade module
  #
  # EXAMPLES:
  #   1. Model calls view_image with path to a PNG → facade maps to Read, returns base64-encoded PNG data
  #   2. Model calls view_image with path to a JPEG → facade maps to Read, returns base64-encoded JPEG data
  #   3. Model calls view_image with path to a text file → error: not an image file
  #   4. Model calls view_image with path to an SVG file → error: SVG is text-based, not a binary image
  #   5. Model calls view_image with non-existent path → error: file not found
  #   6. Model calls view_image with oversized image (>5MB base64) → error: image too large
  #   7. Model calls view_image with blocklisted path → error: blocked
  #   8. Facade registered in Codex agent → tool appears in agent's tool list as view_image
  #
  # ========================================
  Background: User Story
    As a Codex model
    I want to call view_image to view local image files
    So that I can view images the same way as the native Codex CLI

  @tool
  @codex
  Scenario: CodexViewImageFacade maps view_image path to InternalFileParams::Read
    Given a CodexViewImageFacade instance
    When the Codex model calls view_image with path "/tmp/screenshot.png"
    Then the facade maps to InternalFileParams::Read with file_path "/tmp/screenshot.png"
    And the facade tool name is "view_image"
    And the facade provider is "codex"

  @tool
  @codex
  Scenario: CodexViewImageFacade accepts detail param for compatibility
    Given a CodexViewImageFacade instance
    When the Codex model calls view_image with path and detail "original"
    Then the facade maps to InternalFileParams::Read ignoring the detail param

  @tool
  @codex
  Scenario: CodexViewImageFacade rejects missing path
    Given a CodexViewImageFacade instance
    When view_image is called with no path parameter
    Then the facade returns a validation error for tool "view_image" mentioning "path"

  @tool
  @codex
  Scenario: CodexViewImageFacade rejects empty path
    Given a CodexViewImageFacade instance
    When view_image is called with an empty path
    Then the facade returns an error

  @tool
  @codex
  Scenario: Tool definition matches Codex CLI spec
    Given a CodexViewImageFacade instance
    When the tool definition is requested
    Then the tool name is "view_image"
    And the description mentions viewing a local image
    And the parameters schema has a required "path" property of type string
    And additionalProperties is false
    And a "detail" property exists for model compatibility
