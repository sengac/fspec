@BUG-695
@done
@bug-fix
@rust
@tui
@security
@session
@rpc
@BUG-601
Feature: Blocklist Toggle Rule turns a rule off and on for the session
  """
  Toggle Rule used to flip a set kept only in the TUI (AgentViewStore), so the rule stayed enforced (audit AK-2). Two FspecService methods are appended (RPC-449 order; WIRE CHANGE): blocklist_disabled_rules(session_id) -> Result<Vec<String>, String> reads the rule ids turned off for a live session, and blocklist_set_rule_enabled(session_id, rule_id, enabled) -> Result<Vec<String>, String> turns one rule off or on and answers the new list. They run through SessionManagerHandle (refusing default, a stub that keeps per-session sets), FspecService and its MCP tools, FspecBackend and the embedded and WebSocket transports. codelet-sessions checks the session is live and the rule id is one blocklist_list lists, saves the set in the session manifest (SessionManifest.blocklist_disabled_rules) and hands it to codelet-tools, whose per-session store (blocklist/session_rules.rs) drops the turned-off rules before the Bash and file checks (check_bash_command, check_file_path, check_file_path_semantic) build their matcher. create_session_from_manifest (resume, also after a daemon restart) restores the set; destroy_session forgets it in memory while the manifest keeps it. The TUI keeps only the daemon's last answer per session: it reads the set when the view opens and replaces it with each toggle's answer; a failed call is shown as a session error, never as an empty or all-on view.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Toggle Rule in the blocklist view really turns the rule's enforcement off and on: a turned-off rule neither blocks nor asks, and turning it on again restores it
  #   2. The view shows the rule state the daemon enforces (read back from the daemon), never a state kept only in the TUI
  #   3. The toggle works the same over the embedded and the WebSocket transport
  #   4. A turned-off rule is off only for the session the blocklist view was opened from; every other session still enforces it (product owner, 2026-11-08)
  #   5. The rules a session turned off are saved with that session, so they stay off after a daemon restart and when the session is resumed (product owner, 2026-11-08)
  #   6. Turning a rule off or on for a session the daemon does not hold, or for a rule id the blocklist does not have, fails with an exact error that the view shows; nothing changes
  #
  # EXAMPLES:
  #   1. Session s-1 runs a command the rule git-checkout-block blocks: it is blocked; the user turns git-checkout-block off in the blocklist view; s-1 runs it again: it runs; the user turns it on: it is blocked again
  #   2. Sessions s-1 and s-2 both run a command git-checkout-block blocks; the user turns the rule off in the blocklist view opened from s-1: s-1 runs the command, s-2 is still blocked
  #   3. The user turns git-checkout-block off for s-1, the daemon restarts and s-1 is resumed: the rule is still off for s-1, its command runs, and the blocklist view shows the rule off
  #   4. The user turns rule no-such-rule off for s-1: the daemon answers 'the blocklist has no rule "no-such-rule"' and the view shows that error
  #   5. The TUI over the WebSocket transport turns git-checkout-block off for s-1 and reads back the same turned-off rules as over the embedded transport
  #
  # QUESTIONS (ANSWERED):
  #   Q: What scope and lifetime should a turned-off rule have? (a) only the session the blocklist view was opened from, kept in the per-session store BUG-606 uses for 'Approve for session' and forgotten when that session closes (matches today's per-session view, but not persisted); (b) only that session, but saved with the session so it survives a daemon restart and resume; (c) the whole project: written into the blocklist config (project rules into .fspec/blocklist.json; a system rule as a disabled id in the project file) so every session and every restart sees it. The card says 'persisted' and the brief says 'the same store BUG-606 keys per session where that applies'; these point to (b) or (c). Recommended: (b).
  #   A: Product owner, 2026-11-08 (option b): a turned-off rule is off only for the session the blocklist view was opened from, and is saved with that session, so it stays off after a daemon restart or resume; other sessions are unaffected.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to turn a blocklist rule off and on from the blocklist view
    So that the agent's tools really follow the rule state the view shows

  Scenario: Turning a rule off lets the session run what it blocked, and turning it on blocks it again
    Given session "s-1" in a daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    And "git checkout main" run under "s-1" is blocked with "Blocked: git checkout can overwrite uncommitted changes. Use git switch <branch>."
    When the user turns "git-checkout-block" off for "s-1"
    Then the daemon answers that "s-1" has turned off "git-checkout-block"
    And "git checkout main" run under "s-1" is allowed
    When the user turns "git-checkout-block" on for "s-1"
    Then the daemon answers that "s-1" has no rule turned off
    And "git checkout main" run under "s-1" is blocked again

  Scenario: A turned-off rule that asks no longer asks
    Given session "s-1" in a daemon whose blocklist asks before "git reset --hard" (rule "git-hard-reset-prompt") and before reading ".env" (rule "env-file-prompt")
    When the user turns "git-hard-reset-prompt" and "env-file-prompt" off for "s-1"
    And "s-1" runs "echo bug601 git reset --hard" and reads ".env"
    Then both are allowed and "s-1" is not asked

  Scenario: A rule turned off in one session is still enforced in every other session
    Given sessions "s-1" and "s-2" in one daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    When the user turns "git-checkout-block" off for "s-1"
    Then "git checkout main" run under "s-2" is still blocked
    And the daemon answers that "s-2" has no rule turned off

  Scenario: Turned-off rules stay off after a daemon restart and resume
    Given session "s-1" turned off "git-checkout-block"
    When the daemon restarts and "s-1" is resumed
    Then the daemon answers that "s-1" has turned off "git-checkout-block"
    And "git checkout main" run under "s-1" is allowed

  Scenario: Turning off a rule the blocklist does not have fails and changes nothing
    Given session "s-1" with no rule turned off
    When the user turns "no-such-rule" off for "s-1"
    Then the daemon answers with the error 'the blocklist has no rule "no-such-rule"'
    And the daemon answers that "s-1" has no rule turned off

  Scenario: Turning a rule off for a session the daemon does not hold fails
    Given a daemon whose blocklist has rule "git-checkout-block" and no session "00000000-0000-0000-0000-000000000601"
    When the user turns "git-checkout-block" off for "00000000-0000-0000-0000-000000000601"
    Then the daemon answers with the error "Session not found: 00000000-0000-0000-0000-000000000601"
    And reading the turned-off rules of "00000000-0000-0000-0000-000000000601" fails with the same error

  Scenario: Opening the blocklist view shows the rules the daemon has turned off for the session
    Given the daemon lists rules "git-checkout-block" and "cat-block" and has turned off "cat-block" for session "s-1"
    When the user opens "/blocklist" from "s-1"
    Then the TUI asks the daemon for the turned-off rules of "s-1"
    And the view shows "○ cat-block" with "(disabled)" and "● git-checkout-block"

  Scenario: Toggle Rule asks the daemon and shows its answer
    Given the blocklist view opened from "s-1" shows "git-checkout-block" on
    When the user presses Space on "git-checkout-block"
    Then the TUI asks the daemon to turn "git-checkout-block" off for "s-1"
    And the view shows "○ git-checkout-block"
    When the user presses Space on "git-checkout-block" again
    Then the TUI asks the daemon to turn "git-checkout-block" on for "s-1"
    And the view shows "● git-checkout-block"

  Scenario: A refused toggle shows the daemon's error and keeps the rule's state
    Given the blocklist view opened from "s-1" shows "git-checkout-block" on, and the daemon refuses toggles with 'the blocklist has no rule "git-checkout-block"'
    When the user presses Space on "git-checkout-block"
    Then the TUI shows the error 'Toggle Rule failed: the blocklist has no rule "git-checkout-block"'
    And the view shows "● git-checkout-block"

  Scenario: A failed read shows the error instead of an empty or all-on view
    Given the daemon refuses to list the blocklist rules with "cannot load the project blocklist" and to read the turned-off rules of "s-1" with "Session not found: s-1"
    When the user opens "/blocklist" from "s-1"
    Then the TUI shows the errors "Blocklist rules could not be loaded: cannot load the project blocklist" and "Turned-off blocklist rules could not be loaded: Session not found: s-1"

  Scenario: Toggle Rule without a session shows an error
    Given the blocklist view is open while no session is open
    When the user presses Space on "git-checkout-block"
    Then the TUI shows the error "Toggle Rule failed: open /blocklist from a session to turn its rules off or on"
    And the daemon is not asked to change any rule

  Scenario: The embedded and WebSocket transports turn a rule off and read it back the same way
    Given a daemon listing rule "git-checkout-block" behind an embedded and a WebSocket backend
    When the WebSocket backend turns "git-checkout-block" off for "s-1"
    Then the embedded backend reads "git-checkout-block" as turned off for "s-1"
    When the embedded backend turns "git-checkout-block" on for "s-1"
    Then the WebSocket backend reads no rule turned off for "s-1"

  Scenario: A rule turned off through the daemon is enforced only in that session and stays off after a daemon restart
    Given sessions "A" and "B" in a daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    When the user turns "git-checkout-block" off for "A" through the daemon's RPC
    Then "git checkout main" run under "A" is allowed
    And "git checkout main" run under "B" is still blocked
    When the daemon restarts and "A" is resumed through the daemon's RPC
    Then the daemon's RPC answers that "A" has turned off "git-checkout-block"
    And "git checkout main" run under "A" is allowed
