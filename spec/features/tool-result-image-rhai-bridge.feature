@done
@bug-fix
@rust
@tool-result
@multimodal
@providers
@BUG-141
Feature: Propagate ToolResultContent::Image through Rhai custom-provider bridge so build_request sees image blocks
  """
  rig_message_convert.rs::convert_user_message refactored: instead of collapsing each ToolResult's contents into a single joined string, walk the OneOrMany<ToolResultContent> and build a Vec<ToolResultPart> in order — Text → ToolResultPart::Text, Image → ToolResultPart::Image{source: image_to_source(...)}. If image_to_source returns None (Raw/Unknown), skip that entry. Use ContentPart::tool_result_text when the resulting parts vector is exactly one Text entry (preserves backcompat); otherwise use ContentPart::tool_result_parts.
  messages_to_rhai requires NO change — it serialises via serde and ContentPart::ToolResult.parts already serialises to {type:'text'|'image', ...}. Will validate via integration test that the bridged Rhai value preserves the structure verbatim.
  claude_rhai.rhai build_request must transform user messages whose `content` is an array of part-maps into Anthropic's wire format. Today it copies content verbatim. New logic per part-map: type='text' → {type:'text', text}; type='image' → {type:'image', source}; type='tool_use' → {type:'tool_use', id, name, input}; type='tool_result' → {type:'tool_result', tool_use_id, is_error, content: <transformed inner parts array> }. The inner content for tool_result is built by mapping each ToolResultPart entry to the Anthropic shape: Text→{type:'text',text}, Image→{type:'image',source}. When `parts` is empty/absent, fall back to the legacy `content` string.
  Tests live in: codelet/providers/src/custom/rig_message_convert.rs (#[cfg(test)] for the Rust conversion side) AND a new integration test file codelet/providers/tests/tool_result_image_bridge_tests.rs that uses messages_to_rhai end-to-end. The Rhai-script-side behaviour is verified via a lightweight in-memory Rhai engine test that loads the updated claude_rhai.rhai content and asserts on the body produced by build_request — pattern follows existing custom_http_lifecycle_tests.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. rig_messages_to_internal must map a UserContent::ToolResult containing a ToolResultContent::Image into a ContentPart::ToolResult whose parts vector contains a ToolResultPart::Image with the corresponding ImageSource (Base64 or Url) — never an empty string.
  #   2. When a tool_result contains a mix of Text and Image entries, the resulting ContentPart::ToolResult parts vector preserves the original order (text and image entries appear in the same sequence as the rig source).
  #   3. Image variants whose ImageSource cannot be derived (DocumentSourceKind::Raw or Unknown) are skipped (with a debug log) but never become an empty Text part — text parts that exist alongside them remain intact.
  #   4. Pure-text tool_result inputs MUST continue to use the tool_result_text helper so the legacy `content` string and the single Text part remain in sync (no behavioural change for text-only tool results).
  #   5. messages_to_rhai serialises the ContentPart::ToolResult parts array verbatim into Rhai so that scripts inspecting `msg.content[i].parts` see image entries with shape {type:"image", source:{type:"base64", media_type, data}} (no transformation needed — relies on serde Serialize).
  #   6. claude.rhai's build_request, when emitting a user message, MUST detect tool_result entries inside `msg.content` (an array of part maps) and convert each into an Anthropic-shape `{type:"tool_result", tool_use_id, content:[…]}` block whose inner `content` array carries text and image blocks derived from the structured `parts`.
  #   7. Backward compatibility: when a Rhai script sees a user message whose content is a plain string (text-only message, MessageContent::Text), the script's existing string-passthrough branch must continue to forward it to the Anthropic body unchanged (no regression for non-tool-result user messages).
  #
  # EXAMPLES:
  #   1. Given a rig user message whose content is a single ToolResult with id 'call_img' and one ToolResultContent::Image (Base64 data 'AAA', media_type=PNG), when converted via rig_messages_to_internal, then the resulting Message has MessageContent::Parts containing a single ContentPart::ToolResult whose parts vector is [ToolResultPart::Image{source: ImageSource::Base64{media_type:'image/png', data:'AAA'}}].
  #   2. Given a rig user message whose ToolResult content is [Text('analysis complete'), Image(Base64 'BBB', JPEG)], when converted, then ContentPart::ToolResult.parts equals [ToolResultPart::Text{text:'analysis complete'}, ToolResultPart::Image{source: ImageSource::Base64{media_type:'image/jpeg', data:'BBB'}}] in that order, and ContentPart::ToolResult.content (legacy string) equals 'analysis complete\n[image]'.
  #   3. Given a rig user message whose ToolResult content is [Text('only text')], when converted, then ContentPart::ToolResult is constructed via tool_result_text so legacy `content` equals 'only text' AND parts equals [ToolResultPart::Text{text:'only text'}] (preserving today's behaviour).
  #   4. Given a rig user message whose ToolResult content is one ToolResultContent::Image with DocumentSourceKind::Url('https://x/y.png'), when converted, then the resulting parts vector contains a single ToolResultPart::Image{source: ImageSource::Url{url:'https://x/y.png'}}.
  #   5. Given a Rust-built Vec<Message> containing one User message with a ContentPart::ToolResult (tool_use_id='tu_x', parts=[Image base64 'CCC' image/png]), when messages_to_rhai serialises it, then the resulting Rhai Dynamic value (decoded back to JSON) contains messages[0].content[0].parts[0] == {type:'image', source:{type:'base64', media_type:'image/png', data:'CCC'}}.
  #   6. Given the updated claude_rhai.rhai script and a request whose messages contain one user message with a tool_result part carrying image parts, when build_request runs, then the returned body.messages[0].content equals [{type:'tool_result', tool_use_id:'tu_x', content:[{type:'image', source:{type:'base64', media_type:'image/png', data:'CCC'}}]}].
  #   7. Given the updated claude_rhai.rhai script and a request whose messages contain one user message with a tool_result part carrying [Text('done'), Image base64], when build_request runs, then body.messages[0].content[0].content == [{type:'text', text:'done'}, {type:'image', source:{...}}] in that order.
  #   8. Given the updated claude_rhai.rhai script and a request whose messages contain a user message with a plain text MessageContent (no tool_result), when build_request runs, then body.messages[0] equals {role:'user', content:'<the original text>'} (no transformation, backwards compatible).
  #
  # ========================================
  Background: User Story
    As a developer using a Rhai custom provider with vision-capable models
    I want to have ToolResultContent::Image propagate through the Rhai bridge as structured image parts
    So that my build_request script can build Anthropic-shape tool_result blocks with embedded images and the model can see image data returned from Read

  Scenario: Convert tool_result with single base64 image to structured ToolResultPart::Image
    Given a rig user message whose content is a single ToolResult with id "call_img" and one ToolResultContent::Image carrying a base64 payload "AAA" with media_type PNG
    When I convert the rig history slice via rig_messages_to_internal with no preamble
    Then the resulting Vec<Message> has exactly one User message
    And that message's MessageContent is Parts containing a single ContentPart::ToolResult
    And that ContentPart::ToolResult has tool_use_id "call_img"
    And that ContentPart::ToolResult parts vector equals [ToolResultPart::Image with ImageSource::Base64 media_type "image/png" data "AAA"]

  Scenario: Convert tool_result mixing text and base64 image preserves order and derives legacy content
    Given a rig user message whose ToolResult content is in order Text "analysis complete" followed by Image base64 "BBB" media_type JPEG
    When I convert the rig history slice via rig_messages_to_internal with no preamble
    Then the resulting User message contains a single ContentPart::ToolResult
    And that ContentPart::ToolResult parts vector is [ToolResultPart::Text "analysis complete", ToolResultPart::Image with ImageSource::Base64 media_type "image/jpeg" data "BBB"] in that order
    And that ContentPart::ToolResult legacy content string equals "analysis complete\n[image]"

  Scenario: Convert text-only tool_result keeps tool_result_text shape for backcompat
    Given a rig user message whose ToolResult content is a single Text "only text"
    When I convert the rig history slice via rig_messages_to_internal with no preamble
    Then the resulting User message's ContentPart::ToolResult legacy content equals "only text"
    And the parts vector equals [ToolResultPart::Text "only text"]

  Scenario: Convert tool_result with URL image preserves the URL source
    Given a rig user message whose ToolResult content is a single Image whose DocumentSourceKind is Url "https://x/y.png" with media_type PNG
    When I convert the rig history slice via rig_messages_to_internal with no preamble
    Then the resulting User message's ContentPart::ToolResult parts vector equals [ToolResultPart::Image with ImageSource::Url "https://x/y.png"]

  Scenario: Convert tool_result with unsupported image variant skips the image but keeps siblings
    Given a rig user message whose ToolResult content is in order Text "context" followed by Image whose DocumentSourceKind is Unknown
    When I convert the rig history slice via rig_messages_to_internal with no preamble
    Then the resulting User message's ContentPart::ToolResult parts vector equals [ToolResultPart::Text "context"]
    And the legacy content string equals "context"

  Scenario: messages_to_rhai serialises tool_result image parts verbatim into Rhai
    Given an internal Vec<Message> with one User message whose content is Parts containing ContentPart::ToolResult with tool_use_id "tu_x" and parts [Image base64 "CCC" media_type "image/png"]
    When I serialise it via messages_to_rhai
    And I round-trip the resulting Rhai Dynamic back into JSON
    Then the JSON path messages[0].content[0].type equals "tool_result"
    And the JSON path messages[0].content[0].tool_use_id equals "tu_x"
    And the JSON path messages[0].content[0].parts[0] equals {"type":"image","source":{"type":"base64","media_type":"image/png","data":"CCC"}}

  Scenario: claude_rhai build_request emits Anthropic tool_result block with embedded image for image-only parts
    Given the updated claude_rhai.rhai script loaded into a Rhai engine
    And a request map whose messages contain one user message with a single tool_result part where tool_use_id is "tu_x" and parts is [Image base64 "CCC" media_type "image/png"]
    When I invoke build_request with that request
    Then the returned body.messages has length 1
    And body.messages[0].role equals "user"
    And body.messages[0].content is an array with one entry
    And body.messages[0].content[0] equals {"type":"tool_result","tool_use_id":"tu_x","is_error":false,"content":[{"type":"image","source":{"type":"base64","media_type":"image/png","data":"CCC"}}]}

  Scenario: claude_rhai build_request emits mixed text and image blocks inside tool_result content
    Given the updated claude_rhai.rhai script loaded into a Rhai engine
    And a request map whose messages contain one user message with a single tool_result part where tool_use_id is "tu_y" and parts is [Text "done", Image base64 "DDD" media_type "image/jpeg"]
    When I invoke build_request with that request
    Then body.messages[0].content[0].type equals "tool_result"
    And body.messages[0].content[0].tool_use_id equals "tu_y"
    And body.messages[0].content[0].content equals [{"type":"text","text":"done"},{"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":"DDD"}}] in that order

  Scenario: claude_rhai build_request leaves plain-text user messages unchanged
    Given the updated claude_rhai.rhai script loaded into a Rhai engine
    And a request map whose messages contain one user message whose content is the plain string "hello world"
    When I invoke build_request with that request
    Then body.messages[0].role equals "user"
    And body.messages[0].content equals "hello world"
