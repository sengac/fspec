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

  Scenario: Valid forward transition records a state-history entry
    Given a work unit "AUTH-001" exists with status "backlog"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    Then the command succeeds
    And the work unit status becomes "specifying"
    And a state-history entry for "specifying" is recorded with a timestamp

  Scenario: Invalid transition is rejected with an allowed-transitions message
    Given a work unit "AUTH-001" exists with status "backlog"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "done"
    Then the command fails
    And the error message names the allowed transitions from "backlog"
    And the work unit status remains "backlog"

  Scenario: Unknown work unit id is rejected
    Given no work unit "NOPE-999" exists
    When the dispatcher runs update-work-unit-status for "NOPE-999" with status "specifying"
    Then the command fails
    And the error message is "Work unit NOPE-999 does not exist"

  Scenario: Moving to blocked requires a blockedReason
    Given a work unit "AUTH-001" exists with status "specifying"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "blocked" and no blockedReason
    Then the command fails
    And the error message requires a blockedReason
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "blocked" and blockedReason "waiting on API"
    Then the command succeeds
    And the work unit status becomes "blocked"

  Scenario: specifying to testing is blocked by prefill placeholders
    Given a work unit "AUTH-001" exists with status "specifying"
    And its linked feature file contains prefill placeholders
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then the command fails
    And the error message reports the prefill placeholders that must be resolved
    And the work unit status remains "specifying"

  Scenario: specifying to testing is blocked when the feature has no scenarios
    Given a work unit "AUTH-001" exists with status "specifying"
    And its linked feature file has no scenarios
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then the command fails
    And the error message reports that scenarios are required before testing
    And the work unit status remains "specifying"

  Scenario: validating transition is blocked when coverage is incomplete
    Given a work unit "AUTH-001" exists with status "implementing"
    And its linked feature has scenarios without test coverage mappings
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "validating"
    Then the command fails
    And the error message reports the uncovered scenarios
    And the work unit status remains "implementing"

  Scenario: Temporal validation blocks a forward transition unless skipped
    Given a work unit "AUTH-001" exists with status "specifying"
    And its linked feature file was last modified before the work unit entered "specifying"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then the command fails
    And the error message reports a temporal-ordering violation
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing" and skipTemporalValidation true
    Then the command succeeds
    And the work unit status becomes "testing"

  Scenario: A dirty working directory creates an automatic checkpoint before the transition
    Given a work unit "AUTH-001" exists with status "specifying"
    And the git working directory has uncommitted changes
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then an automatic git checkpoint is created before the transition is applied
    And the command succeeds

  Scenario: Transitioning to backlog does not create an automatic checkpoint
    Given a work unit "AUTH-001" exists with status "blocked"
    And the git working directory has uncommitted changes
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "backlog"
    Then no automatic git checkpoint is created
    And the command succeeds

  Scenario: Transitioning to done compacts the work unit and cleans auto-checkpoints
    Given a work unit "AUTH-001" exists with status "validating"
    And the work unit has both automatic and manual checkpoints
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "done"
    Then the command succeeds
    And the work unit is compacted
    And automatic checkpoints are removed while manual checkpoints are preserved
    And a consolidated status-change system-reminder is emitted

  Scenario: A blocking pre-hook failure prevents the transition
    Given a work unit "AUTH-001" exists with status "specifying"
    And a blocking pre-transition hook is configured to fail
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then the command fails
    And the work unit status remains "specifying"
    And the blocking hook stderr is surfaced in a system-reminder

  Scenario: IPC notification is a no-op in the Rust port
    Given a work unit "AUTH-001" exists with status "backlog"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    Then the command succeeds
    And no IPC notification is attempted
