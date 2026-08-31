@done
@MUX-002
@tui
@navigation
@multi-session
@mux
Feature: Multiple agent panes with grouped agent-view cycling
  """
  Implementation shape: MultiplexLayout gains a `window_start: usize` (agent window offset, clamped to max(0, sessions - agent_slots) on each render/session change). Agent slot i renders session at index window_start + i. Shift+Right at the rightmost pane emits the same CreateSessionDialog path as non-mux (App::handle_open_create_session_dialog) with NO work-unit attachment; on session creation the window advances so the new (last) session lands in the last agent slot and focus moves there. Shift+Left/Right rotation happens only on the rightmost AGENT pane when all agent slots are filled; otherwise normal focus cycling (left edge stops, no wrap).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All agent panes are grouped together in the agent-view section of the grid; non-agent panes (board, files, checkpoints) stay pinned in place when agent views rotate
  #   2. The /mux pane list defines fixed SLOTS. Non-agent slots (board, files, checkpoints) are pinned in place; agent slots form a window over the ordered list of open agent sessions.
  #   3. An agent slot renders the session at (window_start + slot_index) in the ordered open-session list; when fewer sessions are open than agent slots, only the filled slots render (no blank panes) and the remaining space goes to the other panes.
  #   4. Shift+Left/Shift+Right move pane focus one pane at a time; Shift+Left at the first (leftmost/topmost) pane STOPS (no wrap-around).
  #   5. Shift+Right at the rightmost pane ALWAYS prompts to create a new agent (same CreateSessionDialog as non-mux mode), regardless of the rightmost pane's kind. The new session is created WITHOUT work-unit attachment; it fills the last agent slot via a forward window rotation and focus moves to the new agent pane.
  #   6. With all agent slots filled, Shift+Right on the rightmost agent pane rotates the agent window forward (e.g. [A1][A2] -> [A2][A3]); Shift+Left on it rotates backward (e.g. [A2][A3] -> [A1][A2]). When the window cannot rotate backward (already at the start), Shift+Left falls through to normal focus movement (to the previous pane).
  #   7. Closing an agent session shrinks the open-session list; the agent slots stay in the grid and the window re-clamps to the remaining sessions (no pane is removed from the layout).
  #
  # EXAMPLES:
  #   1. User runs /mux board agent agent with 1 open session. The grid shows [Board][Agent 1] (the second agent slot is not rendered). Pressing Shift+Right twice moves focus to the rightmost pane; pressing Shift+Right again opens the new-agent modal. Confirming creates session 2 and the grid becomes [Board][Agent 1][Agent 2] with focus on the Agent 2 pane.
  #   2. With /mux board agent agent and 3 open sessions, the grid shows [Board][Agent 1][Agent 2]. Focus on the Agent 2 pane, Shift+Right rotates the window forward: [Board][Agent 2][Agent 3]. Shift+Left on the rightmost pane rotates backward: [Board][Agent 1][Agent 2]. The Board pane never moves or changes.
  #   3. With /mux board agent and the Board pane focused, pressing Shift+Left does nothing (focus stays on Board — no wrap-around to the rightmost pane).
  #   4. With /mux board agent agent, 2 open sessions, and focus on the rightmost agent pane (window already at the start [A1][A2]), pressing Shift+Left does NOT rotate — it moves focus left to the Agent 1 pane; pressing Shift+Left again moves focus to the Board pane.
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

  # Rule 5: right edge prompts even when the rightmost pane is not an agent slot
  Scenario: Shift+Right at the right edge prompts to create a new agent even when the rightmost pane is not an agent
    Given mux mode is active with the pane list board, agent and files
    And one agent session is open
    And the files pane is focused
    When I press Shift+Right and confirm the new-agent dialog
    Then the new-agent dialog is shown
    And a second agent session is created WITHOUT work-unit attachment
    And the agent window advances so the new session fills the last agent slot
    And the new agent pane is focused

  # Example 2: window rotation forward and backward with all agent slots filled
  Scenario: Shift+Right rotates the agent window forward when the window can advance
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the grid shows Board, agent 1 and agent 2
    And the rightmost agent pane is focused
    When I press Shift+Right
    Then the grid shows Board, agent 2 and agent 3
    And the Board pane never moved or changed
    And the rightmost agent pane is still focused

  Scenario: Shift+Left on the rightmost agent pane rotates the agent window backward
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the grid shows Board, agent 2 and agent 3
    And the rightmost agent pane is focused
    When I press Shift+Left
    Then the grid shows Board, agent 1 and agent 2
    And the rightmost agent pane is still focused

  # Example 5: rightmost pane is a non-agent (files) and the window is at the
  # last position — Shift+Right prompts; the new session cycles the window
  # forward and the new agent pane is focused
  Scenario: Shift+Right on the rightmost files pane at the last window position prompts to create a new agent
    Given mux mode is active with the pane list board, agent, agent and files
    And three agent sessions are open
    And the agent window shows agent 2 and agent 3
    And the files pane is focused
    When I press Shift+Right and confirm the new-agent dialog
    Then the new-agent dialog is shown
    And a fourth agent session is created WITHOUT work-unit attachment
    And the agent window shows agent 3 and agent 4
    And the agent 4 pane is focused
    And the files pane stays pinned in its slot

  Scenario: Shift+Right on the rightmost agent pane at the last window position prompts to create a new agent
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the agent window shows agent 2 and agent 3
    And the rightmost agent pane is focused
    When I press Shift+Right and confirm the new-agent dialog
    Then the new-agent dialog is shown
    And a fourth agent session is created WITHOUT work-unit attachment
    And the agent window shows agent 3 and agent 4
    And the agent 4 pane is focused

  # Rule 4: Shift+Left at the first pane stops (no wrap-around)
  Scenario: Shift+Left at the first pane stops without wrapping
    Given mux mode is active with the pane list board and agent
    And the Board pane is focused
    When I press Shift+Left
    Then the Board pane is still focused
    And the focus did not wrap to the rightmost pane

  # Example 4: Shift+Left falls through to focus movement when the window cannot rotate backward
  Scenario: Shift+Left falls through to focus movement when the window cannot rotate backward
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the grid shows Board, agent 1 and agent 2
    And the rightmost agent pane is focused
    When I press Shift+Left twice
    Then the agent 1 pane is focused after the first press
    And the agent window did not rotate
    And the Board pane is focused after the second press

  # Rule 7: closing a session shrinks the list; slots stay and the window re-clamps
  Scenario: closing an agent session keeps the agent slots and re-clamps the window
    Given mux mode is active with the pane list board, agent and agent
    And three agent sessions are open
    And the grid shows Board, agent 2 and agent 3
    When the agent 2 session is closed
    Then the grid still shows the Board pane and the agent slots
    And the agent window re-clamps to the remaining sessions
    And no pane is removed from the layout
