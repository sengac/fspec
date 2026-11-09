@BUG-582
@done
@tui
@rust
@agent-view
@scrollback
@BUG-534
Feature: A collapsed diff hint names a key, not the removed /expand command
  """
  codelet-fspec-tui only. One shared helper formats the hidden-lines hint ('... +N lines (Tab to select, Enter to view full)'), used by chunk_wrap::collapse_tool_body (tool output), diff_format::collapse_hint (Edit/Write, keeps its gutter indent) and patch_diff::produce_patch_diff (apply_patch). The existing tests that asserted '(select turn to /expand)' (rpc390, rpc391, rpc393, tui111, pending_tool_diff unit test) and the agentview-edit-diff-generation scenario text move to the new hint.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A collapsed Edit, Write or apply_patch diff in the agent view ends with '... +N lines (Tab to select, Enter to view full)', the same hint a collapsed tool output shows; the hint text has one source for all collapsed tool cards
  #   2. No hint in the agent view names /expand: the command was removed by TUI-045 and Enter on a selected turn opens the full turn instead
  #   3. The full diff the turn modal shows carries no collapse hint
  #
  # EXAMPLES:
  #   1. an Edit replacing 60 lines with 60 others: the agent view shows the first 25 diff lines and '... +N lines (Tab to select, Enter to view full)', and no line mentions /expand; the turn modal's full diff has every line and no hint
  #   2. an apply_patch adding a 40-line file: the agent view shows 25 rows (the file header and 24 added lines) and '... +16 lines (Tab to select, Enter to view full)', and no line mentions /expand
  #
  # ========================================
  Background: User Story
    As a user reading tool cards in the TUI agent view
    I want to have a collapsed edit or patch diff tell me how to see the rest with a key that exists
    So that I am never pointed at the /expand command, which was removed

  Scenario: A long Edit diff ends with the Enter to view full hint
    Given a session whose agent edits a file, replacing 60 lines with 60 others
    When the edit's tool result arrives in the agent view
    Then the tool card shows 25 diff lines followed by a "... +N lines (Tab to select, Enter to view full)" hint
    And no line of the tool card mentions "/expand"
    And the full diff the turn modal shows holds every line and no hint

  Scenario: A long apply_patch diff ends with the Enter to view full hint
    Given a session whose agent applies a patch adding a 40-line file
    When the patch's tool result arrives in the agent view
    Then the tool card shows 25 diff rows, the file header and 24 added lines, followed by "... +16 lines (Tab to select, Enter to view full)"
    And no line of the tool card mentions "/expand"
