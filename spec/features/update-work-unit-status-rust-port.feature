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
  #   12. Task work units cannot move to testing (type-specific lane rule)
  #   13. Task work units skip the testing phase (specifying→implementing) and are exempt from step/coverage gates
  #   14. Story work units cannot skip testing (specifying→implementing is rejected with an ACDD hint)
  #   15. specifying→testing is hard-blocked when Example Mapping (rules+examples) is incomplete (REMIND-014 Level 1)
  #   16. Bug work units must link an existing feature file before testing; they are exempt from Level-1 review validation
  #   17. Unanswered (non-deleted, non-selected) questions block specifying→testing (BUG-060)
  #   18. Hard blockedBy dependencies that are not done block movement into active states (specifying/testing/implementing/validating)
  #   19. Soft dependsOn dependencies that are not done emit a warning but never block
  #   20. Backward transitions (e.g. validating→specifying) are allowed and record the reason; done→implementing is allowed (ACDD backward movement)
  #   21. A parent cannot be marked done while children are incomplete (COV-006)
  #   22. An optional reason is recorded in stateHistory on the transition entry
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

  Scenario: Task work unit cannot move to testing
    Given a task work unit "CLEAN-001" exists with status "specifying"
    When the dispatcher runs update-work-unit-status for "CLEAN-001" with status "testing"
    Then the command fails
    And the error message explains "Tasks do not have a testing phase"
    And the work unit status remains "specifying"

  Scenario: Task work unit skips the testing phase from specifying
    Given a task work unit "CLEAN-001" exists with status "specifying"
    When the dispatcher runs update-work-unit-status for "CLEAN-001" with status "implementing"
    Then the command succeeds
    And the work unit status becomes "implementing"

  Scenario: Task work unit skips step and coverage gates when moving to validating
    Given a task work unit "CLEAN-001" exists with status "implementing"
    And its linked feature has scenarios without test coverage mappings
    When the dispatcher runs update-work-unit-status for "CLEAN-001" with status "validating"
    Then the command succeeds
    And the work unit status becomes "validating"

  Scenario: Story work unit cannot skip the testing phase
    Given a work unit "AUTH-001" exists with status "specifying"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "implementing"
    Then the command fails
    And the error message says to move to "testing" state first
    And the error message explains "ACDD requires tests before implementation"
    And the work unit status remains "specifying"

  Scenario: specifying to testing is blocked when Example Mapping is incomplete
    Given a work unit "AUTH-001" exists with status "specifying"
    And the work unit has no rules or examples
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    Then the command fails
    And the error message reports "Cannot transition to testing - Example Mapping incomplete"

  Scenario: Bug work unit must link an existing feature file before testing
    Given a bug work unit "BUG-001" exists with status "specifying"
    And the bug has no linked feature file
    When the dispatcher runs update-work-unit-status for "BUG-001" with status "testing"
    Then the command fails
    And the error message requires linking an existing feature file

  Scenario: Bug work unit can move to testing when a feature file is linked
    Given a bug work unit "BUG-001" exists with status "specifying"
    And a feature file is tagged with "@BUG-001" and listed in linkedFeatures
    When the dispatcher runs update-work-unit-status for "BUG-001" with status "testing" and skipTemporalValidation true
    Then the command succeeds
    And the work unit status becomes "testing"

  Scenario: specifying to testing is blocked when questions are unanswered
    Given a work unit "AUTH-001" exists with status "specifying"
    And the work unit satisfies review validation
    And the work unit has an unanswered question
    And a scenario is tagged with "@AUTH-001"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing" and skipTemporalValidation true
    Then the command fails
    And the error message reports "Unanswered questions prevent state transition"

  Scenario: Starting work is blocked by incomplete hard dependencies
    Given a work unit "AUTH-001" exists with status "backlog"
    And work unit "API-001" is blocking "AUTH-001" with status "implementing"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    Then the command fails
    And the error message names "API-001" as an active blocker
    And the work unit status remains "backlog"

  Scenario: Soft dependencies produce a warning but not a block
    Given a work unit "AUTH-001" exists with status "specifying"
    And the work unit satisfies review validation
    And work unit "AUTH-002" is not done and listed in dependsOn
    And a scenario is tagged with "@AUTH-001"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing" and skipTemporalValidation true
    Then the command succeeds
    And the output includes a warning about incomplete soft dependencies

  Scenario: Backward transition to specifying records the reason
    Given a work unit "AUTH-001" exists with status "validating"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying" and reason "Acceptance criteria incomplete"
    Then the command succeeds
    And the work unit status becomes "specifying"
    And a state-history entry for "specifying" carries the reason

  Scenario: A done work unit can move backward to implementing
    Given a work unit "AUTH-001" exists with status "done"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "implementing"
    Then the command succeeds
    And the work unit status becomes "implementing"

  Scenario: Parent cannot be marked done while children are incomplete
    Given a work unit "AUTH-001" exists with status "validating"
    And work unit "AUTH-002" has parent "AUTH-001" and status "implementing"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "done"
    Then the command fails
    And the error message reports "Cannot mark parent as done while children are incomplete"

  Scenario: The reason is recorded in the state history
    Given a work unit "AUTH-001" exists with status "backlog"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying" and reason "kickoff"
    Then the command succeeds
    And the state-history entry for "specifying" carries the reason "kickoff"

  Scenario: IPC notification is a no-op in the Rust port
    Given a work unit "AUTH-001" exists with status "backlog"
    When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    Then the command succeeds
    And no IPC notification is attempted
