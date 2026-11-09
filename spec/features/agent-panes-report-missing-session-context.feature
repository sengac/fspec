@BUG-582
@done
@BUG-377
@BUG-391
@tui
@agent-view
Feature: Agent panes report missing session context
  """
  An agent pane paints the composer of one session: the focused pane paints the live composer (paint_input_area in
  rust/fspec-tui/src/views/agent/input_area.rs) and every other mux pane paints a ghost draft (paint_ghost_input_row).
  Before BUG-377 both read the pane's session context with unwrap_or_default, so a pane naming a session that is no
  longer open painted an idle composer or an empty ghost draft, and App::switch_to_session_index restored an empty
  draft when the session at the target index was missing. Now views/agent/pane_composer.rs classifies the pane as
  having no session open (a fresh idle composer), an open session, or a missing session; a missing session paints
  "session <id> is not open" in the input row and logs a tracing error. AgentViewStore::focus_open_session returns the
  focused context or a SessionNotOpen error naming the index and the open-session count, and switch_to_session_index
  restores the draft from that context or logs the error and leaves focus and the composer unchanged.
  BUG-391: AgentViewStore::session_index_for returns None (not a (0, 0) slot) for a session that is not open; the
  SessionHeader takes an Option slot and paints no number for None, and menu_snapshot::chip_inputs builds no chip for
  a session that is not open.
  """

  Background: User Story
    As a developer switching between agent sessions in the TUI
    I want to have a session the TUI can no longer find reported instead of shown as an empty idle composer
    So that a lost session context never silently clears my draft or misstates what the session is doing

  Scenario: Focusing a session index that is not open is refused and keeps the current focus
    Given two agent sessions are open and the first one is focused
    When the store is asked to focus session index 5
    Then it reports that no session is open at index 5 out of 2 open sessions
    And the first session stays focused

  Scenario: An unfocused pane whose session is no longer open says so
    Given an unfocused agent pane names session "gone" that is not open
    When the pane is painted
    Then its input row reads "session gone is not open"
    And the input placeholder hint is not shown

  Scenario: The focused pane whose session is no longer open says so
    Given the focused agent pane names session "gone" that is not open
    When the pane is painted
    Then its input row reads "session gone is not open"
    And the input placeholder hint is not shown

  Scenario: Looking up the slot of a session that is not open reports it as missing
    Given two agent sessions are open and the first one is focused
    When the store is asked for the slot of session "gone"
    Then it reports that session "gone" is not open
    And the open sessions report slots 1 and 2 of 2

  Scenario: The agent bar shows no chip for a session that is not open
    Given two agent sessions are open and the first one is focused
    When the agent bar builds chips for sessions "first" and "gone"
    Then only session "first" gets a chip, numbered 1
