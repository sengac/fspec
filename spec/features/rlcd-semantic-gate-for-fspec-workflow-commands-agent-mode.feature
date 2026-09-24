@done
@rlcd-003
@RLCD-003
@rlcd
@tools
Feature: RLCD semantic gate for fspec workflow commands (agent-mode)

  """
  The RLCD engine (current backend: laya-rs) semantically reviews gated
  fspec workflow commands issued by the agent (agent-mode tool path only,
  at the FspecToolFacadeWrapper boundary, keyed by session_id) BEFORE
  execution. One choice question per gated command, criteria
  {proceed, hold, ask_user}; thresholds under rlcd.gate (holdThreshold
  0.65, askThreshold 0.40, proceedMargin 0.15, all config). Outcome:
  proceed => execute via the normal fspec handler path; hold => reject
  with an 'RLCD gate:' error (handler NOT called); ask_user => Triple
  pause (AllowOnce / AllowSession / Deny). Session allowance
  'rlcd-gate:<command>' suppresses further prompts until TUI restart.
  FAIL-OPEN (HITL decision 2026-09-24): unreachable engine or
  rlcd.enabled=false => execute anyway + warn log; the gate never
  hard-blocks and never mutates fspec state itself. CLI (fspec-core)
  invocations have no session context and remain ungated.
  """

  Background: User Story
    As a user of the ACDD workflow
    I want to have risky fspec workflow commands (e.g. work-unit stage transitions, deletions) semantically reviewed by the RLCD engine before execution
    So that bad-timing or destructive workflow commands are caught (or surfaced to me) without a full human approval on every command

  # R1: agent-mode boundary; R2: config gate list; R4: one choice question
  Scenario: A confident proceed answer executes the command normally
    Given a gated command "update-work-unit-status" for work unit "AUTH-001" from "implementing" to "validating"
    And a reachable RLCD engine answering the gate question with P(proceed)=0.90, P(hold)=0.05, P(ask_user)=0.05
    When the agent issues the command
    Then the command executes through the normal fspec handler path
    And the agent receives the unchanged success result
    And no pause prompt was shown to the user

  # R4, R5(b): hold rejects BEFORE execution
  Scenario: A hold answer rejects the command before execution
    Given a gated command "update-work-unit-status" for work unit "AUTH-002" from "implementing" to "done"
    And a reachable RLCD engine answering the gate question with P(hold)=0.80, P(proceed)=0.05, P(ask_user)=0.15
    When the agent issues the command
    Then the command is rejected with an error starting "RLCD gate:"
    And the fspec handler is NOT called (the work unit's status is unchanged)
    And no pause prompt was shown to the user

  # R5(c): ask_user -> Triple pause; Deny rejects
  Scenario: An ask_user answer with a Deny response rejects the command
    Given a gated command "update-work-unit-status" for work unit "AUTH-003" from "testing" to "implementing"
    And a reachable RLCD engine answering the gate question with P(ask_user)=0.55, P(hold)=0.20, P(proceed)=0.25
    When the agent issues the command
    Then a Triple pause prompt appears showing the command and the engine's rationale
    And the user chooses Deny
    Then the command is rejected with an error containing "RLCD gate: user denied"
    And the fspec handler is NOT called

  # R5(c): AllowOnce executes once, prompts again next time
  Scenario: An ask_user answer with AllowOnce executes once and prompts again
    Given a reachable RLCD engine answering the gate question with P(ask_user)=0.50, P(hold)=0.10, P(proceed)=0.40
    When the agent issues the gated command "update-work-unit-status" and the user chooses Allow Once
    Then the command executes through the normal fspec handler path
    When the agent issues the same gated command again
    Then a Triple pause prompt appears again

  # R5(c), R6: AllowSession records the allowance and suppresses re-prompts
  Scenario: An ask_user answer with AllowSession suppresses later prompts
    Given a reachable RLCD engine answering the gate question with P(ask_user)=0.50, P(hold)=0.10, P(proceed)=0.40
    When the agent issues the gated command "update-work-unit-status" and the user chooses Allow Session
    Then the command executes through the normal fspec handler path
    When the agent issues the same gated command again
    Then the command executes without another prompt and without any RLCD call

  # R7: fail-open on unreachable engine
  Scenario: An unreachable engine fails open and executes the command
    Given no reachable RLCD engine anywhere
    When the agent issues the gated command "update-work-unit-status"
    Then the command executes through the normal fspec handler path
    And the agent receives the unchanged success result

  # R2: commands not in the gate list bypass the gate entirely
  Scenario: A command not in the gate list passes through without RLCD involvement
    Given a reachable RLCD engine
    When the agent issues the command "list-work-units"
    Then the command executes through the normal fspec handler path
    And no RLCD gate question was asked

  # R2: gate list is config-extensible without code changes
  Scenario: A command added to the gate list via config is also gated
    Given the user config lists rlcd.gate.commands as ["update-work-unit-status", "delete-work-unit"]
    And a reachable RLCD engine answering the gate question with P(hold)=0.90, P(proceed)=0.05, P(ask_user)=0.05
    When the agent issues the gated command "delete-work-unit"
    Then the command is rejected with an error starting "RLCD gate:"
    And the fspec handler is NOT called

  # R3: state includes the work unit's title/epic when available
  Scenario: The gate state includes the work unit's title and epic when available
    Given a project whose spec/work-units.json defines work unit "AUTH-001" with title "User Login" and epic "authentication"
    And a reachable RLCD engine
    When the agent issues the gated command "update-work-unit-status" for work unit "AUTH-001"
    Then the RLCD request state mentions "AUTH-001", the status transition, "User Login" and "authentication"

  # R3: missing context degrades to a raw state, never fails
  Scenario: A missing work-units.json degrades to a raw command state without failing
    Given a project without a spec/work-units.json file
    And a reachable RLCD engine
    When the agent issues the gated command "update-work-unit-status" for work unit "GHOST-999"
    Then the gate proceeds with a state naming the command and its arguments (no title, no epic)
    And the command still goes through the RLCD review

  # R7: disabled RLCD bypasses the gate
  Scenario: A disabled RLCD config bypasses the gate entirely
    Given the user config has rlcd.enabled = false
    And a reachable RLCD engine
    When the agent issues the gated command "update-work-unit-status"
    Then the command executes through the normal fspec handler path
    And no RLCD gate question was asked

  # R5(a): thin-margin proceed is treated conservatively as ask_user
  Scenario: A low-confidence proceed answer asks the user instead of proceeding
    Given a reachable RLCD engine answering the gate question with P(proceed)=0.50, P(ask_user)=0.38, P(hold)=0.12
    When the agent issues the gated command "update-work-unit-status"
    Then a Triple pause prompt appears (the proceed margin 0.12 is below proceedMargin 0.15)
    And the fspec handler is NOT called until the user answers

  # R5(b): P(hold) at/above holdThreshold rejects even when another criterion is argmax
  Scenario: A hold probability at or above holdThreshold rejects the command
    Given a reachable RLCD engine answering the gate question with P(ask_user)=0.80, P(hold)=0.70, P(proceed)=0.50
    When the agent issues the gated command "update-work-unit-status"
    Then the command is rejected with an error starting "RLCD gate:" (P(hold)=0.70 meets the default holdThreshold of 0.65)
    And the fspec handler is NOT called

  # R5: thresholds are read from the user config
  Scenario: Configured thresholds change the gate's decision
    Given the user config has rlcd.gate.holdThreshold = 0.95
    And a reachable RLCD engine answering the gate question with P(ask_user)=0.80, P(hold)=0.70, P(proceed)=0.50
    When the agent issues the gated command "update-work-unit-status"
    Then a Triple pause prompt appears (P(hold)=0.70 is below the configured holdThreshold of 0.95 and argmax is ask_user)
    And the fspec handler is NOT called until the user answers
