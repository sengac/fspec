@done
@tui
@board
@dialog
@search
@bug-160
Feature: Board search dialog result rows show a dimmed title/description snippet
  """
  BUG-160: The BOARD-022 WorkUnitSearchDialog result rows show only the
  work-unit id, so while typing the user cannot tell which unit a match is.
  filter_work_units now returns richer matches (id + snippet pairs, in board
  order); the snippet is the title in Id/Title mode and the description in
  Description mode (falling back to the title when a unit has no
  description, so the snippet is never empty). build_rows builds each row
  via the shared label_description_row primitive (marker + label + dimmed
  description) and width-bounds the snippet with truncate_to so a long
  title/description cannot widen the fixed frame rect (BUG-159). The
  selection/scroll math (move_by / ensure_visible / wrap_index) operates on
  the match count only and is unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every result row shows the work-unit id first, followed by a ' - '
  #      separator and a snippet, built via the shared label_description_row
  #      primitive (marker + label + dimmed description)
  #   2. The snippet is mode-aware: the title in Id and Title modes, the
  #      description in Description mode (falling back to the title when a
  #      unit has no description, so the snippet is never empty)
  #   3. The selected row's snippet is not dimmed; unselected rows' snippets
  #      carry Modifier::DIM (label_description_row semantics)
  #   4. The snippet is width-bounded with truncate_to so a long
  #      title/description cannot widen the fixed dialog frame (BUG-159); the
  #      budget is derived from the fixed body width (inner width minus
  #      marker, id width and the ' - ' separator)
  #   5. filter_work_units returns richer matches (id + snippet pairs) in
  #      board order; the existing selection/scroll math (move_by,
  #      ensure_visible, wrap_index) operates on the match count only and is
  #      unchanged
  #
  # EXAMPLES:
  #   - Typing 'auth' in Id mode with unit AUTH-001 titled 'User login'
  #     shows a row '▸ AUTH-001 - User login' (snippet not dimmed on the
  #     selected row)
  #   - In Description mode, unit DOC-001 (title 'Docs', description 'Open
  #     the attachment viewer') shows the row '  DOC-001 - Open the
  #     attachment viewer' with the snippet dimmed because the row is not
  #     selected
  #   - A unit with a 60-character title shows a snippet truncated to the
  #     row width budget with a trailing '…', and the dialog frame width
  #     stays at the fixed rect (BUG-159)
  #
  # QUESTIONS:
  #   (none — all resolved by the BUG-159 merged design and the research note)
  # ========================================
  # SCENARIOS
  # ========================================
  # @BUG-160
  Scenario: A result row shows the id followed by a title snippet
    Given a board with a work unit "AUTH-001" in backlog titled "User login"
    When I open the search dialog and type "auth"
    Then the dialog lists exactly one match "AUTH-001"
    And the result row shows the id "AUTH-001" followed by " - User login"

  Scenario: In Description mode the snippet is the unit description
    Given a board with a work unit "DOC-001" in backlog titled "Docs" whose description contains "viewer"
    When I open the search dialog and switch the search mode to Description
    And I type "viewer" into the dialog
    Then the dialog lists exactly one match "DOC-001"
    And the result row shows the id "DOC-001" followed by " - Open the attachment viewer"

  Scenario: A unit without a description falls back to the title as snippet
    Given a board with a work unit "NO-DESC-1" in backlog titled "No description unit" that has no description
    When I open the search dialog and type "no-desc"
    Then the dialog lists exactly one match "NO-DESC-1"
    And the result row shows the id "NO-DESC-1" followed by " - No description unit"

  Scenario: A long title is truncated with a trailing ellipsis and the frame stays fixed
    Given a board with a single work unit "LONG-001" in backlog whose title is 60 characters long
    When I open the search dialog and render it on an 80x24 terminal
    Then the result row snippet ends with the ellipsis character "…"
    And the dialog frame width is the fixed rect width (not widened by the title)

  Scenario: The selected row snippet is not dimmed but unselected rows are
    Given a board with two work units "AAA-001" and "AAA-002" in backlog both titled "Same title"
    When I open the search dialog and type "aaa"
    And I press Down to move the selection to the second match
    Then the snippet of the selected row "AAA-002" is not dimmed
    And the snippet of the unselected row "AAA-001" is dimmed

  Scenario: The match order and selection math are unchanged by the richer matches
    Given a board with three work units "AAA-001", "AAA-002" and "AAA-003" in backlog
    When I open the search dialog and type "aaa"
    Then the dialog lists the matches in board order "AAA-001", "AAA-002", "AAA-003"
    And pressing Down wraps the selection within the match list
