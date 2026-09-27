@done
@tui-component
@agent-view
@rust
@mux
@bug
@bug-fix
@tui
@regression
@multi-session
@bug-194
Feature: Mux: thinking indicator disappears when focus moves off a running agent pane
  """
  Critical implementation requirements: (1) transition state machine semantics (transition_driver::advance_transition rules [busy->Loading/Compacting, busy+Hiding/Showing->reset Loading, !busy+Loading/Compacting->Hiding with cached spinner line, Hiding->Showing->Idle]) are unchanged — only their storage becomes per-session. (2) The spinner glyph cadence (80ms/frame, 10 braille frames, DIM style from components/spinner.rs) and finish-sweep cadence (5 chars / 17ms frame, 34ms phase delay from input_transition.rs) must remain byte-identical to single-view behavior; elapsed_ms for Loading/Compacting derives from per-session spinner_started_at so each pane's spinner animates on its own clock but with the same frame sequence. (3) last_spinner_line capture for the Hiding transition uses the per-session cached line (the line painted in that pane's last frame), NOT the view-level cache. (4) Single-view mode must stay byte-for-byte unchanged (the focused pane == the only pane). (5) Existing source-shape tests pinning 300-LoC file ceilings in views/agent/*.rs must keep passing; keep tick logic in animation.rs and ghost-paint logic in input_area.rs (or a new sibling module) to stay under the ceilings. (6) Per-session transition state must be cleared/reset on SessionContext reset_scrollback and must not leak across sessions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The thinking indicator (InputTransitionState machine: Loading/Compacting/Hiding/Showing) MUST be per-session, not a single shared view-level state on AgentView
  #   2. Every rendered agent pane MUST paint the SAME thinking spinner/finish-sweep rows as the focused pane when its session is Running/Compacting/idle-transition — the indicator must be visually identical to the focused pane's, driven by that pane's OWN session status (user decision: 'it's supposed to be the same thinking spinner - it's not supposed to change')
  #   3. Moving mux focus (Shift+Left/Right, click-to-focus, window rotation, /mux pane changes) MUST NOT reset, skip, or otherwise mutate any session's transition state — the only input to a session's transition machine is that session's own SessionStatus
  #   4. When a session goes Idle, its finish sweep (Hiding -> Showing -> Idle) MUST run to completion for THAT session's pane even if no pane is focused on it; the run-loop redraw gate (tick_should_draw) stays open while ANY session's transition is mid-animation or ANY session is Running/Compacting
  #   5. The live MultiLineInput composer stays exclusive to the FOCUSED agent pane (BUG-163 invariant): only the focused pane's input row hosts the editable composer/cursor; unfocused panes that are idle (transition Idle) keep painting their read-only ghost draft. Only the indicator rows (Loading/Compacting/Hiding/Showing) become per-pane and identical to the focused pane's
  #
  # EXAMPLES:
  #   1. Mux grid [Board | Agent1 | Agent2]; Agent1 is Running. Focus Agent1 -> its input row shows the animated 'Thinking...' spinner. Press Shift+Right to focus Agent2 (Idle): Agent1's pane STILL shows its 'Thinking...' spinner (identical to before, still animating), Agent2's pane shows its idle composer; focus stays on Agent2 for typing
  #   2. Mux grid [Board | Agent1]; Agent1 is Running and the BOARD pane is focused: Agent1's pane still shows its 'Thinking...' spinner exactly as it did before focus moved; the board keeps receiving keyboard input
  #   3. Mux grid [Agent1 | Agent2]; Agent1 is Running, Agent2 is focused (Idle). Agent1 finishes its turn (goes Idle): Agent1's pane plays the same finish sweep (spinner text sweeps out 5 chars/frame, placeholder grows in) and ends with its ghost draft row — even though Agent2's pane has keyboard focus the whole time
  #   4. Mux grid [Board | Agent1 | Agent2]; Agent2 is Compacting while Agent1's pane (Idle) is focused: Agent2's pane shows its 'Compacting...' spinner row (same style/cadence as Thinking, with the compaction label) while Agent1's pane shows its idle composer
  #
  # ASSUMPTIONS:
  #   1. User decision (2026-09-27): unfocused panes show the SAME full thinking spinner as the focused pane (not a dimmed/badged variant), and the finish sweep behaves per-pane identically too ('it's supposed to be the same thinking spinner - it's not supposed to change'). Single-view mode remains byte-for-byte unchanged because the focused pane == the only pane there.
  #
  # ========================================
  Background: User Story
    As a TUI user supervising parallel agents in mux mode
    I want to keep the thinking indicator of a running agent pane visible and animating
    So that not misread an active agent as idle when I move focus to another pane

  # ========================================
  # SCENARIOS
  # ========================================
  # Example 1: the running pane keeps its spinner after focus moves away
  Scenario: a running agent pane keeps its thinking spinner after focus moves to another agent pane
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the agent 1 session is running
    And the agent 2 session is idle
    And the agent 1 pane is focused
    When the grid is rendered
    Then the agent 1 pane input shows the live thinking spinner
    When focus moves to the agent 2 pane
    And the grid is rendered
    Then the agent 1 pane input still shows the live thinking spinner
    And the agent 2 pane input shows the live composer
    And keyboard input still reaches the agent 2 pane only

  # Example 2: focus on a non-agent pane (Board) does not kill the spinner
  Scenario: a running agent pane keeps its thinking spinner when the board pane is focused
    Given mux mode is active with the pane list board and agent
    And one agent session is open
    And the agent session is running
    And the agent pane is focused
    When the grid is rendered
    Then the agent pane input shows the live thinking spinner
    When focus moves to the board pane
    And the grid is rendered
    Then the agent pane input still shows the live thinking spinner
    And the board pane receives keyboard input

  # Example 3: the finish sweep completes per-pane while the pane is unfocused
  Scenario: a running agent pane plays its finish sweep after going idle while unfocused
    Given mux mode is active with the pane list agent and agent
    And two agent sessions are open
    And the agent 1 session is running
    And the agent 2 session is idle
    And the agent 2 pane is focused
    When the agent 1 session becomes idle
    And the grid is rendered
    Then the agent 1 pane input shows the captured spinner line shrinking
    And the agent 2 pane input shows the live composer
    When the agent 1 finish sweep completes
    And the grid is rendered
    Then the agent 1 pane input shows its ghost draft
    And the agent 2 pane input still shows the live composer

  # Example 4: a compacting pane shows its Compacting spinner while unfocused
  Scenario: a compacting agent pane keeps its compacting spinner while another pane is focused
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the agent 1 session is idle
    And the agent 2 session is compacting
    And the agent 1 pane is focused
    When the grid is rendered
    Then the agent 2 pane input shows the live compacting spinner
    And the agent 1 pane input shows the live composer

  # Rule 2 / assumption 1: the unfocused spinner is visually IDENTICAL (same
  # text, style and glyph set) — not a dimmed or reduced variant.
  Scenario: an unfocused running pane paints the identical spinner row as a focused one
    Given mux mode is active with the pane list agent and agent
    And two agent sessions are open
    And both agent sessions are running
    And the agent 1 pane is focused
    When the grid is rendered
    Then the agent 1 pane spinner row contains "Thinking... (Esc to stop)" with a braille glyph
    And the agent 2 pane spinner row contains "Thinking... (Esc to stop)" with the same braille glyph

  # Rule 3: focus movement never mutates a session's transition state
  Scenario: moving focus away and back does not restart a running session's spinner clock
    Given mux mode is active with the pane list agent and agent
    And two agent sessions are open
    And the agent 1 session is running
    And the agent 2 session is idle
    And the agent 1 pane is focused
    When the grid is rendered
    Then the agent 1 pane spinner has a recorded elapsed time since start
    When focus moves to the agent 2 pane and back to the agent 1 pane
    And the grid is rendered
    Then the agent 1 pane spinner elapsed time is strictly greater than before
    And the spinner has not reset to zero

  # Rule 4 / redraw gate: the finish sweep of an unfocused pane is not frozen
  Scenario: the run loop keeps redrawing while an unfocused pane's finish sweep is in flight
    Given mux mode is active with the pane list board and agent
    And one agent session is open
    And the agent session just became idle
    And the board pane is focused
    When the redraw gate is evaluated while the agent session's finish sweep is in progress
    Then the redraw gate stays open even though no session is busy
    When the grid is rendered repeatedly
    Then the agent pane finish sweep advances and ends with the agent pane showing its ghost draft

  # Assumption 1: single-view mode is byte-for-byte unchanged
  Scenario: single-view mode keeps its existing spinner and finish-sweep behavior
    Given mux mode is inactive
    And one agent session is open and running
    When the agent view is rendered
    Then the agent input row shows the live thinking spinner exactly as before
    When the agent session becomes idle
    And the agent view is rendered
    Then the agent input row plays the finish sweep exactly as before
