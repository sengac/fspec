@done
@BUG-174
@bug-174
@tui
@mux
@keyboard-navigation
Feature: Mux empty agent grid — transient board pane instead of a blank 0-pane screen
  """
  BUG-174 (2026-09-04): with a mux layout that has NO non-agent panes
  (e.g. saved panes = [Agent, Agent]) and no open agent sessions,
  recompute_effective_panes() drops every agent slot (MUX-002 rule 3:
  "unfilled agent slots are dropped") — the rendered pane list becomes
  EMPTY. The render pass then paints nothing but the 1-row MUX footer
  ("MUX 0 panes []") and the keyboard is dead:
  forward_mux_event_to_focused_pane swallows every key (focus 0 >=
  0 panes), the BUG-165 stage-4 exit-dialog guard needs a Board pane
  at the focus index (none exists), and only Ctrl+D gets through. The
  saved all-agent config re-arms the trap on every restart.

  Implementation shape: MultiplexLayout::recompute_effective_panes
  floors the rendered pane list at one pane — when the window math
  would produce an EMPTY list, a single full-width TRANSIENT Board
  pane is used instead (live-only: the stored config.panes is never
  mutated, so the user's saved all-agent layout auto-restores the
  moment a session fills an agent slot). The transient pane owns the
  full body area (no dividers) and hosts the live BoardView, so:
  - Esc opens the BoardExitConfirmationDialog (BUG-165 stage-4 guard
    now sees a Board pane at the focus index);
  - Shift+Right opens the new-agent CreateSessionDialog (MUX-002);
  - Enter on a selected work unit starts a session (R8 path);
  - a new session restores the configured layout (the transient pane
    vanishes; agent slots fill from the window).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The rendered mux pane list is NEVER empty: when the configured panes hold
  #      no non-agent kinds AND no agent slot has a session, the grid renders a
  #      single full-width Board pane as a stand-in (the "transient board pane").
  #   2. The transient pane is LIVE-ONLY: the stored config (panes / splits /
  #      orientation / focused_pane) is never mutated by it — no auto-save side
  #      effects, and the saved all-agent layout is restored verbatim when a
  #      session appears.
  #   3. The transient Board pane is FOCUSED (focus index 0), so the BUG-165
  #      stage-4 guard (mux_board_pane_focused) fires and Esc pushes the
  #      BoardExitConfirmationDialog — the same dialog the single Board view
  #      shows. Esc on this pane must NEVER quit directly.
  #   4. Shift+Right on the focused transient Board pane opens the new-agent
  #      CreateSessionDialog (MUX-002 right-edge rule — the rightmost pane is a
  #      non-agent, so the prompt fires regardless of kind).
  #   5. When an agent session is created/opened while the transient pane is
  #      showing, the configured agent slots fill from the window and the
  #      transient Board pane disappears from the rendered list (the config is
  #      restored, not mutated).
  #   6. Layouts that already render at least one pane are UNCHANGED: a
  #      configured Board pane (or Files/Checkpoints pane) is never replaced by
  #      the transient stand-in, and the MUX-002 "no blank panes" rule still
  #      holds for every other case (regression guard — default Board|Agent
  #      with zero sessions renders exactly one Board pane, as today).
  #
  # EXAMPLES:
  #   1. Saved config panes=[Agent,Agent]. /mux -> open 1 agent -> Esc ->
  #      Close Session -> Enter. The screen shows a full-width Board pane +
  #      the MUX footer ("MUX 1 panes [Board]") — NOT a blank body. The board's
  #      work units are visible and selectable.
  #   2. In that state, Esc opens the "Exit fspec?" confirmation dialog.
  #      Confirming quits; cancelling returns to the full-width board pane.
  #   3. In that state, Shift+Right opens the new-agent dialog. Confirming
  #      creates a session; the grid shows exactly ONE agent pane (the
  #      transient board pane is gone; the still-unfilled second agent slot
  #      is dropped per MUX-002 rule 3) with the new agent focused.
  #   4. In that state, Enter on a selected work unit creates the session
  #      (lazy-session path) and the grid shows exactly one agent pane with
  #      the new agent focused.
  #   5. Default layout Board|Agent with zero sessions (the pre-existing
  #      BUG-165 shape) still renders exactly one Board pane — the transient
  #      stand-in only kicks in when the window math would yield ZERO panes.
  #
  # ========================================
  Background: User Story
    As a supervisor running an all-agent mux layout
    I want closing my last agent to land on a usable screen
    So that I am never stuck on a blank terminal with dead keys

  # ========================================
  # SCENARIOS
  # ========================================
  # Rule 1: an all-agent layout with zero sessions renders a full-width board pane
  Scenario: closing the last agent in an all-agent layout shows a full-width board pane
    Given mux mode is active with the pane list agent and agent and no board pane
    And one agent session is open
    When the agent session is closed with Close Session
    Then the TUI is still in mux mode with mux enabled
    And the grid renders exactly one pane: a full-width Board pane
    And no blank body is painted above the MUX footer row
    And the board work units are visible in the board pane

  # Rule 2: the transient pane is live-only — the stored config is untouched
  Scenario: the transient board pane does not mutate the stored mux config
    Given mux mode is active with the pane list agent and agent and no board pane
    And one agent session is open
    When the agent session is closed with Close Session
    Then the stored mux config still lists exactly the panes agent and agent
    And the stored config keeps its orientation and focused pane unchanged
    And the rendered pane list (transient board stand-in) differs from the stored config

  # Rule 3 + BUG-165: Esc on the transient board pane offers the exit dialog
  Scenario: pressing Esc on the transient board pane shows the exit dialog
    Given mux mode is active with the pane list agent and agent and no board pane
    And no agent sessions are open
    When I press the Esc key
    Then the BoardExitConfirmationDialog is shown over the full screen
    And the application does not quit directly
    And confirming the Exit option quits the application

  # Rule 4: Shift+Right on the transient board pane prompts a new agent
  Scenario: Shift+Right on the transient board pane opens the new-agent dialog
    Given mux mode is active with the pane list agent and agent and no board pane
    And no agent sessions are open
    When I press Shift+Right
    Then the new-agent CreateSessionDialog is shown
    And confirming it creates an agent session

  # Rule 5: a new session restores the configured all-agent layout
  Scenario: creating a session restores the configured agent panes
    Given mux mode is active with the pane list agent and agent and no board pane
    And no agent sessions are open
    When I press Shift+Right and confirm the new-agent dialog
    Then the grid shows exactly one agent pane (the transient board pane is gone; the still-unfilled second agent slot is dropped per MUX-002 rule 3)
    And no board pane is rendered
    And the new agent pane is focused

  # Rule 6 (regression guard): the default layout with zero sessions is unchanged
  Scenario: the default board agent layout with zero sessions renders one board pane
    Given mux mode is active with the default Board and Agent panes
    And no agent sessions are open
    Then the grid renders exactly one pane: the configured Board pane
    And the MUX footer shows one pane: Board
    And the stored config is unchanged (Board and Agent)
