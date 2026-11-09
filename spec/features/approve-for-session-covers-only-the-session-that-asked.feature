@done
@bug-fix
@rust
@session
@security
@BUG-606
Feature: Approve for session approves the tool in every session in the daemon
  """
  The blocklist's session allowances (tools/src/blocklist/middleware.rs SESSION_ALLOWANCES) were one daemon-wide set of rule ids, shared by the blocklist command and file prompts, the RLCD security prompt (rlcd/security.rs RLCD_CHECK_ALLOWANCE) and the RLCD workflow gate (rlcd/gate.rs 'rlcd-gate:<command>' keys), so Approve for session in any session allowed the rule in every session. Fix: the allowances are keyed by session id: allow_for_session(session_id, key), is_session_allowed(session_id, key), forget_session_allowances(session_id) (called by SessionManager::destroy_session) and clear_session_allowances() (tests); is_session_gate_allowed / allow_session_gate take the session id; every caller passes the session that asked.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Approve for session covers only the session that asked: the same approval, asked again in that session, runs without asking; every other session, including the asking session's parent, its sub-agents and unrelated sessions, is still asked; each sub-agent asks separately (product owner, 2026-11-07, decision 7)
  #   2. The scope is the same for every approval that offers Approve for session: blocklist command prompts, blocklist file prompts, the RLCD security prompt and the RLCD workflow gate
  #   3. Closing a session forgets its session approvals
  #
  # EXAMPLES:
  #   1. Session A runs 'echo bug606 git reset --hard', is asked and chooses Approve for session; A runs it again and is not asked; session B runs it and is asked
  #   2. Sub-agent S of a bot approves reading .env for the session; the bot reading .env is still asked, and so is S's sibling sub-agent
  #   3. Session A approved 'git reset --hard' for the session and is then closed; nothing in the daemon keeps A's approval
  #   4. Session A chooses Approve for session on the RLCD workflow gate for update-work-unit-status; A's next update-work-unit-status is not gated; session B's still is
  #
  # ========================================
  Background: User Story
    As a user approving a tool for one session
    I want to have Approve for session cover only the session that asked
    So that approving something for one agent never silently allows it for the bot, its other sub-agents or any other session in the daemon

  Scenario: Approve for session on a command prompt covers only the session that asked
    Given sessions "A" and "B" in one daemon, where "A" ran "echo bug606 git reset --hard", was asked for approval and chose Approve for session
    When "A" runs "echo bug606 git reset --hard" again
    Then "A" is not asked
    When "B" runs "echo bug606 git reset --hard"
    Then "B" is asked for approval

  Scenario: Approve for session on a sensitive file covers only the sub-agent that asked
    Given a bot "P" with sub-agents "S1" and "S2", where "S1" read ".env", was asked for approval and chose Approve for session
    When "P" and "S2" read ".env"
    Then each of them is asked for approval

  Scenario: Approve for session on the RLCD workflow gate covers only the session that asked
    Given sessions "A" and "B" whose update-work-unit-status is held by the RLCD workflow gate, where "A" was asked and chose Approve for session
    When "A" runs update-work-unit-status again
    Then "A" is not asked
    When "B" runs update-work-unit-status
    Then "B" is asked for approval

  Scenario: Closing a session forgets its session approvals
    Given session "A" approved "echo bug606 git reset --hard" for the session
    When "A" is closed
    Then a command run under "A"'s id is asked for approval again
