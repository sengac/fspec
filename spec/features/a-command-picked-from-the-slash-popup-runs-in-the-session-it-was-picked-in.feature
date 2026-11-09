@done
@tui
@rust
@agent-view
@multi-session
@input
@concurrency
@BUG-560
Feature: A command picked from the slash popup runs in the session it was picked in
  """
  codelet-fspec-tui only. Action::SlashCommandSelected(Option<SessionId>, SlashCommandAction): AgentView::handle_popup_key emits it with composer_session (BUG-552, set by App::sync_composer_draft to the BUG-549 bound session), None when no session is open. The App::dispatch arm runs handle_slash_command(action) only when the carried session equals agent_view_store.current_session(); otherwise it is not run and the carried session gets the BUG-552 yellow notice (shared helper with refuses_unfocused_command, same text). The typed bare-command path (BUG-169 BareCommand) already applies the same check in handle_input_submitted. Every test that dispatched SlashCommandSelected(action) passes the focused session (app.current_session()); view-level matches use the new shape; the refac019 action snapshot records it.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A command picked from the slash popup carries the session whose composer it was picked in (the BUG-552 composer session), captured when the pick key is handled; a pick made while no session is open carries no session
  #   2. A picked command runs only while the session it carries is still the focused one (no session carried: only while no session is focused); otherwise it is not run, and the carried session shows '[notice] /<name> was not run: another session was focused before it was handled' (the BUG-552 decision for commands that do not name their session; no popup command names its session)
  #   3. A focus change handled between the pick and the command never takes the command: the newly focused session's scrollback, history and draft are untouched
  #   4. With no focus change in between, a picked command runs on the focused session exactly as before
  #   5. A pick made while no session was open that is handled after a session took focus is not run and shows nothing in that session (logged only): it was picked in no session (conservative option, see question)
  #
  # EXAMPLES:
  #   1. s-1 is focused and s-2 holds the draft 's2 text'; a chip click focusing s-2 is already waiting when the user picks /clear from the popup in s-1: no history is cleared on the daemon, s-2 keeps 's2 text', and s-1 shows a notice that /clear was not run
  #   2. s-1 is focused; a new session s-2 taking focus is already waiting when the user picks /compact from the popup: no session is compacted and s-1 shows a notice that /compact was not run
  #   3. s-1 is focused and nothing is waiting when the user picks /clear from the popup: s-1's history is cleared on the daemon
  #   4. no session is open and nothing is waiting when the user picks /help from the popup: the help dialog opens
  #   5. no session is open; a new session s-1 taking focus is already waiting when the user picks /clear from the popup: nothing is cleared on the daemon and s-1 shows no notice
  #
  # QUESTIONS (ANSWERED):
  #   Q: a pick made while no session was open (it carries no session) is handled after a session took focus: should it run on that session, or not run? Nothing was typed in any session, so there is no session to show a notice in.
  #   A: Conservative option taken (supervisor instruction, 2026-11-06: take the most conservative option, pending product owner review): it is not run, nothing is shown in the newly focused session, and the refusal is logged (tracing warn). Same rule as BUG-552: a command never acts on a session it was not typed or picked in.
  #
  # ========================================
  Background: User Story
    As a user picking commands from the slash popup in the TUI agent views
    I want to have a command I pick from the popup act only on the session whose composer I picked it in, even if focus moves before the TUI handles it
    So that a picked /clear, /compact or dialog command never clears, compacts or changes a session I did not pick it in

  # ─────────────────────────────────────────────────────────────────────
  # A focus change waiting to be handled never takes the pick (rules 1-3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A /clear picked while a chip click is waiting clears no other session
    Given session s-2 holds the draft "s2 text"
    And session s-1 is focused
    And a chip click focusing s-2 is waiting to be handled
    When the user picks "/clear" from the slash popup
    And the TUI handles the waiting actions
    And the draft save window has passed
    Then no session history is cleared
    And the agent view shows s-2 with the composer holding "s2 text"
    And the daemon draft of s-2 is "s2 text"
    And s-1 shows a notice that /clear was not run because another session was focused

  Scenario: A /compact picked while a new session is taking focus compacts no session
    Given session s-1 is focused
    And a new session s-2 taking focus is waiting to be handled
    When the user picks "/compact" from the slash popup
    And the TUI handles the waiting actions
    Then no session is compacted
    And the agent view shows s-2
    And s-1 shows a notice that /compact was not run because another session was focused

  # ─────────────────────────────────────────────────────────────────────
  # Without a focus change a pick runs as before (rule 4)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A /clear picked with nothing waiting clears the focused session
    Given session s-1 is focused
    When the user picks "/clear" from the slash popup
    And the TUI handles the waiting actions
    Then the history of s-1 is cleared on the daemon
    And s-1 shows no notice that /clear was not run

  Scenario: A /help picked while no session is open opens the help dialog
    Given no session is open
    When the user picks "/help" from the slash popup
    And the TUI handles the waiting actions
    Then the help dialog is open

  # ─────────────────────────────────────────────────────────────────────
  # A pick made in no session (rule 5)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A /clear picked while no session is open is not run on a session that takes focus first
    Given no session is open
    And a new session s-1 taking focus is waiting to be handled
    When the user picks "/clear" from the slash popup
    And the TUI handles the waiting actions
    Then no session history is cleared
    And the agent view shows s-1
    And s-1 shows no notice that /clear was not run
