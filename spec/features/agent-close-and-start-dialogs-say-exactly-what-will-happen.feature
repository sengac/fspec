@done
@tui
@dialog
@agent-view
@high
@TUI-124
Feature: Agent close and start dialogs say exactly what will happen
  """
  Architecture notes:
  - Words follow the kind of session (product owner, 2026-11-07): a bot is a session whose role starts with the TUI-114 bot marker (AgentViewStore::is_bot_session); every other session is an agent. No close, start or delete dialog says 'session', 'AI session' or 'conversation'.
  - The backend close (SessionManager::destroy_session) is the same for both kinds: it interrupts the running turn, releases the live session and keeps its saved manifest, so /resume can resume it; for an isolated session it marks the git-session manifest terminated, so only /worktrees prune removes the worktree. It never closes subordinates (ChainOfCommand cleanup only drops the links).
  - Close flow (fspec-tui): the [ Close ] button and the Esc cascade first gather the close facts (get_subordinates and get_supervisors for the focused session, plus the store's status, isolation and kind), then mount ExitConfirmationDialog with them. Confirming closes the bot's sub-agents one by one and then the session (product owner rule, 2026-11-07); each backend close that succeeds is folded through the session-removed cleanup (tab, work-unit links, mux window); the first failure stops the close and is reported, so every session is either fully closed or fully open.
  - Start flow: CreateSessionDialog (New Agent, '.', Enter on a card, Shift+Right, New Bot) and the /resume delete ConfirmDialog. The shared three-button builder paints the description lines and returns the click areas from the same labels.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Words follow the kind of session: a bot says 'bot', every other session says 'agent' (product owner, 2026-11-07)
  #   2. The agent bar's close button is '[ Close ]' for both kinds
  #   3. 'Close Agent?' / 'Close Bot?': idle 'Close'; working 'Stop and Close'
  #   4. The close dialog says the history is kept and can be resumed with /resume
  #   5. An isolated session's close dialog says its worktree is kept until /worktrees prune
  #   6. Closing a bot closes its sub-agents through the same close path; the dialog counts them and the running ones first (product owner, 2026-11-07)
  #   7. Closing a sub-agent closes only it, and says so (product owner, 2026-11-07)
  #   8. Closing an agent never closes its sub-agents; the dialog says how many keep running
  #   9. Closing is all or nothing for each session; failures stop the close and are reported (product owner, 2026-11-07)
  #   10. Start dialogs: 'Start New Agent?' / 'Start Agent for <id>?' / 'Start New Bot?' with 'Start', 'Start in Worktree', 'Cancel'
  #   11. The /resume delete dialog is 'Delete Agent?' / 'Delete Bot?' and says the saved history is deleted
  #   12. Click areas follow the new labels
  #
  # EXAMPLES:
  #   1. Idle agent close dialog
  #   2. Running agent close dialog
  #   3. Isolated agent close dialog
  #   4. Bot with no sub-agents
  #   5. Bot with three sub-agents, one running and one not open here
  #   6. A sub-agent already closed is not counted
  #   7. A sub-agent open in a mux pane closes with its bot
  #   8. A sub-agent that fails to close stops the close
  #   9. Focused sub-agent closes alone
  #   10. Agent with sub-agents leaves them running
  #   11. New Agent, card and New Bot start dialogs
  #   12. /resume delete of a bot
  #
  # ========================================
  Background: User Story
    As a developer closing and starting agents and bots in the TUI
    I want to have every close, start and delete dialog say exactly what will happen to the agent or bot
    So that I never lose work or leave sub-agents running by surprise

  Scenario: The agent bar's close button reads [ Close ] and is clicked where it is painted
    Given an agent is open in the agent view
    When the agent bar is painted
    Then its right-aligned buttons read "[ New Bot ]", "[ New Agent ]" and "[ Close ]"
    And a left click on "[ Close ]" opens the "Close Agent?" dialog

  Scenario: Closing an idle agent says it ends the agent and keeps its history
    Given an idle agent with an empty input is focused
    When the user activates "[ Close ]"
    Then the dialog titled "Close Agent?" says "Closing ends this agent and returns you to the board."
    And it says "Its history is kept: resume it later with /resume."
    And its options are "Close" and "Cancel"

  Scenario: Closing a running agent says closing stops it
    Given a running agent is focused
    When the user activates "[ Close ]"
    Then the dialog titled "Close Agent?" says "This agent is working. Closing stops it and returns you to the board."
    And its options are "Stop and Close" and "Cancel"

  Scenario: Closing an isolated agent says its worktree is kept
    Given an idle isolated agent is focused
    When the user activates "[ Close ]"
    Then the dialog says "Its worktree is kept until you prune it with /worktrees."

  Scenario: Esc on an idle bot opens the Close Bot dialog
    Given an idle bot with an empty input and no sub-agents is focused
    When the user presses Esc
    Then the dialog titled "Close Bot?" says "Closing ends this bot and returns you to the board."
    And it says "It has no sub-agents to close."

  Scenario: Closing a bot closes its sub-agents and counts the running ones first
    Given a bot has three sub-agents, one running, one idle and one not open in this TUI
    When the user activates "[ Close ]"
    Then the dialog titled "Close Bot?" says "Its 3 sub-agents will be closed too (1 running, 1 status unknown)."
    And it says "All their history is kept: resume any of them later with /resume."
    And its options are "Stop and Close All" and "Cancel"
    When the user confirms "Stop and Close All"
    Then the three sub-agents are closed before the bot
    And the bot and its open sub-agents have no tabs and no work-unit links

  Scenario: A sub-agent that is already closed is neither counted nor closed
    Given a bot has two sub-agents and the first one was already closed
    When the user activates "[ Close ]" on the bot
    Then the dialog says "Its 1 sub-agent will be closed too (none running)."
    And confirming closes only the second sub-agent and then the bot

  Scenario: A sub-agent open in a mux pane closes with its bot
    Given the mux shows a bot and its sub-agent in two agent panes
    When the bot is closed with "Close All"
    Then both sessions are closed and the mux stays on with the board pane focused

  Scenario: A sub-agent that cannot be closed stops the close and is reported
    Given a bot has two sub-agents and the second one fails to close
    When the user confirms "Close All"
    Then the first sub-agent is closed
    And the second sub-agent and the bot keep their tabs
    And the failure "The bot was not closed: sub-agent sub-2 could not be closed: refused. 1 of 2 sub-agents were closed." is reported

  Scenario: An agent whose backend close fails keeps its tab and the failure is reported
    Given an idle agent is focused and its backend close fails with "refused"
    When the user confirms "Close"
    Then the agent keeps its tab
    And the failure "The agent was not closed: refused" is reported

  Scenario: Closing a sub-agent closes only it and says so
    Given a sub-agent started by a bot is focused
    When the user activates "[ Close ]"
    Then the dialog titled "Close Agent?" says "Only this agent is closed; the bot that started it keeps running."
    And confirming closes only the sub-agent

  Scenario: Closing an agent leaves its sub-agents running and says so
    Given an agent that started two sub-agents is focused
    When the user activates "[ Close ]"
    Then the dialog says "Its 2 sub-agents keep running."
    And confirming closes only the agent

  Scenario: If the sub-agents cannot be listed nothing is closed and the failure is reported
    Given listing the focused bot's sub-agents fails with "offline"
    When the user activates "[ Close ]"
    Then no close dialog opens
    And the failure "The bot was not closed: its sub-agents could not be listed: offline" is reported

  Scenario: The New Agent dialog says what will be started
    Given the board is shown
    When the user starts a new agent
    Then the dialog titled "Start New Agent?" says "Starts a new agent, not linked to any work unit."
    And it says "Start in Worktree gives it its own new git worktree."
    And its options are "Start", "Start in Worktree" and "Cancel"

  Scenario: The dialog for a work unit says the agent is linked to it
    Given work unit TUI-124 has no agent
    When the user presses Enter on TUI-124
    Then the dialog titled "Start Agent for TUI-124?" says "Starts a new agent linked to work unit TUI-124."

  Scenario: The New Bot dialog says a bot is started
    Given an agent is open in the agent view
    When the user activates "[ New Bot ]"
    Then the dialog titled "Start New Bot?" says "Starts a new bot that hands your tasks to sub-agents."
    And a left click on "Start in Worktree" starts an isolated bot

  Scenario: Deleting from /resume names the kind and says the history is deleted
    Given the /resume list shows a bot and an agent
    When the user presses D on the bot
    Then the dialog titled "Delete Bot?" says "Delete bot" with the bot's id
    And it says "Its saved history is deleted; it can no longer be resumed."
    When the user presses D on the agent
    Then the dialog is titled "Delete Agent?"
