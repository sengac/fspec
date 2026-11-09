@done
@tui
@rust
@agent-view
@multi-session
@input
@concurrency
@BUG-552
Feature: A submission goes to the session it was typed in
  """
  codelet-fspec-tui only. Action::InputSubmitted(SessionId, String) and Action::InputQueued(SessionId, String) carry the session (steer is InputSubmitted while that session is busy). AgentView keeps composer_session, set by App::sync_composer_draft to the BUG-549 bound session at the end of every dispatch, handle_event and handle_paste; Enter and Alt+Enter emit with it (no session: nothing is emitted, as the old handler dropped it). handle_input_submitted(session, text) and handle_input_queued(session, text) never read current_session: end_composer_draft(session), busy_enter_mode(session), send/submit to session, persistence for session. Slash commands other than the session-naming ones (ClearRole, SetRole, SetThinkingLevel, ThinkingCommand, /archive) run only when session == current_session, else a yellow notice line in the session's context. Every test that dispatched InputSubmitted(text) or InputQueued(text) now passes the session; the refac019 action snapshot records the new shapes. Out of scope (reported): slash-popup picks emit SlashCommandSelected(action), which carries no session.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A submission from the composer (Enter send, Enter steer while running, Alt+Enter queue) carries the session whose composer produced it, captured when the key is handled; the shared composer belongs to exactly one session at that moment (BUG-549 bound session)
  #   2. Every handler of a submission uses the carried session, never the focus at handling time: the text goes to that session as a new turn, a steer or a queued input (steer or queue decided by that session's own status), that session's draft is the one that ends (BUG-549 rule 2), and its history entry is recorded for it
  #   3. A focus change handled between the key and the submission never takes the submission and never ends the newly focused session's draft
  #   4. Slash commands that name their session (/role, /role clear, /thinking with a level, /archive) act on the carried session; any other slash command runs only while the carried session is still focused, otherwise it is not run and a notice in the carried session says so (conservative option, see question)
  #
  # EXAMPLES:
  #   1. s-1 is focused and s-2 holds the draft 's2 text'; a chip click focusing s-2 is already waiting when the user types 'deploy the fix' in s-1 and presses Enter: 'deploy the fix' is sent to s-1 only, the agent view shows s-2 with 's2 text', and s-2's daemon draft is still 's2 text'
  #   2. s-1 is running and focused; a new session s-2 taking focus is already waiting when the user types 'also check the logs' and presses Enter: it is steered into s-1, and nothing is sent to s-2
  #   3. s-1 is running and focused and s-2 is open; an attach to s-2 is already waiting when the user types 'next task' and presses Alt+Enter: it is queued on s-1, s-1's queued list shows 'next task' and nothing is submitted to s-2
  #   4. s-1 is focused and s-2 holds the draft 's2 text'; a chip click focusing s-2 is already waiting when the user types '/clear' in s-1 and presses Enter: s-2 is not cleared and keeps 's2 text', nothing is cleared on the daemon, and s-1 shows a notice that /clear was not run
  #   5. s-1 is focused; a chip click focusing s-2 is already waiting when the user types '/role reviewer' in s-1 and presses Enter: the role reviewer is set on s-1, not s-2
  #
  # QUESTIONS (ANSWERED):
  #   Q: when focus moves between submitting a slash command and the TUI handling it, should commands that act on the focused session or open a dialog (/clear, /compact, /model, /continue, /goal, /mux, /help ...) run against the session they were typed in, or not run? Conservative option taken: they are not run (never act on the wrong session) and a notice in the session they were typed in says so; commands that name their session (/role, /thinking with a level, /archive) act on the typed-in session.
  #   A: Conservative option taken (supervisor instruction, 2026-11-06, pending product owner review): such a command is not run when its session is no longer focused, and a notice in that session says '[notice] <command> was not run: another session was focused before it was handled'. Commands that name their session act on it. Keeps every BUG-549 rule: the typed-in session's draft still ends.
  #
  # ========================================
  Background: User Story
    As a user typing prompts in the TUI agent views
    I want to have every prompt I submit go to the session whose composer I typed it in, even if focus moves before the TUI handles it
    So that a prompt, steer, queued input or command never lands in, clears or changes a session I did not type it into

  # ─────────────────────────────────────────────────────────────────────
  # Text for the model goes to the typed-in session (rules 1-3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A prompt sent while a chip click is waiting goes to the session it was typed in
    Given session s-2 holds the draft "s2 text"
    And session s-1 is focused
    And a chip click focusing s-2 is waiting to be handled
    When the user types "deploy the fix" and presses Enter
    And the TUI handles the waiting actions
    And the draft save window has passed
    Then "deploy the fix" is sent to s-1
    And nothing is sent to s-2
    And the agent view shows s-2 with the composer holding "s2 text"
    And the in-memory draft and the daemon draft of s-2 are "s2 text"
    And the daemon draft of s-1 is empty

  Scenario: A steer submitted while a new session is taking focus goes to the running session
    Given session s-1 is focused and its turn is running
    And a new session s-2 taking focus is waiting to be handled
    When the user types "also check the logs" and presses Enter
    And the TUI handles the waiting actions
    Then "also check the logs" is submitted to s-1 as a steer
    And nothing is submitted to s-2
    And the agent view shows s-2

  Scenario: A queued input submitted while an attach is waiting is queued on the session it was typed in
    Given session s-1 is focused and its turn is running
    And session s-2 is open
    And an attach to s-2 is waiting to be handled
    When the user types "next task" and presses Alt+Enter
    And the TUI handles the waiting actions
    Then "next task" is submitted to s-1 as a queued input
    And the queued inputs of s-1 show "next task"
    And nothing is submitted to s-2

  # ─────────────────────────────────────────────────────────────────────
  # Slash commands (rule 4)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A /clear submitted while focus is moving clears no other session
    Given session s-2 holds the draft "s2 text"
    And session s-1 is focused
    And a chip click focusing s-2 is waiting to be handled
    When the user types "/clear" and presses Enter
    And the TUI handles the waiting actions
    And the draft save window has passed
    Then no session history is cleared
    And the composer holds "s2 text"
    And the daemon draft of s-2 is "s2 text"
    And s-1 shows a notice that /clear was not run because another session was focused

  Scenario: A /role submitted while focus is moving sets the role on the session it was typed in
    Given session s-1 is focused and session s-2 is open
    And a chip click focusing s-2 is waiting to be handled
    When the user types "/role reviewer" and presses Enter
    And the TUI handles the waiting actions
    Then the role "reviewer" is set on s-1
