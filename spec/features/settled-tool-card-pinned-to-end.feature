@done
@tui
@agent-view
@rust
@RPC-399
Feature: Settled tool card must stay pinned to end of output, not jump to start

  """
  This changes the RPC-389 collapse contract for settled cards (first-8 -> last-8). Existing RPC-389 tests and the tool-call-output-collapse.feature settled scenarios must be updated to the end-pinned contract; streaming/modal/diff-bypass behavior stays.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A settled tool-call card whose body exceeds COLLAPSED_LINES (8) shows the LAST 8 body lines (end-pinned), not the first 8
  #   2. A settled tool-call card whose body is 8 or fewer lines shows the full body with no indicator (unchanged)
  #   3. The settled overflow indicator reflects the number of lines hidden ABOVE the visible window and still reads '... +N lines (Enter to view full)' with N = total - 8
  #   4. The streaming tail window (last STREAMING_WINDOW_SIZE = 10 lines, no indicator) is unchanged
  #   5. The full untruncated body remains in ChunkSource.text so the TurnContentModal (Enter to view full) still shows every line
  #   6. Diff cards (is_diff: true) bypass this collapse entirely and are unaffected
  #
  # EXAMPLES:
  #   1. A settled tool card with a 5-line body renders all 5 lines and no '... +N lines' indicator
  #   2. A settled tool card with a 20-line body renders the LAST 8 body lines (line-13..line-20) with a '... +12 lines (Enter to view full)' indicator, and line-12 is hidden
  #   3. A streaming tool card with a 25-line body renders only the last 10 body lines (line-16..line-25) and no indicator (unchanged)
  #   4. A tool card that finishes streaming with a 25-line body stays end-pinned: it shows the LAST 8 body lines (line-18..line-25) plus '... +17 lines (Enter to view full)', so the last lines from streaming remain visible and it does NOT jump to line-1
  #   5. Selecting a collapsed 20-line tool card and pressing Enter opens the TurnContentModal showing all 20 lines (unchanged)
  #
  # ========================================

  Background: User Story
    As a developer watching the agent TUI
    I want to keep a settled tool-call card pinned to the END of its output (the last lines I was reading while it streamed)
    So that the last-line context I was watching stays visible instead of jumping back to the start when the tool finishes

  Scenario: Settled tool card with a short body shows the full body
    Given a settled tool-call card whose body has 5 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show all 5 body lines
    And no "... +N lines" indicator line is shown

  Scenario: Settled tool card with a long body collapses to the last 8 lines
    Given a settled tool-call card whose body has 20 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show the last 8 body lines
    And the earlier body lines are hidden
    And the rendered lines include "... +12 lines (Enter to view full)"

  Scenario: Streaming tool card shows only the last 10 lines
    Given a streaming tool-call card whose body has 25 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show only the last 10 body lines
    And no "... +N lines" indicator line is shown

  Scenario: A finished stream stays pinned to the end of output
    Given a streaming tool-call card whose body has 25 lines
    When the tool-call card finishes streaming
    And the tool-call card is rendered into scrollback lines
    Then the rendered lines show the last 8 body lines
    And the first body line is hidden
    And the rendered lines include "... +17 lines (Enter to view full)"

  Scenario: The full body is preserved for the content modal
    Given a settled tool-call card whose body has 20 lines
    When the full text for the card's turn is requested for the TurnContentModal
    Then the returned text contains all 20 body lines
