@rust
@work-unit-management
@cli
@wip
@RPC-319
Feature: Port update-work-unit-status command to Rust

  """
  RPC-319 is the largest port (1404 LOC). Rust impl reuses: codelet_git ghost_commit checkpoint primitives (RPC-202/203/288), the ported hooks executor, configure-tools test/quality command checks, compact-work-unit, and io::gherkin for scenario/prefill detection. IPC is a no-op. Must stay synchronous (poll_sync_future) — use blocking std + git2/gitoxide, no real tokio .await. Files: core commands/update_work_unit_status.rs (rewrite stub to 2-arg), bridge fspec/src/update_work_unit_status.rs, help config, cli + dispatcher tests. Supervisor wires canonical/dispatch/main/mod.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. State transitions are constrained by STATE_TRANSITIONS map: backlog→[specifying,blocked]; specifying→[testing,blocked]; testing→[implementing,specifying,blocked]; implementing→[validating,testing,specifying,blocked]; validating→[done,implementing,testing,specifying,blocked]; done→[specifying,testing,implementing,validating,blocked]; blocked→[backlog,specifying,testing,implementing,validating].
  #   2. Moving to blocked requires a blockedReason; an unknown workUnitId errors with 'Work unit <id> does not exist'; status must be one of the 7 ALLOWED_STATES.
  #   3. Forward transitions enforce validation gates: prefill-detection on the linked feature; review-validation; specifying→testing requires scenarios exist; temporal-validation (checkFileCreatedAfter / findStateHistoryEntry) unless skipTemporalValidation; test docstring/step validation; coverage-completeness on testing/implementing→validating.
  #   4. Side-effects on transition: create automatic git checkpoint before transition when working dir dirty (skip on backlog); on →done compact the work unit and cleanup auto-checkpoints (preserve manual); emit consolidated system-reminders (status-change, virtual-hooks, cleanup); execute pre/post hooks; IPC notify TUI. IPC is a NO-OP in the Rust port (per Batch 18).
  #
  # EXAMPLES:
  #   1. Valid forward transition backlog→specifying succeeds and records a state-history entry
  #   2. Invalid transition backlog→done is rejected with an allowed-transitions error message
  #   3. Unknown work unit id errors with 'Work unit <id> does not exist'
  #   4. Moving to blocked without a blockedReason errors; supplying blockedReason succeeds
  #   5. specifying→testing is blocked when the linked feature contains prefill placeholders
  #   6. specifying→testing is blocked when the linked feature has no scenarios
  #   7. testing/implementing→validating is blocked when scenario coverage is incomplete
  #   8. A forward transition is blocked by temporal validation unless --skipTemporalValidation is passed
  #   9. A dirty working directory triggers an automatic git checkpoint before the transition (skipped for backlog)
  #   10. Transitioning to done compacts the work unit and cleans auto-checkpoints while preserving manual ones
  #   11. Pre/post hooks run around the transition; a blocking pre-hook failure prevents the transition
  #
  # ========================================

  Background: User Story
    As a fspec maintainer / AI agent
    I want to transition a work unit between ACDD states via the ported Rust update-work-unit-status command
    So that the Rust CLI and LLM dispatcher enforce the same state machine, validation gates, auto-checkpoints and system-reminders as the TypeScript original

  Scenario: CLI applies a valid transition and exits zero
    Given a work unit "AUTH-001" exists with status "backlog"
    When I run `fspec update-work-unit-status AUTH-001 specifying`
    Then the command exits with code 0
    And stdout confirms the status changed to "specifying"

  Scenario: CLI rejects an invalid transition with a non-zero exit code
    Given a work unit "AUTH-001" exists with status "backlog"
    When I run `fspec update-work-unit-status AUTH-001 done`
    Then the command exits with a non-zero code
    And stderr names the allowed transitions from "backlog"

  Scenario: CLI rejects an unknown work unit id
    Given no work unit "NOPE-999" exists
    When I run `fspec update-work-unit-status NOPE-999 specifying`
    Then the command exits with a non-zero code
    And stderr contains "Work unit NOPE-999 does not exist"

  Scenario: CLI requires --blocked-reason when moving to blocked
    Given a work unit "AUTH-001" exists with status "specifying"
    When I run `fspec update-work-unit-status AUTH-001 blocked`
    Then the command exits with a non-zero code
    And stderr requires a blocked reason
    When I run `fspec update-work-unit-status AUTH-001 blocked --blocked-reason "waiting on API"`
    Then the command exits with code 0
    And stdout confirms the status changed to "blocked"

  Scenario: CLI rejects an unknown status value
    Given a work unit "AUTH-001" exists with status "backlog"
    When I run `fspec update-work-unit-status AUTH-001 frobnicate`
    Then the command exits with a non-zero code
    And stderr reports that the status must be one of the allowed states

  Scenario: CLI honours --skip-temporal-validation
    Given a work unit "AUTH-001" exists with status "specifying"
    And its linked feature file was last modified before the work unit entered "specifying"
    When I run `fspec update-work-unit-status AUTH-001 testing`
    Then the command exits with a non-zero code
    When I run `fspec update-work-unit-status AUTH-001 testing --skip-temporal-validation`
    Then the command exits with code 0

  Scenario: CLI surfaces blocking hook failure on stderr
    Given a work unit "AUTH-001" exists with status "specifying"
    And a blocking pre-transition hook is configured to fail
    When I run `fspec update-work-unit-status AUTH-001 testing`
    Then the command exits with a non-zero code
    And the blocking hook stderr is wrapped in a system-reminder

  Scenario: CLI help text matches the byte-for-byte fixture
    When I run `fspec update-work-unit-status --help`
    Then the command exits with code 0
    And stdout matches the help fixture exactly
