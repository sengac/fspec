@done
@tui
@board
@dialog
@search
@bug-159
Feature: Board search dialog paints a pinned query row and uses a fixed frame rect
  """
  BUG-159: The BOARD-022 WorkUnitSearchDialog never painted the typed query and its
  shrink-to-content centering re-centered the frame as the match list grew.
  FspecDialog gains query_row: Option<&str> (Default = None).
  render_dialog_at paints it as the first content row (pinned under the title,
  above the match rows) in a distinct accent style with a trailing block cursor,
  and reserves one content row for it so the match rows start one row below.
  body_content_rows(rect_height, footer_h, has_query_row) is the single source of
  truth for the body viewport and returns one less when has_query_row is true.
  WorkUnitSearchDialog::render switches from render_dialog (shrink-to-content) to
  render_dialog_at(fixed_dialog_rect(area)) so the frame is static; only the body
  rows scroll. Dialogs that do not set query_row render byte-identically to before.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The dialog paints the live query text on a dedicated query row pinned
  #      immediately under the title (above the match rows), visible at all
  #      times while the dialog is open, including with an empty query (an
  #      empty query row still occupies the pinned slot)
  #   2. The dialog frame is a FIXED rect computed by fixed_dialog_rect(area)
  #      (area.width-4 x area.height-6, centered) and painted via
  #      render_dialog_at, so the frame top-left corner and size are invariant
  #      as the match list grows or shrinks; only the body rows scroll
  #   3. The query row is a first-class part of the FspecDialog descriptor
  #      (query_row: Option<&str>) painted by render_dialog_at itself — the
  #      visual contract stays in dialog_theme, and dialogs that do not set
  #      query_row render byte-identically to before
  #   4. The visible-rows window for the match list is sized by
  #      body_content_rows(rect.height, footer_lines) — the same helper
  #      render_dialog_at uses — so the scroll window and the painted body
  #      always agree; the ad-hoc (area.height - 8).clamp(1, 20) heuristic
  #      is removed
  #
  # EXAMPLES:
  #   1. body_content_rows(20, 1, has_query_row=true) ==
  #      body_content_rows(20, 1, has_query_row=false) - 1 — the pinned query
  #      row consumes exactly one content row from the body viewport
  #   2. 80x24 terminal, dialog open, user types 'auth' → the buffer row at
  #      fixed_dialog_rect(area).y + 2 (border + title) contains '▸ auth';
  #      the match rows start one row below that, and the dialog's top border
  #      row is at fixed_dialog_rect(area).y for both 1-match and 20-match
  #      states
  #   3. A dialog that does not set a query row (e.g. the ConfirmDialog)
  #      renders byte-identically to before BUG-159 — no extra blank row, no
  #      shifted body
  #   4. User types 'zzz' (zero matches) on an 80x24 terminal → the query
  #      row still shows '▸ zzz' directly under the title and the
  #      '(no work units match "zzz")' row appears below it; the dialog
  #      frame border is at the same top row as when 3 matches were visible
  #
  # ========================================
  Background: User Story
    As a Rust TUI board user
    I want to type a query into the board '/' search dialog and see it echoed on a pinned row while the dialog frame stays put
    So that I can tell what query is active and where the match list is scrolling without the dialog jumping around as results grow or shrink

  Scenario: Typing a query paints the query text on a pinned row under the title
    Given the work-unit search dialog is open on an 80x24 terminal
    When I type "auth" into the dialog
    Then the dialog renders the query "auth" on the row immediately below the title
    And the query row is styled with the accent color and a trailing block cursor

  Scenario: The dialog frame is invariant as the match list grows
    Given a board with 20 work units whose ids all contain "a" and only one contains "ab"
    When I open the search dialog and type "a"
    Then the dialog frame top-left corner is at the fixed_dialog_rect position
    When I type "b" so the query is "ab" leaving only 1 match
    Then the dialog frame top-left corner is unchanged

  Scenario: The query row is visible when the body is at maximum height
    Given the work-unit search dialog is open on an 80x24 terminal with 20 matches
    When the match list fills the body viewport
    Then the query row is still painted on the pinned row under the title

  Scenario: The body viewport accounts for the query row in compact layout
    Given a dialog rect of height 6 with a 1-line footer
    When body_content_rows is called with has_query_row = true
    Then the result is one less than body_content_rows with has_query_row = false

  Scenario: Dialogs without a query row render unchanged
    Given a dialog built with build_dialog and no query_row set
    When the dialog is rendered at a fixed rect
    Then the rendered buffer is byte-identical to the pre-BUG-159 output

  Scenario: An empty query still shows the pinned query row
    Given the work-unit search dialog is open with an empty query
    When the dialog is rendered
    Then the query row is painted with a block cursor on the pinned row under the title
