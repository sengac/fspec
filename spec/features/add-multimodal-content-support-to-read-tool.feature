@done
@read
@multimodal
@tools
@codelet
@TOOLS-001
Feature: Add Multimodal Content Support to Read Tool

  """
  LAYER ARCHITECTURE:
  1. Read Tool (codelet/tools/src/read.rs): Detects file type, returns structured output
  2. Agent Loop: Parses tool output, converts to appropriate message content type
  3. Provider Layer (rig): Converts message content to provider-specific API format
  
  FILE STRUCTURE:
  - codelet/tools/src/read.rs - Read tool implementation
  - codelet/tools/src/file_type.rs - File type detection (new)
  - codelet-napi/src/session.rs - Agent loop tool result handling
  
  CRITICAL REQUIREMENTS:
  - Supported image formats: PNG, JPG/JPEG, GIF, WEBP, SVG
  - File type detection by extension AND magic bytes fallback
  - Graceful error handling for corrupt/unreadable files
  - Backward compatibility for text files with line numbers
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When I ask the agent to read an image file (PNG, JPG, GIF, WEBP, SVG), the agent MUST display the image visually in the conversation
  #   2. When I ask the agent to read a text file, the agent MUST show the file content with line numbers (existing behavior)
  #   3. When I ask the agent to read a corrupted or unreadable image, the agent MUST show a clear error message instead of crashing
  #   4. Image files without extensions MUST still be recognized by their content
  #
  # EXAMPLES:
  #   1. Developer asks agent to read screenshot.png - agent returns image visually embedded in conversation
  #   2. Developer asks agent to read config.json - agent returns text with line numbers (existing behavior preserved)
  #   3. Agent reads corrupted image file - returns graceful error message instead of crashing
  #
  # ========================================

  Background: User Story
    As a developer using the fspec agent
    I want to read images and PDFs visually in the conversation
    So that I can discuss visual content, screenshots, and documents without leaving the agent

  Scenario: Read PNG image and display visually
    Given a PNG image file exists at screenshot.png
    When I ask the agent to read screenshot.png
    Then the agent displays the image visually in the conversation


  Scenario: Read text file with line numbers
    Given a JSON config file exists at config.json
    When I ask the agent to read config.json
    Then the agent shows the file content with line numbers


  Scenario: Handle corrupted image gracefully
    Given a corrupted image file exists at broken.png
    When I ask the agent to read broken.png
    Then the agent shows a clear error message explaining the file could not be read


  Scenario: Detect image type by content when extension missing
    Given a PNG image file exists at image-without-extension with no file extension
    When I ask the agent to read image-without-extension
    Then the agent detects it as a PNG image and displays it visually

