@EXT-013
Feature: Screenshot crashes native messaging connection — image data exceeds Chrome 1MB port limit

  """
  Uses OffscreenCanvas and createImageBitmap for resize/conversion in service worker (no DOM needed)
  native-messaging.mjs must use separate constants for incoming vs outgoing message size limits
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Native message reader must use 64 MiB limit for incoming messages (extension→host direction) instead of 1 MB
  #   2. Native message encoder must keep 1 MB limit for outgoing messages (host→extension direction)
  #   3. When reader encounters an oversized message, it must skip exactly the message bytes without corrupting the stream buffer
  #   4. browser_screenshot must resize images to fit within 1568px on the long edge (Claude's optimal processing size)
  #   5. browser_screenshot must convert PNG to JPEG at quality 80% to reduce payload size
  #   6. If the JPEG base64 data for a single image exceeds 800KB, the image must be sliced into vertical tiles that each fit under 800KB
  #   7. Each tile must be returned as a separate image content block in the MCP result content array
  #   8. The total JSON message sent through native messaging must stay well under the 1 MB host→extension limit (each tile under 800KB base64)
  #
  # EXAMPLES:
  #   1. Small screenshot (640x480 simple page) → single JPEG content block, no slicing needed
  #   2. Large screenshot (1728x992 complex page) → PNG ~3MB → resized to 1568x900, JPEG ~200KB → single image block
  #   3. Very tall full-page screenshot (1568x4000) → sliced into multiple ~1000px tiles, each tile is a separate JPEG content block
  #   4. Native reader receives 2MB message from extension → processes it normally (under 64 MiB limit)
  #   5. Native reader encounters oversized message (>64 MiB) → skips it and processes subsequent messages correctly
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to take screenshots of browser tabs without crashing the native messaging connection
    So that I can use browser_screenshot reliably on any page regardless of viewport size or complexity

  Scenario: Small screenshot returns a single JPEG image
    Given the agent has an active MCP connection to the extension
    And the active tab displays a simple page with viewport 640x480
    When the agent calls browser_screenshot
    Then the result contains exactly 1 image content block
    And the image mimeType is "image/jpeg"
    And the image base64 data is under 800KB

  Scenario: Large screenshot is resized to fit within 1568px on long edge
    Given the agent has an active MCP connection to the extension
    And the active tab displays a complex page with viewport 1728x992
    When the agent calls browser_screenshot
    Then the captured image is resized so the long edge is at most 1568px
    And the aspect ratio is preserved
    And the result contains image content blocks with mimeType "image/jpeg"

  Scenario: Very tall screenshot is sliced into multiple tiles
    Given the agent has an active MCP connection to the extension
    And a full-page capture produces a very tall image before resize
    When the agent calls browser_screenshot
    Then the result contains multiple image content blocks
    And each image content block has mimeType "image/jpeg"
    And each image base64 data is under 800KB

  Scenario: Native message reader accepts messages up to 64 MiB
    Given the native messaging host is running
    And the reader is processing messages from the extension
    When a 2MB message arrives from the extension via stdin
    Then the reader decodes and delivers the message successfully
    And subsequent messages are also processed correctly

  Scenario: Native message reader skips oversized messages without corruption
    Given the native messaging host is running
    And the reader is processing messages from the extension
    When a message exceeding 64 MiB arrives from the extension
    Then the reader skips the oversized message
    And subsequent messages in the stream are processed correctly
    And the buffer is not corrupted

  Scenario: Native message encoder preserves 1MB outgoing limit
    Given the native messaging host is running
    When encoding a message larger than 1MB for the extension
    Then the encoder throws an error indicating the message exceeds the maximum size
    And the 1MB limit is enforced for outgoing messages only
