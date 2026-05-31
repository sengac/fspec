@done
@navigation
@tui
@RPC-023
Feature: App run-loop forwards Event::Mouse to the active view
  """
  Pre-RPC-023, App::handle_event and the Navigator silently dropped
  Event::Mouse(_) because every layer matched Event::Key only.
  This feature pins the integration that wires the wheel through the
  full event chain into BoardView::handle_event.
  """

  Background: User Story
    As a user of the Rust fspec TUI
    I want my mouse-wheel events to reach the active view
    So that BoardView wheel scrolling works end-to-end through the run loop

  Scenario: Event::Mouse is no longer dropped by the App run loop
    Given an App constructed with a MockBackend, the action bus wired, and the Navigator set to ViewMode::Board
    And BoardView has been rendered through Navigator::render_with_stores so last_content_area is populated
    When Event::Mouse(ScrollDown) inside the BACKLOG content area is fed through App::handle_event
    Then App::handle_event returns a Consumed result
    And the action bus carries Action::SelectNext
