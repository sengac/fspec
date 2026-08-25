@TUI-097
@tui
@resume
@scrollbar
Feature: Resume view proportional scrollbar
  """
  Architecture notes:
  - Use crate::views::diff_common::render_pane_scrollbar for scrollbar rendering
  - Scrollbar appears only when session count exceeds visible rows
  - Content width reduced by 1 column when scrollbar is shown
  - Scrollbar uses proportional thumb positioning: thumb_h = (visible * h) / total, thumb_pos = (scroll_offset * h) / total
  - Mirrors CheckpointsView pattern (views/checkpoints/render.rs:125-157)
  """

  Background: User Story
    As a developer using the Rust TUI
    I want to see a proportional scrollbar in the /resume view
    So that I know my position in the session list when it exceeds the visible area

  @scrollbar
  @overflow
  Scenario: Scrollbar appears when session count exceeds visible rows
    Given the resume view has 30 sessions
    And the body area height is 20 rows
    When the view renders the session rows
    Then a proportional scrollbar is rendered on the rightmost column
    And the content width is reduced by 1 column to accommodate the scrollbar

  @scrollbar
  @no-overflow
  Scenario: No scrollbar when session count fits in visible area
    Given the resume view has 5 sessions
    And the body area height is 20 rows
    When the view renders the session rows
    Then no scrollbar is rendered
    And the content uses the full body width

  @scrollbar
  @proportional
  Scenario: Scrollbar thumb position is proportional to scroll offset
    Given the resume view has 30 sessions
    And the body area height is 20 rows
    And the user has scrolled to session index 15
    When the view renders the session rows
    Then the scrollbar thumb is positioned at approximately half the track height

  @scrollbar
  @glyphs
  Scenario: Scrollbar uses DIM styled glyphs for thumb and track
    Given the resume view has 30 sessions
    And the body area height is 20 rows
    When the view renders the session rows
    Then the scrollbar thumb uses the ■ glyph with DIM modifier
    And the scrollbar track uses the │ glyph with DIM modifier
