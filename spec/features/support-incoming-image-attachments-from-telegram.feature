@done
@BRIDGE-007
@telegram
@bridge
Feature: Support Incoming Image Attachments from Telegram
  """
  Architecture Notes:
  - InboundMessage interface change: add images?: Array<{data: string, media_type: string}> field
  - Add getMediaTypeFromPath(filePath: string): string helper to extract media type from file extension
  - Add async downloadPhotoAsBase64(bot, fileId) helper that calls bot.getFileLink(), fetches the file, returns {data: base64, media_type: string}
  - Modify bot.on('message') handler to check msg.photo, use msg.caption (not msg.text), and call downloadPhotoAsBase64 before forwarding

  Assumptions:
  - Media groups (multiple photos in one message) are out of scope - only single photos handled
  - The codelet/Claude session will handle the images array format - no changes needed on that side
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Download the highest resolution version of the photo (last element in msg.photo array)
  #   2. Convert image to base64 and include with proper media type in the message
  #   3. Detect media type from Telegram's file_path extension (jpg/jpeg→image/jpeg, png→image/png, gif→image/gif, webp→image/webp)
  #   4. Use msg.caption (NOT msg.text) for photo messages - text field is undefined for photos
  #   5. If no active WebSocket session exists when photo arrives, log warning and drop the photo
  #   6. If photo download fails, log error but still forward caption text (if any) to the session
  #   7. Only handle msg.photo (photos) - documents, stickers, and other media types are out of scope
  #   8. Unknown file extension defaults to image/jpeg
  #   9. Telegram Bot API limits file downloads to 20MB - photos are typically well under this limit
  #
  # EXAMPLES:
  #   1. User sends photo with 4 resolution variants - bridge downloads largest one (last in array)
  #   2. User sends photo with caption - WebSocket message includes message text AND images array
  #   3. User sends photo without caption - WebSocket message has empty message string and images array
  #   4. User sends photo.jpg file - image sent with media_type='image/jpeg' based on extension
  #   5. WebSocket message format: {type:'input', session_id:'xxx', message:'caption', images:[{data:'base64...', media_type:'image/jpeg'}]}
  #   6. Photo download times out after 30s - bridge logs error and forwards caption only
  #
  # ========================================
  Background: User Story
    As a Telegram user
    I want to send images to Claude via the bridge
    So that Claude can analyze screenshots, diagrams, and other visual content

  # ====================
  # HAPPY PATH SCENARIOS
  # ====================
  Scenario: Download highest resolution photo from Telegram
    Given I have a connected Telegram bridge session
    And Telegram provides a photo with multiple resolutions
      | index | file_id   | width | height |
      | 0     | thumb_id  | 90    | 90     |
      | 1     | small_id  | 320   | 320    |
      | 2     | medium_id | 800   | 800    |
      | 3     | large_id  | 1280  | 1280   |
    When the bridge processes the photo message
    Then it should request the file with file_id "large_id"
    And the downloaded image should be converted to base64
    And the WebSocket message should include an images array with the base64 data

  Scenario: Handle single resolution photo
    Given I have a connected Telegram bridge session
    And Telegram provides a photo with only one resolution
      | index | file_id   | width | height |
      | 0     | single_id | 800   | 600    |
    When the bridge processes the photo message
    Then it should request the file with file_id "single_id"
    And the WebSocket message should include the image in the images array

  Scenario: Include caption text with photo
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message with caption "What error is this?"
    When the bridge processes the photo message
    Then the WebSocket message should have message "What error is this?"
    And the WebSocket message should include the image in the images array

  Scenario: Handle photo without caption
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message without a caption
    When the bridge processes the photo message
    Then the WebSocket message should have message ""
    And the WebSocket message should include the image in the images array

  Scenario: Use msg.caption not msg.text for photo messages
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message where:
      | field   | value                  |
      | caption | This is the caption    |
      | text    | This should be ignored |
    When the bridge processes the photo message
    Then the WebSocket message should have message "This is the caption"
    And the message should NOT contain "This should be ignored"

  Scenario: Verify WebSocket message structure
    Given I have a connected Telegram bridge session with session_id "test-session-123"
    And Telegram provides a photo message with caption "Hello"
    And the photo downloads successfully as base64 "SGVsbG8gV29ybGQ="
    And the photo file_path is "photos/test.jpg"
    When the bridge processes the photo message
    Then the WebSocket message should have this structure:
      | field                | value            |
      | type                 | input            |
      | session_id           | test-session-123 |
      | message              | Hello            |
      | images[0].data       | SGVsbG8gV29ybGQ= |
      | images[0].media_type | image/jpeg       |

  # ====================
  # MEDIA TYPE SCENARIOS
  # ====================
  Scenario: Pass images to LLM as multimodal input
    Given I have a connected Telegram bridge session
    When the bridge processes the photo message and injects it into the session
    Then the session should pass the image to the LLM as a UserContent::Image
    And Telegram provides a photo message with caption "Describe this image"
    And the LLM should receive both the text message and the image

  Scenario Outline: Detect correct media type from file extension
    Given I have a connected Telegram bridge session
    And Telegram provides a photo with file_path "<file_path>"
    When the bridge processes the photo message
    Then the image should have media_type "<media_type>"

    Examples:
      | file_path            | media_type |
      | photos/file_123.jpg  | image/jpeg |
      | photos/file_456.jpeg | image/jpeg |
      | photos/file_789.png  | image/png  |
      | photos/file_abc.gif  | image/gif  |
      | photos/file_def.webp | image/webp |

  Scenario: Default to image/jpeg for unknown extension
    Given I have a connected Telegram bridge session
    And Telegram provides a photo with file_path "photos/file_xyz"
    When the bridge processes the photo message
    Then the image should have media_type "image/jpeg"

  Scenario: Default to image/jpeg for no extension
    Given I have a connected Telegram bridge session
    And Telegram provides a photo with file_path "photos/file_without_extension"
    When the bridge processes the photo message
    Then the image should have media_type "image/jpeg"

  # ====================
  # ERROR SCENARIOS
  # ====================
  Scenario: Drop photo when no active session
    Given the Telegram bridge has no active WebSocket session
    When a user sends a photo through Telegram
    Then the bridge should log a warning about no active session
    And the photo should not be forwarded

  Scenario: Forward caption when photo download fails
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message with caption "Check this out"
    And the photo download will fail with error "Network timeout"
    When the bridge processes the photo message
    Then the bridge should log an error about the download failure
    And the WebSocket message should have message "Check this out"
    And the WebSocket message should have an empty images array

  Scenario: Drop message completely when photo download fails and no caption
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message without a caption
    And the photo download will fail
    When the bridge processes the photo message
    Then the bridge should log an error about the download failure
    And no WebSocket message should be sent

  # ====================
  # EDGE CASE SCENARIOS
  # ====================
  Scenario: Ignore non-photo media types
    Given I have a connected Telegram bridge session
    When a user sends a document through Telegram
    Then the bridge should handle it as a text message
    And no images array should be included

  Scenario: Ignore sticker messages
    Given I have a connected Telegram bridge session
    When a user sends a sticker through Telegram
    Then the bridge should handle it as a text message
    And no images array should be included

  Scenario: Handle photo with empty caption
    Given I have a connected Telegram bridge session
    And Telegram provides a photo message with caption ""
    When the bridge processes the photo message
    Then the WebSocket message should have message ""
    And the WebSocket message should include the image in the images array
