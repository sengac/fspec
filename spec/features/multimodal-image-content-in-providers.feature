@done
@provider-abstraction
@rust
@multimodal
@providers
@PROV-091
Feature: Propagate image and multimodal content through the Rhai request bridge

  """
  Architecture notes:
  - Adds ContentPart::Image { source: ImageSource } variant to codelet_common::types
  - ImageSource is a serde-tagged enum: Url { url } | Base64 { media_type, data } with tag "type"
  - ContentPart serializes as { "type": "image", "source": { ... } } (Anthropic-shaped)
  - extract_text_from_content ignores Image parts (no text contribution)
  - convert_assistant_content continues to reject rig::AssistantContent::Image (request-only support)
  - messages_to_rhai / request_to_rhai serialize Image parts verbatim via serde_json → Dynamic bridge
  - custom/response_bridge::part_from_map intentionally does NOT accept "image" (request-only bridge)
  """

  Background: User Story
    As a developer building multimodal custom providers
    I want to pass image content blocks through the core message model into Rhai build_request scripts
    So that each custom provider can serialize images into the API's native format without the core losing fidelity

  Scenario: Serialize an Image ContentPart with a URL source
    Given a ContentPart::Image whose source is ImageSource::Url "https://example.com/a.png"
    When I serialize the content part to JSON
    Then the JSON type field is "image"
    And the JSON source.type field is "url"
    And the JSON source.url field is "https://example.com/a.png"

  Scenario: Serialize an Image ContentPart with a Base64 source
    Given a ContentPart::Image whose source is ImageSource::Base64 with media_type "image/png" and data "AAA"
    When I serialize the content part to JSON
    Then the JSON type field is "image"
    And the JSON source.type field is "base64"
    And the JSON source.media_type field is "image/png"
    And the JSON source.data field is "AAA"

  Scenario: Round-trip an Image ContentPart through JSON
    Given a ContentPart::Image with a Base64 source
    When I serialize it to JSON and deserialize the JSON back into a ContentPart
    Then the deserialized value equals the original Image variant

  Scenario: extract_text_from_content skips Image parts
    Given a MessageContent::Parts containing a Text "hi", an Image with a URL source, and a Text "bye"
    When I call extract_text_from_content on the content
    Then the returned string is "hi\nbye"

  Scenario: messages_to_rhai preserves a URL image part verbatim
    Given a user Message whose content is Parts containing a Text "look" and an Image with URL "https://example.com/a.png"
    When I convert the messages slice via messages_to_rhai
    Then the resulting Rhai array has one message entry
    And the message's content array second entry has type "image"
    And that entry's source map has type "url" and url "https://example.com/a.png"

  Scenario: messages_to_rhai preserves a base64 image part verbatim
    Given a user Message whose content is Parts containing an Image with Base64 source media_type "image/png" and data "AAA"
    When I convert the messages slice via messages_to_rhai
    Then the first content entry has type "image"
    And its source map has type "base64", media_type "image/png", and data "AAA"

  Scenario: convert_assistant_content still rejects assistant-side images
    Given a rig OneOrMany of AssistantContent containing an Image variant
    When I call convert_assistant_content with a provider name
    Then the call returns a ProviderError::Content whose message mentions images not being supported

  Scenario: response_bridge rejects image parts from parse_response
    Given a Rhai response map whose content array contains an entry with type "image"
    When I call rhai_to_completion_response on that map
    Then the call returns Err with a RhaiRuntimeError mentioning unknown content part type "image"
