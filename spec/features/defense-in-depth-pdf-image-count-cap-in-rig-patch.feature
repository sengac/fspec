@done
@context-window
@bug-168
@tools
@rig
@multimodal
@truncation
@high
Feature: Defense-in-depth image count cap in rig patch parse_tool_result_content
  """
  parse_tool_result_content (rust/patches/rig-core/src/agent/prompt_request/
  streaming.rs) converts a PDF-shape tool result ({"pages": [...],
  "total_pages": N}) into one ToolResultContent::Image per page. The per-page
  defenses (EXT-016 dimension check, per-page rejection) exist, but there is
  NO count cap — any tool emitting a pages array (Read today, MCP or future
  tools tomorrow) can produce an unbounded image list that blows out the
  context window.

  BUG-168 adds a defense-in-depth count cap in this second conversion layer.
  The primary fix lives in the Read tool (tool-side is the source of truth and
  is visible to both front doors); this cap only guards OTHER tools that emit
  PDF-shape JSON.

  Design:
  - const MAX_TOOL_RESULT_PDF_PAGES = 20, matching the CODELET_MAX_PDF_PAGES
  default, so the two layers agree on the budget (single source of truth in
  codelet-common; the patch crate reads the shared constant).
  - When a pages array (top-level OR nested-text variant) exceeds the cap,
  only the first N pages become Image parts; a text part is appended telling
  the model how many pages were dropped and the offset to continue with.
  - The cap is applied to BOTH the top-level pages branch and the nested-text
  ({"type":"text","content":"..."}) twin, so neither path can overflow.
  - Text tool results remain bounded by MAX_TOOL_RESULT_TEXT_BYTES (CMPCT-031);
  image count and text bytes are independent defenses.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. parse_tool_result_content MUST cap the number of ToolResultContent::Image parts produced from a PDF-shape pages array (top-level and nested-text variants) at MAX_TOOL_RESULT_PDF_PAGES (20)
  #   2. When the array exceeds the cap, a trailing text part MUST state how many pages were dropped and the next offset to read from
  #   3. The cap MUST NOT change the behaviour of payloads at or under the cap (all pages become Image parts, exactly as before)
  #   4. The per-image EXT-016 dimension check and the CMPCT-031 text byte bound remain in force and are independent of the count cap
  #
  # EXAMPLES:
  #   1. An MCP tool returns a 67-page PDF-shape result -> 20 Image parts + a text part "47 more pages; continue with offset=21"
  #   2. The Read tool returns a 4-page result (under cap) -> 4 Image parts, no truncation text (unchanged behaviour)
  #   3. The nested-text twin ({"type":"text","content":"{pages...}"}) obeys the same cap
  #
  # ========================================
  Background: User Story
    As an agent loop ingesting tool results
    I want a hard cap on how many image parts a single tool result can produce
    So that no tool can flood the context window with an unbounded image list

  Scenario: A PDF-shape result exceeding the cap is truncated with a continue notice
    Given parse_tool_result_content is called with a result containing 67 pages
    When the helper converts the pages array to ToolResultContent parts
    Then at most MAX_TOOL_RESULT_PDF_PAGES (20) Image parts are returned
    And a trailing text part states how many pages were dropped and the next offset

  Scenario: A PDF-shape result at or under the cap is unchanged
    Given parse_tool_result_content is called with a result containing 4 pages
    When the helper converts the pages array to ToolResultContent parts
    Then exactly 4 Image parts are returned
    And no truncation text part is present

  Scenario: The nested-text variant obeys the same cap
    Given parse_tool_result_content is called with a nested-text result whose content holds 67 pages
    When the helper converts the nested pages array to ToolResultContent parts
    Then at most MAX_TOOL_RESULT_PDF_PAGES (20) Image parts are returned
    And a trailing text part states how many pages were dropped and the next offset

  Scenario: The per-image dimension defense remains independent of the count cap
    Given parse_tool_result_content is called with a result whose first page is oversized (fails EXT-016)
    When the helper converts the pages array
    Then the oversized page is still rejected by the existing dimension check
    And the count cap is applied to the remaining pages independently
