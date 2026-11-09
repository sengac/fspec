@done
@MUX-012
@MUX-002
@tui
@navigation
@multi-session
@mux
Feature: Multiple agent panes with grouped agent-view cycling
  """
  Implementation shape: MultiplexLayout gains a `window_start: usize` (agent window offset, clamped to max(0, sessions - agent_slots) on each render/session change). Agent slot i renders session at index window_start + i. MUX-012: Shift+Left/Right step the focus over the rendered panes in layout reading order (depth first, left to right) and stop at both ends; they never rotate the agent window. Shift+Right on the last pane, when it is an agent (or the BUG-174 transient board), emits the same CreateSessionDialog path as non-mux (App::handle_open_create_session_dialog) with NO work-unit attachment; on session creation the window advances so the new (last) session lands in the last agent slot and focus moves there.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All agent panes are grouped together in the agent-view section of the grid; non-agent panes (board, files, checkpoints) stay pinned in place when the agent window moves
  #   2. The /mux pane list defines fixed SLOTS. Non-agent slots (board, files, checkpoints) are pinned in place; agent slots form a window over the ordered list of open agent sessions.
  #   3. An agent slot renders the session at (window_start + slot_index) in the ordered open-session list; when fewer sessions are open than agent slots, only the filled slots render (no blank panes) and the remaining space goes to the other panes.
  #   4. MUX-012 (product owner, 2026-11-08): Shift+Right moves focus to the next pane and Shift+Left to the previous pane in the layout string's reading order (left to right, depth first, at any depth: [board, agent], [agent, agent] gives board, agent, agent, agent). Movement stops at both ends and never wraps; Shift+Left on the first pane does nothing.
  #   5. MUX-012 (product owner, 2026-11-08): Shift+Right on the last pane prompts to create a new agent (same CreateSessionDialog as non-mux mode) only when that pane is an agent; on a last pane of another kind it does nothing. The BUG-174 transient board of an all-agent layout counts as the empty agent slots and prompts (helper decision). The new session is created WITHOUT work-unit attachment; it fills the next agent slot as a new pane, or the last agent slot via a forward window step when every slot is filled, and focus moves to the new agent pane. Cancelling the dialog leaves the panes, the focus and the window unchanged.
  #   6. MUX-012: Shift+Left/Right never rotate the agent window (the earlier rotation on the rightmost agent pane is removed: it moved the window instead of the focus). Sessions outside the window are reached through their chips, which rotate the window to them (MUX-004). (helper decision derived from the product owner's rules, 2026-11-08)
  #   7. Closing an agent session shrinks the open-session list; the agent slots stay in the grid and the window re-clamps to the remaining sessions (no pane is removed from the layout).
  #
  # EXAMPLES:
  #   1. User runs /mux board agent agent with 1 open session. The grid shows [Board][Agent 1] (the second agent slot is not rendered). Pressing Shift+Right twice moves focus to the rightmost pane; pressing Shift+Right again opens the new-agent modal. Confirming creates session 2 and the grid becomes [Board][Agent 1][Agent 2] with focus on the Agent 2 pane.
  #   2. MUX-012: with /mux board, agent, agent, 3 open sessions and the window on [Agent 1][Agent 2], Shift+Right on the Agent 2 pane opens the new-agent dialog instead of rotating; confirming starts Agent 4 and the window shows [Agent 3][Agent 4] with Agent 4 focused; cancelling leaves [Agent 1][Agent 2] with Agent 2 focused.
  #   3. With /mux board agent and the Board pane focused, pressing Shift+Left does nothing (focus stays on Board — no wrap-around to the rightmost pane).
  #   4. MUX-012: with /mux board, agent, agent, 3 open sessions, the window on [Agent 2][Agent 3] and the Agent 3 pane focused, Shift+Left focuses the Agent 2 pane without moving the window; Shift+Left again focuses the Board pane.
  #   5. MUX-012: /mux [board, agent], [agent, agent] with three agents: Shift+Right from the board visits agent 1, agent 2 and agent 3; /mux board, [agent, [files, [checkpoints, agent]]] visits board, agent 1, files, checkpoints, agent 2.
  #   6. MUX-012: /mux board, agent, files with the files pane focused: Shift+Right does nothing.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents
    I want to cycle agent panes in a fixed mux slot layout
    So that monitor all active agent sessions without reconfiguring the grid

  # ========================================
  # SCENARIOS (one per business rule / example)
  # ========================================
  # Rule 3: fewer sessions than agent slots -> only filled slots render
  Scenario: unfilled agent slots are not rendered when fewer sessions are open
    Given mux mode is active with the pane list board, agent and agent
    And one agent session is open
    When the grid is rendered
    Then the grid shows two panes: Board and the agent session
    And no blank or empty agent pane is rendered
    And the Board pane takes the remaining width

  # Example 1: right edge prompts to create a new agent (fills the empty slot)
  Scenario: Shift+Right at the right edge prompts to create a new agent
    Given mux mode is active with the pane list board, agent and agent
    And one agent session is open
    And the rightmost pane is focused
    When I press Shift+Right and confirm the new-agent dialog
    Then the new-agent dialog is shown
    And a second agent session is created WITHOUT work-unit attachment
    And the grid shows three panes: Board, agent 1 and agent 2
    And the agent 2 pane is focused

  # Rule 4: Shift+Left at the first pane stops (no wrap-around)
  Scenario: Shift+Left at the first pane stops without wrapping
    Given mux mode is active with the pane list board and agent
    And the Board pane is focused
    When I press Shift+Left
    Then the Board pane is still focused
    And the focus did not wrap to the rightmost pane

  # Rule 7: closing a session shrinks the list; slots stay and the window re-clamps
  Scenario: closing an agent session keeps the agent slots and re-clamps the window
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the grid shows Board, agent 2 and agent 3
    When the agent 2 session is closed
    Then the grid still shows the Board pane and the agent slots
    And the agent window re-clamps to the remaining sessions
    And no pane is removed from the layout

  Scenario: Shift+Right on a last pane that is not an agent does nothing
    Given mux mode is active with the pane list board, agent and files
    And one agent session is open
    And the files pane is focused
    When I press Shift+Right
    Then no new-agent dialog is shown
    And the files pane is still focused

  Scenario: Shift+Right and Shift+Left walk the panes in layout order and stop at both ends
    Given mux mode is active with the layout "[board, agent], [agent, agent]"
    And three agent sessions are open and the Board pane is focused
    When I press Shift+Right three times
    Then agent 1, agent 2 and agent 3 are focused in turn
    When I press Shift+Right again
    Then agent 3 stays focused and the new-agent dialog is shown
    When I cancel the dialog and press Shift+Left four times
    Then agent 2, agent 1 and the Board pane are focused in turn
    And the fourth Shift+Left leaves the Board pane focused

  Scenario: Shift+Right follows the layout order through a nesting three levels deep
    Given mux mode is active with the layout "board, [agent, [files, [checkpoints, agent]]]"
    And two agent sessions are open and the Board pane is focused
    When I press Shift+Right four times
    Then agent 1, the files pane, the checkpoints pane and agent 2 are focused in turn
    And the files pane is focused after agent 1 although it lies below agent 1

  Scenario: Shift+Right on the last agent pane prompts for a new agent instead of rotating the agent window
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the agent window shows agent 1 and agent 2 with the agent 2 pane focused
    When I press Shift+Right
    Then the new-agent dialog is shown
    And the agent window still shows agent 1 and agent 2
    When I confirm the new-agent dialog
    Then a fourth agent session is created WITHOUT work-unit attachment
    And the agent window shows agent 3 and agent 4
    And the agent 4 pane is focused

  Scenario: Cancelling the new-agent dialog leaves the panes, the focus and the agent window unchanged
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the agent window shows agent 1 and agent 2 with the agent 2 pane focused
    When I press Shift+Right and cancel the new-agent dialog
    Then no agent session is created
    And the agent window still shows agent 1 and agent 2
    And the agent 2 pane is still focused

  Scenario: Shift+Left moves to the previous pane without rotating the agent window
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the agent window shows agent 2 and agent 3 with the agent 3 pane focused
    When I press Shift+Left
    Then the agent 2 pane is focused
    And the agent window still shows agent 2 and agent 3
    When I press Shift+Left again
    Then the Board pane is focused
