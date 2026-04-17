@done
@CMPCT-031 @cli @rust @compaction @context-management @resilience
Feature: Defense-in-depth bound on tool_result text size in rig patch parse_tool_result_content
  parse_tool_result_content in codelet/patches/rig-core/src/agent/prompt_request/streaming.rs
  (pre-change lines 33-149) bounds oversized image/PDF payloads but wraps plain text
  verbatim at line 149 with vec![ToolResultContent::text(result)]. This work unit adds
  a byte bound on the text branch so no single verbose tool (validate-tags, cargo
  clippy, find, grep) can force megabytes of content into chat_history.

  Design:
  - MAX_TOOL_RESULT_TEXT_BYTES constant (default 64 * 1024 = 65536 bytes).
  - When a text tool_result's UTF-8 byte length EXCEEDS the bound, the text
    content is replaced by a truncation-marker JSON object with fields:
    status (truncated), original_bytes, max_bytes, preview (first 2048 bytes),
    suffix (last 512 bytes), hint (remediation string).
  - Content equal to or under the bound is stored verbatim.
  - Image/PDF size bounds remain untouched.
  - Helper is a private fn in the patch module; tests live alongside it under
    #[cfg(test)] mod tests. Uses serde_json::json! to guarantee proper escaping.

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. parse_tool_result_content enforces MAX_TOOL_RESULT_TEXT_BYTES (default 64 KiB) on plain-text tool results
  #   2. When exceeded, content is replaced with a JSON truncation marker (status, original_bytes, max_bytes, preview, suffix, hint)
  #   3. The bound applies at content ingestion so chat_history never holds the full oversized payload (not even transiently)
  #   4. Image/PDF size bounds remain unchanged — only the TEXT branch is bounded by this change
  #   5. A single catch-all helper performs the bounding; invoked only from the text branch to keep the patch minimal and auditable
  #
  # EXAMPLES:
  #   1. 200 KiB text → marker with 2 KiB preview + 512 B suffix
  #   2. 32 KiB text (under bound) → stored verbatim, no marker
  #   3. exactly 64 KiB text (equal to bound) → stored verbatim (strict >, not >=)
  #   4. oversized image still rejected by existing image check
  #   5. marker JSON valid UTF-8 and self-describing
  #   6. validate-tags --verbose on 3000 files does not cascade into PromptCancelled
  #
  # ========================================

  Background: User Story
    As a AI agent running in the fspec interactive stream loop
    I want oversized plain-text tool_result payloads truncated at a fixed byte bound before they enter chat_history
    So that no single verbose tool can force the compaction system to swallow megabytes of text

  Scenario: A 200 KiB text tool result is replaced by a truncation marker
    Given parse_tool_result_content is called with a plain-text payload of 200 KiB
    When the helper evaluates the byte length against MAX_TOOL_RESULT_TEXT_BYTES
    Then the returned ToolResultContent::text payload is a JSON truncation marker
    And the marker has field "status" equal to "truncated"
    And the marker has field "original_bytes" equal to the original byte length
    And the marker has field "max_bytes" equal to MAX_TOOL_RESULT_TEXT_BYTES
    And the marker has field "preview" containing the first 2048 bytes of the original text
    And the marker has field "suffix" containing the last 512 bytes of the original text
    And the marker has a non-empty "hint" field

  Scenario: A 32 KiB text tool result is stored verbatim without a truncation marker
    Given parse_tool_result_content is called with a plain-text payload of 32 KiB
    When the helper evaluates the byte length against MAX_TOOL_RESULT_TEXT_BYTES
    Then the returned ToolResultContent::text payload equals the original 32 KiB verbatim
    And the payload does NOT contain a JSON truncation marker

  Scenario: A text payload exactly at the bound is stored verbatim
    Given parse_tool_result_content is called with a plain-text payload of exactly MAX_TOOL_RESULT_TEXT_BYTES bytes
    When the helper evaluates the byte length
    Then the returned ToolResultContent::text payload equals the original verbatim
    And the bound condition is a strict greater-than (`>`) not greater-than-or-equal (`>=`)

  Scenario: An oversized image is still rejected by the existing image size check
    Given parse_tool_result_content is called with an oversized IMAGE payload
    When the helper evaluates the payload
    Then the existing image-size rejection path fires unchanged
    And the new text bound is NOT consulted
    And the behavior matches the pre-change image handling exactly

  Scenario: The truncation marker is valid UTF-8 JSON that the model can reason about
    Given parse_tool_result_content has produced a truncation marker for an oversized text payload
    When the marker is read back as UTF-8 and parsed as JSON
    Then parsing succeeds with no errors
    And every field value round-trips cleanly (no mojibake, no unescaped control chars)
    And the top-level object contains exactly the expected keys: status, original_bytes, max_bytes, preview, suffix, hint

  Scenario: Running a verbose tool no longer cascades into PromptCancelled
    Given an fspec interactive session whose agent has just invoked a tool that returned 500 KiB of plain text output
    When the rig patch processes that tool result via parse_tool_result_content
    Then the resulting chat_history entry for that tool_result holds only the truncation marker
    And the chat_history byte count for that tool_result is bounded by a small constant (marker overhead + preview + suffix)
    And the next LLM turn does NOT exceed the context window from this tool_result alone
