@done
@help-text
@dialog
@tui
@RPC-397
Feature: View-specific accurate help content for board and agent

  """
  Parameterize HelpDialog with a HelpKind enum {Board, Agent} (or two constructors HelpDialog::for_board()/for_agent()). Keep the canonical id 'help-dialog' so compositor.contains guards still work. Store the content slice per-kind; title stays 'Help' (or 'Board Help'/'Agent Help').
  Call site 1: app/events.rs:127 handle_app_shortcut '?' pushes the Board variant (board is the active view when '?' fires there). Call site 2: app/dispatch_slash_commands.rs:31-35 SlashCommandAction::Help pushes the Agent variant. Slash-command descriptions sourced from views/agent/slash_commands.rs SLASH_COMMANDS. Board keys from views/board.rs handle_event. Agent keys from views/agent/dispatch.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The help dialog shown from the board ('?') lists board-specific keybindings only; the one shown from the agent view ('/help') lists agent keybindings AND all slash commands
  #   2. Each keybinding entry has the key(s) and a short one-line explanation of what it does
  #   3. Help content must be accurate to the current source: it must NOT claim 'q Quit' (q does not quit; global quit is Ctrl+D), and must reflect actual bindings
  #   4. The board '?' handler pushes the board help variant; the agent '/help' handler pushes the agent help variant
  #   5. The agent help variant lists every slash command (/help, /clear, /quit, /model, /thinking, /role, /resume, /search, /provider, /debug, /compact, /isolation, /blocklist, /detach, /merge-worktree, /schedule, /loop) with its description
  #
  # EXAMPLES:
  #   1. A user on the board presses '?' and sees board keys like arrows/hjkl navigate, '.' new agent, Shift+Right open agent, [ ] reorder, f/c/d/a viewers, ESC exit — but NOT slash commands
  #   2. A user in the agent view types /help and sees agent keys (Enter send, Shift+Enter newline, Ctrl+C interrupt, scrollback paging, Shift+arrows history/sessions, Tab select turn) AND the full slash-command list with descriptions
  #   3. Neither help variant shows the misleading 'q Quit fspec-tui' line; the quit entry correctly reads 'Ctrl+D Quit'
  #
  # ========================================

  Background: User Story
    As a fspec TUI user pressing '?' on the board or typing /help in the agent view
    I want to see a help dialog whose content is accurate and specific to the view I'm in
    So that I can learn exactly which keys and slash commands work in my current view without seeing wrong or irrelevant entries

  Scenario: Board help shows board keybindings and no slash commands
    Given a HelpDialog constructed for the board rendered against a 200x60 TestBackend
    When the rendered buffer is inspected
    Then it contains board keybindings including "New Agent" and "Reorder"
    And it does not contain any slash command starting with "/"

  Scenario: Agent help shows agent keybindings and the full slash-command list
    Given a HelpDialog constructed for the agent rendered against a 200x60 TestBackend
    When the rendered buffer is inspected
    Then it contains agent keybindings including "Send" and "Interrupt"
    And it contains the slash command "/compact" with its description
    And it contains the slash command "/model" with its description

  Scenario: Neither help variant shows the misleading q Quit line
    Given a HelpDialog constructed for the board and a HelpDialog constructed for the agent
    When each rendered buffer is inspected
    Then neither buffer contains "q       Quit"
    And each buffer contains "Ctrl+D" paired with "Quit"
