@EXT-016
@recovery
Feature: Image Content Recovery — session survives API 400 errors caused by oversized image content in conversation history
  """
  Recovery mechanism lives in stream_loop.rs error handler.
  Pattern: detect invalid_request_error that mentions 'image' or 'dimensions' or 'size',
  call sanitize_image_content() which walks messages backward and replaces Image content
  with text placeholders, emit the error so the LLM sees what went wrong, and return
  the session to idle so it can accept the next message.
  """

  Background: User Story
    As a AI agent user
    I want the session to recover when an API rejects image content
    So that a single bad image does not permanently break my conversation

  @recovery
  @critical
  Scenario: Session recovers when API rejects image content with 400 error
    Given a conversation has image content in its history
    And the API returns a 400 invalid_request_error mentioning "image dimensions"
    When the stream loop handles the error
    Then it should scan recent messages for image content
    And it should replace image content with a text placeholder describing the removal
    And it should emit the error so the LLM knows what went wrong
    And the session should return to idle and accept new input

  @recovery
  @unknown-error
  Scenario: Session survives any content-related 400 error
    Given a conversation has non-text content in its history
    And the API returns a 400 invalid_request_error for an unknown reason
    When the stream loop handles the error
    Then it should show the error to the user
    And the session should remain in Idle state and accept new input
