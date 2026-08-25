@tui-component
@done
@TUI-098
Feature: Double-click to resume session in /resume view
  """
  Add a DoubleClickDetector struct to ResumeSessionView that tracks the last click timestamp and row index. On a second click within 300ms on the same row, emit ResumeSessionViewOutcome::Selected
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A double-click (MouseEventKind::Down Left twice within 300ms) on a session row immediately resumes that session
  #   2. Single-click behavior is unchanged — it still only moves the selection cursor
  #   3. The double-click detection must use a timing window (e.g., 300ms) between two consecutive left-button-down events on the same row
  #   4. If the second click is on a different row than the first, it is treated as two independent single-clicks (no resume)
  #   5. The double-click handler must be added to ResumeSessionView::handle_mouse() and the outcome must be routed in mouse_dispatch.rs
  #   6. The footer hint text must be updated to indicate double-click resumes: 'DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel'
  #
  # EXAMPLES:
  #   1. User double-clicks (two left clicks within 300ms) on session 'Project Alpha' at row 3: the session is immediately resumed and the /resume view closes
  #   2. User clicks once on session 'Project Beta' and waits 500ms before clicking again: both clicks are treated as single-clicks (selection moves twice), no session is resumed
  #   3. User clicks on session 'Alpha' at row 2, then quickly clicks on session 'Beta' at row 5: both are treated as single-clicks because the clicks are on different rows, no session is resumed
  #
  # ========================================
  Background: User Story
    As a fspec-tui user
    I want to double-click a session in the /resume picker
    So that I can resume that session with one mouse gesture instead of click-then-Enter

  @double-click
  @resume
  @mouse
  Scenario: Double-click on a session row immediately resumes that session
    Given the /resume session picker is open with 20 sessions and visible_rows is 8
    And the scroll_offset is 0 so rows 0..7 are visible
    When the user double-clicks (two left-button-down events within 300ms) on the third visible row
    Then the selected_index becomes 2
    And the session at index 2 is resumed (ResumeSessionViewOutcome::Selected is emitted)
    And the /resume view closes

  @single-click
  @resume
  @mouse
  Scenario: Two clicks separated by more than 300ms are treated as independent single-clicks
    Given the /resume session picker is open with 20 sessions and visible_rows is 8
    And the scroll_offset is 0 with selected_index 0
    When the user clicks on row 3 and then clicks the same row again after 500ms
    Then the selected_index becomes 3 after the first click
    And the selected_index remains 3 after the second click
    And no session is resumed (ResumeSessionViewOutcome::Selected is NOT emitted)

  @different-row
  @resume
  @mouse
  Scenario: Quick clicks on different rows are treated as independent single-clicks
    Given the /resume session picker is open with 20 sessions and visible_rows is 8
    And the scroll_offset is 0 with selected_index 0
    When the user clicks on row 2 and then quickly clicks on row 5 within 200ms
    Then the selected_index becomes 2 after the first click
    And the selected_index becomes 5 after the second click
    And no session is resumed (ResumeSessionViewOutcome::Selected is NOT emitted)

  @footer-hint
  @resume
  Scenario: The footer hint text indicates double-click resumes a session
    Given the /resume session picker is open
    When the view renders the footer
    Then the footer displays "DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
