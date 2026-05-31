@done
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: HelloComponent (Background-priority placeholder)
  Background-priority placeholder Component that renders a centered
  static greeting via Layout::vertical([Min, Length, Min]) +
  Layout::horizontal([Min, Length, Min]) — the doc 06 centered-modal
  helper pattern. Never consumes events; ignored events propagate
  through to whichever Critical-priority modal sits on top.
  Replaced by the real list view in RPC-009.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want HelloComponent to render a centered static greeting at Priority::Background and never consume events
    So that the App's pre-populated compositor produces a visible placeholder while the real list view (RPC-009) is being built

  Scenario: HelloComponent renders a centered static text via Layout vertical and horizontal Min Length Min
    Given an isolated HelloComponent rendered onto an 80x24 TestBackend buffer
    When I scan the rendered buffer for the static greeting text
    Then the greeting text appears on a row inside the middle vertical third of the buffer
    And the greeting text is horizontally centered such that its left and right padding columns differ by at most 1
    And the HelloComponent never returns Consumed from handle_event
