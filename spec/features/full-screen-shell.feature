@model-selection
@done
@ts-parity
@rust
@model-selector
@tui
@RPC-337
Feature: Shared full-screen shell scaffold
  """
  Shell shape: free function render_full_screen_scaffold(area, buf, title/count/suffix, footer_hint, body_fn(body_area, buf), Option<&ConfirmDialog> overlay). Body closure receives body_area so a view (&mut self) can capture body height itself; no shell struct/trait.
  The shell splits the area into exactly four vertical constraints: title Length(1), separator Length(1), body Min(0), footer Length(1). It paints an OPTIONAL ConfirmDialog overlay over the full area AFTER the body (Some painted, None skipped).
  """

  Background: User Story
    As a fspec TUI user
    I want a shared full-screen scaffold behind all mode-views
    So that the scaffold is not duplicated across views

  Scenario: Shell renders a view with a title count and no overlay
    Given the Resume Session view has 5 available sessions
    When the shared shell renders it onto the full area
    Then the title row reads "Resume Session (5 available)"
    And the body lists the 5 session rows
    And the footer shows the static hint
    And no ConfirmDialog overlay is painted

  Scenario: Shell paints an optional ConfirmDialog overlay over the full area
    Given the Provider Settings view has a delete confirmation active
    When the shared shell renders it onto the full area
    Then the list body is painted first
    And the ConfirmDialog overlay is painted over the full area on top of the body

  Scenario: Shell skips the overlay slot when no overlay is supplied
    Given a view with no destructive action pending
    When the shared shell renders it onto the full area
    Then the body renders normally
    And no overlay is painted over the body

  Scenario: Shell reports body height to the body renderer
    Given a terminal area that is 24 rows tall
    When the shared shell splits the area into title, separator, body and footer
    Then the body sub-rect height reported to the body renderer is 21

  Scenario: Shell collapses the body gracefully on a tiny area
    Given a terminal area that is 3 rows tall or smaller
    When the shared shell splits the area
    Then the body sub-rect height is 0
    And the body renderer receives height 0 and produces no output
