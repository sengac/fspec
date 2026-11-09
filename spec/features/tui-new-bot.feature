@TUI-114
@tui
@agent-manager
Feature: New Bot starts an orchestrator bot like New Agent
  """
  A bot is an orchestrator assistant, in the way Claude Code's main agent orchestrates sub-agents: a session whose
  role overlay (BackgroundSession::set_role, prepended as role_preamble on every provider request) instructs it to
  delegate work to sub-agents through the AgentManager tool (spawn / message / await_idle / get_status / close)
  and report their results, instead of doing the work itself. The orchestrator role text and is_bot_role() live
  once in codelet_sessions::bot; the overlay starts with a stable marker line so bots stay recognisable from
  SessionInfo.role. New Bot mirrors New Agent: MenuAction::NewBot -> Action::OpenCreateSessionDialog with a bot
  flag; CreateSessionDialog titles itself "New Bot"; CreateSessionSubmitted carries the flag and dispatch calls
  backend.create_session(Some(role)) / create_isolated_session(Some(role)). No engine changes: the role path,
  AgentManager and supervisor/subordinate links are reused. Research: spec/attachments/TUI-114/research-new-bot.md.
  """

  Background: User Story
    As an fspec user
    I want to start a bot that orchestrates other agents, the way I start a new agent
    So that one session can break work up, run sub-agents on it and report back

  Scenario: The board offers New Bot beside New Agent
    Given the board view is showing
    When the board menu bar is painted
    Then Zone C shows "[ New Bot ]" beside "[ New Agent ]"
    And the Kanban menu lists a "New Bot" row on key "b"

  Scenario: Pressing b on the board opens the New Bot dialog
    Given the board view is showing
    When the user presses "b"
    Then the create-session dialog opens titled "New Bot"

  Scenario: Confirming New Bot creates an orchestrator session and opens it
    Given the New Bot dialog is open
    When the user confirms "Start"
    Then a session is created with the orchestrator bot role
    And the Agent view opens on the new bot session without binding it to a work unit

  Scenario: Cancelling New Bot leaves the board untouched
    Given the New Bot dialog is open over the board
    When the user cancels
    Then no session is created
    And the board view is still showing

  Scenario: The agent bar offers New Bot
    Given the Agent view is showing a session
    When the agent bar is painted
    Then Zone C shows "[ New Bot ]" beside "[ New Agent ]"

  Scenario: The orchestrator role tells the bot to delegate through AgentManager
    Given the orchestrator bot role from codelet-sessions
    When its text is inspected
    Then it starts with the bot marker line
    And it instructs the session to spawn, message and await sub-agents with the AgentManager tool
    And it instructs the session to report its sub-agents' results rather than do the work itself

  Scenario: Bot sessions are marked as bots in the agent bar
    Given an open bot session and an open agent session
    When the agent bar is painted
    Then the bot session's chip carries the "bot:" marker
    And the agent session's chip does not

  Scenario: On a narrow board New Bot gives way before New Agent
    Given the board view is showing in an 80-column terminal
    When the board menu bar is painted
    Then Zone C still shows "[ New Agent ]"
    And "[ New Bot ]" is not painted
