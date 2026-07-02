@done
@rust
@agent-view
@tui
@RPC-391
Feature: Render colored Edit/Write diffs in the Rust agent view
  """
  Consumes RPC-390 diff_format module. Touch points: chunk_processor.rs (handle_tool_call capture, handle_tool_result produce), chunk_wrap.rs (marker decode into ratatui spans), session_context.rs (pending_tool_diffs map), ChunkKind::ToolCall is_diff flag, Rust TurnContentModal full-diff decode.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Edit old_string/new_string and Write content are captured at tool-call time (keyed by tool_call_id) and consumed on the matching ToolResult to build a diff
  #   2. Removed diff lines render with a dark-red (#8B0000 / rgb 139,0,0) background and white text
  #   3. Added diff lines render with a dark-green (#006400 / rgb 0,100,0) background and white text
  #   4. Context lines render with a gray line-number gutter and default-white content; the [R]/[A] marker characters are stripped before display
  #   5. Diff cards bypass the RPC-389 8-line tool-output collapse (they self-collapse at 25 inside the diff formatter)
  #   6. Non-Edit/Write tool results (Bash, Grep, etc.) render unchanged with no diff coloring (no regression)
  #   7. The full (uncollapsed) diff is retained for the turn-content modal and decoded there too (markers never shown literally)
  #
  # EXAMPLES:
  #   1. An Edit replacing one line shows the old line on a red background and the new line on a green background
  #   2. A Write of a new 3-line file shows three green-background lines
  #   3. A Bash tool result still renders plain white with no diff coloring
  #   4. An Edit producing more than 25 display lines shows 25 lines plus a '... +N lines' indicator inline, while the modal shows the full diff
  #   5. An Edit whose pending input was not captured (malformed JSON) falls back to plain raw text without panicking
  #
  # ========================================
  Background: User Story
    As a fspec-tui user
    I want to see Edit/Write tool results as colored red/green diffs in the agent view
    So that I can read what changed at a glance, matching the TypeScript client

  Scenario: Edit replacing one line shows the old line on red and the new line on green
    Given an Edit tool call whose old_string and new_string differ in one line is captured at tool-call time
    When the matching ToolResult arrives and the diff card is wrapped into lines
    Then the removed line span has a background of rgb 139,0,0 and white text
    And the added line span has a background of rgb 0,100,0 and white text

  Scenario: Write of a new three-line file shows three green-background lines
    Given a Write tool call whose content has three lines is captured at tool-call time
    When the matching ToolResult arrives and the diff card is wrapped into lines
    Then three line spans each have a background of rgb 0,100,0 and white text
    And no removed-line background appears

  Scenario: A Bash tool result renders plain white with no diff coloring
    Given a Bash tool call and its ToolResult with no captured pending diff
    When the tool card is wrapped into lines
    Then no span carries a red or green diff background
    And the card is collapsed by the existing eight-line tool-output rule

  Scenario: An Edit over the collapse limit shows 25 lines inline while the modal shows the full diff
    Given an Edit producing more than 25 diff display lines is captured at tool-call time
    When the matching ToolResult arrives and the diff card is wrapped into lines
    Then the inline body shows 25 display lines plus a '... +N lines' indicator
    And the retained full diff exposed to the turn-content modal contains all display lines

  Scenario: An Edit with uncaptured pending input falls back to raw text without panicking
    Given an Edit tool call whose input is malformed JSON so no pending diff is captured
    When the matching ToolResult arrives and the tool card is wrapped into lines
    Then the raw ToolResult content is shown as plain text with no diff coloring
    And no panic occurs

  Scenario: Context diff lines render with a gray line-number gutter and white content
    Given an Edit diff card containing a context line of the form '  250   foo'
    When the diff card is wrapped into lines
    Then the line-number gutter span is gray and the content span is white

  Scenario: The marker characters are stripped before display
    Given an Edit diff card with removed and added lines
    When the diff card is wrapped into lines
    Then no rendered span text contains the literal '[R]' or '[A]' marker

  Scenario: Diff cards bypass the eight-line tool-output collapse
    Given an Edit diff card whose collapsed body has more than eight lines
    When the diff card is wrapped into lines
    Then no '... +N lines (Enter to view full)' indicator from the eight-line collapse appears
    And all of the diff body lines are rendered
