@done
@backward-compat
@provider-abstraction
@providers
@tool-result
@multimodal
@rust
@BUG-140
Feature: Extend ContentPart::ToolResult to carry structured (text+image) content with serde backcompat

  """
  Adds ToolResultPart (tagged enum: Text | Image) and a `parts: Vec<ToolResultPart>` field on ContentPart::ToolResult in codelet_common::types. Existing String `content` field retained and kept in sync (single Text part == content). ToolResultPart::Image reuses ImageSource (Url/Base64) from PROV-091. Serde: `#[serde(tag="type", rename_all="lowercase")]` on ToolResultPart; `#[serde(default)]` on parts to allow older JSON (missing parts) to deserialise with a derived single-Text part. Construction helpers: ContentPart::tool_result_text(id, text, is_error) mirrors old API; ContentPart::tool_result_parts(id, parts, is_error) builds structured form; internal invariant: parts non-empty. No consumer wiring in this unit — BUG-141 wires up the Rhai bridge + claude.rhai, BUG-142/BUG-143 cover Bash and Glob audit paths.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ContentPart::ToolResult must gain a `parts: Vec<ToolResultPart>` field (or equivalent) alongside the legacy `content: String`; ToolResultPart is a tagged enum covering Text { text } and Image { source: ImageSource }.
  #   2. Backward compatibility: when a ToolResult is constructed with only text, its JSON MUST still serialise such that existing callers reading the `content` string field see the same text they always did (no break of PROV-063 tests, no break of the Rhai `messages_to_rhai` contract for text-only tool results).
  #   3. ToolResultPart::Image must reuse the existing ImageSource enum (Url/Base64) introduced by PROV-091 so image wire formats stay consistent between request-side Image parts and tool_result Image parts.
  #   4. Round-trip JSON serialisation: a ContentPart::ToolResult with mixed text+image parts must deserialise back into an equal value; a pure-text ContentPart::ToolResult must also round-trip losslessly under the new shape.
  #   5. When constructing a ToolResult from a plain string (existing call sites), `parts` is initialised with a single ToolResultPart::Text mirroring `content`, so consumers that walk parts see the same text.
  #
  # EXAMPLES:
  #   1. A developer constructs ContentPart::ToolResult { tool_use_id: 'tu_1', content: 'file contents', parts: [Text('file contents')], is_error: false } and serialises it — the JSON contains both `content: "file contents"` and `parts: [{"type":"text","text":"file contents"}]`.
  #   2. A Rhai script author reads a tool_result entry from the messages array and sees both `content` (a plain string summary for text-only consumers) AND `parts` (a structured array) — so they can pick whichever shape fits their provider.
  #   3. A Rhai script author reading a Read-tool result with an image part sees `parts: [{"type":"image","source":{"type":"base64","media_type":"image/png","data":"..."}}]`, and can forward it verbatim into Anthropic's tool_result.content block array.
  #   4. An existing test that serialises ContentPart::ToolResult { content: "hello" } to JSON still passes — the JSON object's `content` key still equals the string "hello" (backcompat preserved).
  #
  # ========================================

  Background: User Story
    As a author of a Rhai-scripted custom provider
    I want to receive tool results that may include image content blocks alongside text in a stable, typed shape
    So that I can construct Anthropic-shape request bodies that embed images inside tool_result blocks without losing fidelity

  Scenario: Serialise text-only ToolResult preserves legacy content field
    Given I build a ContentPart::ToolResult with tool_use_id "tu_1", content "file contents", a single Text part "file contents", and is_error false
    When I serialize the content part to JSON
    Then the JSON type field is "tool_result"
    Then the JSON content field equals "file contents"
    Then the JSON tool_use_id field equals "tu_1"
    Then the JSON is_error field equals false


  Scenario: Serialise text-only ToolResult exposes a single text part
    Given I build a ContentPart::ToolResult via the text helper with content "hello"
    When I serialize the content part to JSON
    Then the JSON parts array has exactly one entry
    Then that entry's type field is "text"
    Then that entry's text field equals "hello"


  Scenario: Serialise ToolResult with a base64 image part
    Given I build a ContentPart::ToolResult via the parts helper with a single ToolResultPart::Image whose source is ImageSource::Base64 media_type "image/png" and data "AAA"
    When I serialize the content part to JSON
    Then the JSON parts array has one entry whose type field is "image"
    Then that entry's source.type field is "base64"
    Then that entry's source.media_type field equals "image/png"
    Then that entry's source.data field equals "AAA"


  Scenario: Round-trip mixed text and image parts through JSON
    Given I build a ContentPart::ToolResult whose parts are Text "summary" followed by Image with Base64 source media_type "image/jpeg" and data "BBB"
    When I serialize it to JSON and deserialize the JSON back into a ContentPart
    Then the deserialized value's parts equal the original parts in order
    Then the deserialized value's tool_use_id, is_error, and content fields equal the original


  Scenario: Deserialize legacy JSON without parts field yields a single text part
    Given I have legacy tool_result JSON {"type":"tool_result","tool_use_id":"tu_x","content":"old output","is_error":false} with no parts field
    When I deserialize the JSON into a ContentPart
    Then the deserialized ContentPart::ToolResult has content equal to "old output"
    Then the deserialized ContentPart::ToolResult has a parts vector containing exactly one ToolResultPart::Text whose text equals "old output"


  Scenario: ToolResult with URL image part serialises the source verbatim
    Given I build a ContentPart::ToolResult via the parts helper with a single ToolResultPart::Image whose source is ImageSource::Url "https://example.com/a.png"
    When I serialize the content part to JSON
    Then the JSON parts array's single entry has type "image"
    Then that entry's source.type field equals "url"
    Then that entry's source.url field equals "https://example.com/a.png"

