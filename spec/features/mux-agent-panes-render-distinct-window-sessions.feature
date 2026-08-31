@done
@BUG-163
@mux
@multi-session
@regression
Feature: Mux agent panes render distinct window sessions
  """
  Implementation shape: multiplex/render.rs derives the agent-pane session
  window from the agent_view_store (window_start + slot_index) before
  paint; render.rs paints a per-pane ghost draft (SessionContext.input_draft)
  instead of the live MultiLineInput for the UNfocused agent panes; the
  focused agent pane paints the live composer; AgentView keeps its single
  live MultiLineInput (no second buffer). multiplex/window.rs sync_window
  gains a focused-agent-pane-index parameter; note_session_created receives
  the index of the last rendered agent pane.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each rendered agent pane paints the session at (window_start + slot_index) of the ordered open-session list — header, scrollback and input. The MUX-002 window math was previously applied to pane SLOTS only; now it is applied to the PAINTED content.
  #   2. No two agent panes may render the same session; no agent pane may render a session outside the agent window.
  #   3. The focused agent pane renders the live input composer (the shared MultiLineInput, which always holds the FOCUSED session's draft). All unfocused agent panes render a ghost of their own session's persisted input_draft (or the empty-placeholder hint when the draft is empty).
  #   4. The focused agent pane's ghost draft MUST equal the live composer contents, so moving focus between panes never changes what is painted.
  #   5. Keyboard input always reaches the focused agent pane only (unchanged mux isolation); ghost panes render read-only.
  #
  # EXAMPLES:
  #   1. /mux board agent agent with one open session (agent 1) and the agent 1 pane focused: the grid shows [Board][agent 1] — exactly one agent pane; its header shows agent 1's tokens and its input shows the live composer.
  #   2. The same grid after creating session 2 and rotating the window so both slots are filled ([agent 1][agent 2]): the left agent pane's scrollback shows agent 1's messages and the right agent pane's scrollback shows agent 2's messages — the two panes are never identical.
  #   3. Agent 1's pane holds a live draft "hello"; the user moves focus to the agent 2 pane: agent 1's pane keeps showing "hello" as its ghost draft while agent 2's pane shows the live composer (agent 2's draft, or the placeholder when empty).
  #   4. Agent 1's session is Idle and agent 2's session is Running: agent 1's pane shows its input composer; agent 2's pane (if unfocused) shows its ghost draft, NOT a live "Thinking" spinner — the spinner is only painted in the focused pane.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents in mux mode
    I want every agent pane to render its own agent session
    So that I can see and type into each open agent session without panes duplicating the focused session

  # Rule 1: each agent pane paints the session at window_start + slot_index
  Scenario: each agent pane paints the session at its window slot position
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open with distinct scrollback content
    And the agent window shows agent 1 and agent 2
    When the grid is rendered
    Then the left agent pane header shows agent 1's session index
    And the right agent pane header shows agent 2's session index
    And the left agent pane scrollback shows agent 1's messages
    And the right agent pane scrollback shows agent 2's messages

  # Rule 2: no duplication, no out-of-window sessions
  Scenario: no two agent panes render the same session
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    When the grid is rendered
    Then the two agent panes render different sessions

  # Example 1: the unfilled-slot path still shows a single pane with the live composer
  Scenario: an unfilled agent slot shows a single agent pane with the live composer
    Given mux mode is active with the pane list board, agent and agent
    And one agent session is open
    And the agent pane is focused
    When the grid is rendered
    Then exactly one agent pane is rendered
    And the agent pane header shows the open session's model and tokens
    And the agent pane input shows the live composer

  # Example 3: ghost drafts keep their pane's text while focus moves
  Scenario: moving focus between agent panes keeps the unfocused pane's draft visible
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the agent 1 pane is focused
    And the focused agent composer holds the draft "hello"
    When focus moves to the agent 2 pane
    Then the agent 1 pane still shows the draft "hello" as its ghost input
    And the agent 2 pane shows the live composer

  # Example 4: only the focused agent pane renders the live spinner
  Scenario: an unfocused agent pane renders its ghost draft instead of a live spinner
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the agent 2 session is running
    And the agent 1 pane is focused
    When the grid is rendered
    Then the agent 1 pane input shows the live composer
    And the agent 2 pane input shows its ghost draft without a live spinner
