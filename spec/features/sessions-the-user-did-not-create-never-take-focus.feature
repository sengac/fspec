@done
@tui
@session-management
@mux
@multi-session
@TUI-125
Feature: Sessions the user did not create never take focus
  """
  Implementation:
  - App::initialize_session takes a SessionFocus (Take / Keep). Action::SessionObserved (the daemon's session-created broadcast and the reconnect snapshot replay) uses Keep: AgentViewStore appends the tab without moving current_session_index, the active-session watch and the daemon pointer are not touched, and the mux new-agent prompt is not consumed. Action::SessionCreated / WorkUnitSessionCreated (the TUI's own creation answer) use Take; a tab the observation already appended unfocused is focused once on the first creation answer (a per-tab flag on SessionContext), so a repeated answer never refocuses. The bootstrap snapshot replay keeps Take. MuxWindow::sync_window keeps the focused pane's identity (the same agent session or the same pane kind) when panes are added.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User request (2026-11-07): a session the TUI did not create on the user's request (a sub-agent spawned with AgentManager by an agent or a bot, a session created from another client, or one seen again on reconnect) is added as a new tab without taking focus: the current session, the active view and the composer draft stay as they are.
  #   2. In mux view the new session fills a free agent slot (or waits outside the window) without moving the focused pane: the focused pane keeps showing the same session or pane.
  #   3. A session the user creates through the TUI (New Agent / New Bot dialog, a work-unit session, the mux Shift+Right prompt) still opens focused, also when the daemon's session-created event for it arrives before the creation answer.
  #   4. A spawned session that does not take focus does not move the daemon's active-session pointer; when no session was open yet, the new session becomes the current one (there is nothing to keep).
  #   5. At startup the open sessions replayed from the daemon snapshot are seeded as before (the last one replayed is focused unless the session restore re-asserts the daemon's active session); this card does not change startup.
  #
  # EXAMPLES:
  #   1. Viewing agent A with a draft typed; A's AgentManager spawns sub-agent S: a tab for S appears, the agent view still shows A, and the composer still holds A's draft
  #   2. Viewing bot B; B's AgentManager spawns sub-agent S: a tab for S appears and the agent view still shows B
  #   3. On the board with agent A current; a sub-agent S is spawned: the view stays on the board and A stays the current session
  #   4. Mux 'board agent agent' with A focused in the first agent pane and one free agent slot; sub-agent S is spawned: S fills the second agent pane and the first agent pane (A) keeps focus
  #   5. Mux 'agent agent board' with the board pane focused and one session; sub-agent S is spawned: S fills the second agent pane and the board pane keeps focus
  #   6. Mux with a Shift+Right new-agent prompt open; sub-agent S is spawned before the user confirms: S does not take the prompt's slot or focus, and the user's new agent still opens focused
  #   7. The user creates agent N from the New Agent dialog and the daemon's session-created event for N arrives before the creation answer: N opens focused
  #   8. No session open; a session created from another client appears: it becomes the current session
  #
  # ========================================
  Background: User Story
    As a user watching an agent while it spawns sub-agents with AgentManager
    I want to keep working in the agent I am viewing when a new agent session appears
    So that a spawned sub-agent never pulls me away from the agent, draft or pane I am using, and I can still see and open the new session

  Scenario: A sub-agent spawned by the viewed agent opens a tab without taking focus
    Given I am viewing agent A with a draft typed in the composer
    When A's AgentManager spawns sub-agent S and the daemon announces S
    Then a tab for S is open
    And the agent view still shows A
    And the composer still holds A's draft
    And the daemon's active session is not moved to S

  Scenario: A sub-agent spawned by the viewed bot opens a tab without taking focus
    Given I am viewing bot B
    When B's AgentManager spawns sub-agent S and the daemon announces S
    Then a tab for S is open
    And the agent view still shows B

  Scenario: A sub-agent spawned while the board is shown leaves the board and the current session alone
    Given the board is shown and agent A is the current session
    When a sub-agent S is announced by the daemon
    Then the board is still shown
    And A is still the current session

  Scenario: In mux view a spawned sub-agent fills a free agent pane while the focused agent pane keeps focus
    Given mux view "board agent agent" with agent A focused in the first agent pane
    When a sub-agent S is announced by the daemon
    Then the second agent pane shows S
    And the focused pane still shows A

  Scenario: In mux view a spawned sub-agent added before the focused board pane does not move focus
    Given mux view "agent agent board" with one agent open and the board pane focused
    When a sub-agent S is announced by the daemon
    Then the second agent pane shows S
    And the board pane is still focused

  Scenario: A sub-agent spawned while the mux new-agent prompt is open does not take the prompt's pane
    Given mux view with agent A focused and the Shift+Right new-agent prompt open
    When a sub-agent S is announced by the daemon before I confirm the prompt
    Then the focused pane still shows A
    When I confirm the prompt and my new agent N is created
    Then N is the current session and the focused pane shows N

  Scenario: A session the user creates opens focused even when the daemon announces it first
    Given I am viewing agent A
    When I create agent N from the New Agent dialog and the daemon announces N before the creation answer arrives
    Then N is the current session
    And a repeated creation answer for N after I go back to A leaves A current

  Scenario: The first session to appear when none is open becomes the current session
    Given no session is open
    When a session created from another client is announced by the daemon
    Then that session is the current session
