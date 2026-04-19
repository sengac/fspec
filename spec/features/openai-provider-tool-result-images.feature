@done
@PROV-084 @providers @rust @rig @bug-fix @multimodal @image @tool-result
Feature: OpenAI Chat Completions drops tool-returned images (Read/PDF/MCP)

  """
  Primary fix location: codelet/patches/rig-core/src/providers/openai/completion/mod.rs. The existing `impl TryFrom<message::ToolResult> for Message` at lines 393-414 returns a single Message — NOT suitable because we may need to emit a follow-up user message. Change the integration at lines 507-540 (`TryFrom<OneOrMany<message::UserContent>> for Vec<Message>`) so the tool-results branch (lines 517-524) routes through a NEW helper `tool_result_to_messages(tool_result) -> Result<Vec<Message>, MessageError>` that returns [tool_message] for text-only and [tool_message, user_image_message] for image-bearing results. Flatten the Vec<Vec<Message>> into Vec<Message>. Keep the original `TryFrom<message::ToolResult> for Message` impl for backward compat (it can return ConversionError for image inputs, now an unreachable path inside the crate).
  Reference implementation: the Responses API path at codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs:295-322 already handles mixed text+image tool results correctly using `ToolResultOutput::ContentItems`. We do NOT copy that wholesale (Chat Completions uses a different message shape), but the same pattern — iterate parts, partition into text vs image, emit appropriately — applies. Similar reference: Anthropic at anthropic/completion.rs:485-499 which uses content blocks.
  Tests in codelet/patches/rig-core/src/providers/openai/completion/mod.rs (`#[cfg(test)] mod prov_084_tests`). Each test builds a rig `message::Message::User { content: OneOrMany<UserContent::ToolResult(...)> }`, converts to Vec<Message> via `TryFrom`, and asserts the shape of the result: number of messages, roles, tool_call_ids, content parts. Serialize to JSON and spot-check `image_url.url` prefix. Reuse base64 decoding helpers from PROV-081's test infrastructure if needed.
  Dependency on PROV-083: Once PROV-084 emits image_url parts for base64 tool-result images, those parts go through the same `UserContent::try_from` at mod.rs:416-470 → DocumentSourceKind::Base64 branch. If PROV-083 is NOT merged first, image tool results will fail with the PROV-083 ConversionError instead of Break B, and the follow-up user message will panic. PROV-084 must NOT land until PROV-083 is in.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A rig `message::ToolResult` whose content contains any `ToolResultContent::Image` must convert into a sequence of OpenAI messages that delivers BOTH the tool-result acknowledgement AND the image(s) to the model — no silent drop, no ConversionError.
  #   2. Because the OpenAI Chat Completions `tool` role only accepts string content, image parts MUST be delivered via a follow-up `user` role message immediately after the `tool` message. The `tool` message carries the text parts joined by newlines (or a non-empty placeholder like "[image attached below]" if all parts are images).
  #   3. Image parts on a ToolResult must serialize as `{"type":"image_url","image_url":{"url":"data:<mime>;base64,<payload>"}}` in the follow-up user message (base64 path reuses the PROV-083 fix) or as `{"type":"image_url","image_url":{"url":"<url>"}}` (URL path).
  #   4. Text-only ToolResults must convert exactly as they do today — one OpenAI `tool` message with joined text content. Existing behaviour is PRESERVED for the non-image path.
  #   5. When the tool-result contains multiple images, ALL images must appear in the single follow-up user message (one user message with multiple image content parts) — not one user message per image. This keeps message count minimal and preserves ordering.
  #   6. The `tool_call_id` on the emitted OpenAI `tool` message must match the rig `ToolResult.id` — caller correlation must be preserved.
  #
  # EXAMPLES:
  #   1. A user asks the model to read a PNG file. codelet's Read tool returns an image tool result. The next Chat Completion request contains: (1) the tool-call assistant message, (2) a `tool` role message with a short placeholder like "[image attached below]" and the correct tool_call_id, (3) a `user` role message whose content array holds one `image_url` part pointing at `data:image/png;base64,...`. The model describes the image.
  #   2. A user asks the model to read a text file. Read returns only text. The next request contains the existing single `tool` role message with the text content — no follow-up user message, no regression.
  #   3. A user asks the model to read a PDF in visual mode with 3 pages. Read returns 3 image tool-result parts. The request contains: (1) tool-call, (2) one `tool` message with the tool_call_id, (3) one `user` message with 3 `image_url` parts in page order. The model describes all pages.
  #   4. An MCP tool returns mixed content: a text summary and an image. The tool message carries the text summary; the follow-up user message carries the image. Ordering is text-first tool message, image-bearing user message immediately after.
  #
  # ========================================

  Background: User Story
    As a codelet user asking the model to analyse an image it just read via the Read tool or pulled out of a PDF or received from an MCP tool
    I want to have that image actually reach the vision model and be described
    So that multi-turn image workflows (Read → describe → follow up) work against vLLM and other OpenAI-compatible vision servers

  @unit @rust
  Scenario: Tool result with a single base64 image emits tool-message + follow-up user-message
    Given a rig user message whose content is a `ToolResult` with id "call_abc", one `ToolResultContent::Image` (base64, media_type=image/png), and no text parts
    When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    Then the resulting Vec<Message> has exactly 2 elements
    And the first element is a `tool` role message with tool_call_id "call_abc"
    And the first element's content is a non-empty placeholder string
    And the second element is a `user` role message
    And the second element contains exactly one `image_url` content part
    And that image_url.url starts with "data:image/png;base64,"
    And that image_url.detail equals "auto"

  @unit @rust @regression
  Scenario: Text-only tool result still emits a single tool-message (no regression)
    Given a rig user message whose content is a `ToolResult` with id "call_text" and one `ToolResultContent::Text("hello")`
    When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    Then the resulting Vec<Message> has exactly 1 element
    And the element is a `tool` role message with tool_call_id "call_text"
    And the element's content equals "hello"

  @unit @rust
  Scenario: Tool result with three images yields one tool-message + one user-message with three image parts
    Given a rig user message whose content is a `ToolResult` with id "call_pdf" and three `ToolResultContent::Image` parts in page order (page-1, page-2, page-3)
    When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    Then the resulting Vec<Message> has exactly 2 elements
    And the first element is a `tool` role message with tool_call_id "call_pdf"
    And the second element is a `user` role message
    And the second element contains exactly three `image_url` content parts in page order

  @unit @rust
  Scenario: Mixed text and image tool result puts text on the tool-message and image on the follow-up user-message
    Given a rig user message whose content is a `ToolResult` with id "call_mcp", one `ToolResultContent::Text("summary text")`, and one `ToolResultContent::Image` (base64, media_type=image/jpeg)
    When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    Then the resulting Vec<Message> has exactly 2 elements
    And the first element is a `tool` role message with tool_call_id "call_mcp"
    And the first element's content contains the substring "summary text"
    And the second element is a `user` role message
    And the second element contains exactly one `image_url` content part
    And that image_url.url starts with "data:image/jpeg;base64,"
