@BOARD-001
@cli
@project-management
@workflow
@kanban
Feature: Kanban Workflow State Management
  """
  Architecture notes:
  - Work units flow through ACDD-aligned states: backlog → specifying → testing → implementing → validating → done
  - Additional state: blocked (can occur from any state when progress is prevented)
  - State transitions are enforced to maintain ACDD methodology integrity
  - State history is tracked for metrics and auditing
  - States index in work-units.json maintains work unit IDs by state for fast queries
  - Priority is determined by position in state array (first = highest priority)

  Workflow states and their purpose:
  - backlog: Work exists but not started
  - specifying: Example mapping and writing Gherkin scenarios (discovery phase)
  - testing: Writing tests that map to scenarios BEFORE implementation
  - implementing: Writing code to make tests pass
  - validating: Running lifecycle hooks, build, all quality checks
  - done: All validations passed, work complete
  - blocked: Cannot progress due to dependency, question, or external blocker

  Critical implementation requirements:
  - MUST enforce valid state transitions (no skipping steps in ACDD workflow)
  - MUST update states index when work unit state changes
  - MUST record state transition in stateHistory with timestamps
  - MUST update work unit updatedAt timestamp on state change
  - MUST validate prerequisites for state transitions (e.g., scenarios exist before testing)
  - MUST prevent parent from being done while children incomplete
  - MUST support priority reordering within state arrays

  State transition rules:
  Valid: backlog → specifying, specifying → testing, testing → implementing, implementing → validating,
  validating → done, any → blocked, blocked → [previous state]
  Invalid: backlog → implementing (must go through specifying and testing first)
  any → backlog (work doesn't return to backlog)

  Data model:
  - work-units.json: Contains workUnits object and states object
  - states object: Arrays of work unit IDs keyed by state name
  - stateHistory: Array of {state, timestamp, reason?} objects in each work unit

  References:
  - Project Management Design: project-management.md (section 6: Workflow States)
  - ACDD Methodology: spec/CLAUDE.md
  """

  Background: User Story
    As an AI agent practicing ACDD
    I want work units to flow through structured workflow states
    So that I follow the correct sequence of specification → testing → implementation

  @critical
  @happy-path
  Scenario: Move work unit from backlog to specifying
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "backlog"
    When I run "fspec update-work-unit AUTH-001 --status=specifying"
    Then the command should succeed
    And the work unit "AUTH-001" status should be "specifying"
    And the states.backlog array should not contain "AUTH-001"
    And the states.specifying array should contain "AUTH-001"
    And the stateHistory should include transition from "backlog" to "specifying"
    And the updatedAt timestamp should be updated

  @happy-path
  Scenario: Complete ACDD workflow from backlog to done
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "backlog"
    When I run "fspec update-work-unit AUTH-001 --status=specifying"
    And I run "fspec update-work-unit AUTH-001 --status=testing"
    And I run "fspec update-work-unit AUTH-001 --status=implementing"
    And I run "fspec update-work-unit AUTH-001 --status=validating"
    And I run "fspec update-work-unit AUTH-001 --status=done"
    Then the work unit status should be "done"
    And the states.done array should contain "AUTH-001"
    And the stateHistory should have 6 entries
    And the stateHistory should show progression: backlog → specifying → testing → implementing → validating → done

  @validation
  @error-handling
  Scenario: Attempt to skip specifying state (violates ACDD)
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "backlog"
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should fail
    And the error should contain "Invalid state transition"
    And the error should contain "Must move to 'specifying' state first"
    And the error should explain "ACDD requires specification before testing"

  @validation
  @error-handling
  Scenario: Attempt to skip testing state (violates ACDD)
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    When I run "fspec update-work-unit AUTH-001 --status=implementing"
    Then the command should fail
    And the error should contain "Invalid state transition"
    And the error should contain "Must move to 'testing' state first"
    And the error should explain "ACDD requires tests before implementation"

  @validation
  @error-handling
  Scenario: Attempt to move work back to backlog
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec update-work-unit AUTH-001 --status=backlog"
    Then the command should fail
    And the error should contain "Cannot move work back to backlog"
    And the error should suggest "Use 'blocked' state if work cannot progress"

  @blocked-state
  @happy-path
  Scenario: Move work unit to blocked state from any state
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec update-work-unit AUTH-001 --status=blocked --blocked-reason='Waiting for API endpoint'"
    Then the command should succeed
    And the work unit status should be "blocked"
    And the states.implementing array should not contain "AUTH-001"
    And the states.blocked array should contain "AUTH-001"
    And the work unit should have blockedReason "Waiting for API endpoint"
    And the stateHistory should record the blocked transition

  @blocked-state
  @happy-path
  Scenario: Unblock work unit and return to previous state
    Given I have a project with spec directory
    And a work unit "AUTH-001" has stateHistory:
      | state      | timestamp            |
      | backlog    | 2025-01-15T10:00:00Z |
      | specifying | 2025-01-15T11:00:00Z |
      | blocked    | 2025-01-15T12:00:00Z |
    When I run "fspec update-work-unit AUTH-001 --status=specifying"
    Then the command should succeed
    And the work unit status should be "specifying"
    And the work unit blockedReason should be cleared
    And the states.blocked array should not contain "AUTH-001"
    And the states.specifying array should contain "AUTH-001"

  @blocked-state
  @validation
  Scenario: Require blocked reason when moving to blocked state
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec update-work-unit AUTH-001 --status=blocked"
    Then the command should fail
    And the error should contain "Blocked reason is required"
    And the error should suggest "Use --blocked-reason='description of blocker'"

  @validation
  @prerequisites
  Scenario: Validate Gherkin scenarios exist before moving to testing
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And no scenarios are tagged with "@AUTH-001"
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should fail
    And the error should contain "No Gherkin scenarios found"
    And the error should contain "At least one scenario must be tagged with @AUTH-001"
    And the error should suggest "Use 'fspec generate-scenarios AUTH-001' or manually tag scenarios"

  @validation
  @prerequisites
  Scenario: Successfully move to testing when scenarios exist
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And a scenario is tagged with "@AUTH-001" in spec/features/authentication.feature
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should succeed
    And the work unit status should be "testing"

  @validation
  @prerequisites
  Scenario: Validate estimate assigned before moving from specifying
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "specifying"
    And the work unit has no estimate
    And a scenario is tagged with "@AUTH-001"
    When I run "fspec update-work-unit AUTH-001 --status=testing"
    Then the command should display warning "No estimate assigned"
    And the command should suggest "Consider adding estimate with --estimate=<points>"
    But the transition should succeed

  @validation
  @parent-child
  Scenario: Prevent parent from being marked done with incomplete children
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "validating"
    And a work unit "AUTH-002" exists with parent "AUTH-001" and status "implementing"
    When I run "fspec update-work-unit AUTH-001 --status=done"
    Then the command should fail
    And the error should contain "Cannot mark parent as done"
    And the error should list incomplete child "AUTH-002" with status "implementing"
    And the error should suggest "Complete all children first"

  @validation
  @parent-child
  Scenario: Allow parent to be marked done when all children complete
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "validating"
    And a work unit "AUTH-002" exists with parent "AUTH-001" and status "done"
    And a work unit "AUTH-003" exists with parent "AUTH-001" and status "done"
    When I run "fspec update-work-unit AUTH-001 --status=done"
    Then the command should succeed
    And the work unit "AUTH-001" status should be "done"

  @priority
  @happy-path
  Scenario: Reorder work unit to top of backlog (highest priority)
    Given I have a project with spec directory
    And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
    When I run "fspec prioritize AUTH-003 --position=top"
    Then the command should succeed
    And the states.backlog array should contain in order: "AUTH-003", "AUTH-001", "AUTH-002"

  @priority
  @happy-path
  Scenario: Reorder work unit to bottom of backlog (lowest priority)
    Given I have a project with spec directory
    And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
    When I run "fspec prioritize AUTH-001 --position=bottom"
    Then the command should succeed
    And the states.backlog array should contain in order: "AUTH-002", "AUTH-003", "AUTH-001"

  @priority
  @happy-path
  Scenario: Move work unit before another work unit
    Given I have a project with spec directory
    And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
    When I run "fspec prioritize AUTH-003 --before=AUTH-002"
    Then the command should succeed
    And the states.backlog array should contain in order: "AUTH-001", "AUTH-003", "AUTH-002"

  @priority
  @happy-path
  Scenario: Move work unit after another work unit
    Given I have a project with spec directory
    And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
    When I run "fspec prioritize AUTH-001 --after=AUTH-003"
    Then the command should succeed
    And the states.backlog array should contain in order: "AUTH-002", "AUTH-003", "AUTH-001"

  @priority
  @happy-path
  Scenario: Set work unit to specific position in backlog
    Given I have a project with spec directory
    And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003", "AUTH-004"
    When I run "fspec prioritize AUTH-004 --position=1"
    Then the command should succeed
    And the states.backlog array should contain in order: "AUTH-001", "AUTH-004", "AUTH-002", "AUTH-003"

  @priority
  @validation
  Scenario: Attempt to prioritize work not in backlog
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec prioritize AUTH-001 --position=top"
    Then the command should fail
    And the error should contain "Can only prioritize work units in backlog state"
    And the error should contain "AUTH-001 is in 'implementing' state"

  @priority
  @validation
  Scenario: Attempt to position work unit before non-existent work unit
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "backlog"
    And no work unit "AUTH-999" exists
    When I run "fspec prioritize AUTH-001 --before=AUTH-999"
    Then the command should fail
    And the error should contain "Work unit 'AUTH-999' does not exist"

  @state-history
  @happy-path
  Scenario: Track complete state history with timestamps
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "backlog"
    And the current time is "2025-01-15T10:00:00Z"
    When I run "fspec update-work-unit AUTH-001 --status=specifying" at "2025-01-15T11:00:00Z"
    And I run "fspec update-work-unit AUTH-001 --status=blocked --blocked-reason='Question'" at "2025-01-15T12:00:00Z"
    And I run "fspec update-work-unit AUTH-001 --status=specifying" at "2025-01-15T14:00:00Z"
    And I run "fspec update-work-unit AUTH-001 --status=testing" at "2025-01-15T15:00:00Z"
    Then the stateHistory should have 5 entries
    And the stateHistory should be:
      | state      | timestamp            | reason   |
      | backlog    | 2025-01-15T10:00:00Z |          |
      | specifying | 2025-01-15T11:00:00Z |          |
      | blocked    | 2025-01-15T12:00:00Z | Question |
      | specifying | 2025-01-15T14:00:00Z |          |
      | testing    | 2025-01-15T15:00:00Z |          |

  @state-history
  @query
  Scenario: Calculate time spent in each state from history
    Given I have a project with spec directory
    And a work unit "AUTH-001" has stateHistory:
      | state        | timestamp            |
      | backlog      | 2025-01-15T10:00:00Z |
      | specifying   | 2025-01-15T11:00:00Z |
      | testing      | 2025-01-15T13:00:00Z |
      | implementing | 2025-01-15T14:00:00Z |
      | validating   | 2025-01-15T17:00:00Z |
      | done         | 2025-01-15T18:00:00Z |
    When I run "fspec query work-unit AUTH-001 --show-cycle-time"
    Then the output should show:
      | state        | duration |
      | backlog      | 1 hour   |
      | specifying   | 2 hours  |
      | testing      | 1 hour   |
      | implementing | 3 hours  |
      | validating   | 1 hour   |
    And the total cycle time should be "8 hours"

  @validation
  @state-enforcement
  Scenario: Allow validation to move back to implementing on test failure
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "validating"
    When I run "fspec update-work-unit AUTH-001 --status=implementing --reason='Test failures'"
    Then the command should succeed
    And the work unit status should be "implementing"
    And the stateHistory should record the reason "Test failures"

  @validation
  @state-enforcement
  Scenario: Allow validation to move back to specifying on spec error
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "validating"
    When I run "fspec update-work-unit AUTH-001 --status=specifying --reason='Acceptance criteria incomplete'"
    Then the command should succeed
    And the work unit status should be "specifying"

  @validation
  @state-enforcement
  @acdd
  Scenario: Allow moving from done to fix mistakes (ACDD backward movement)
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "done"
    When I run "fspec update-work-unit AUTH-001 --status=implementing"
    Then the command should succeed
    And the work unit status should be "implementing"
    And the stateHistory should include the backward transition
    And the work unit should be moved from states.done to states.implementing

  @query
  @filtering
  Scenario: Query work units by current state
    Given I have a project with spec directory
    And work units exist:
      | id       | status       |
      | AUTH-001 | backlog      |
      | AUTH-002 | specifying   |
      | AUTH-003 | implementing |
      | DASH-001 | implementing |
      | API-001  | done         |
    When I run "fspec query work-units --status=implementing --output=json"
    Then the output should be valid JSON
    And the JSON should contain 2 work units
    And the JSON should include "AUTH-003" and "DASH-001"

  @query
  @board-view
  Scenario: Display Kanban board showing all states
    Given I have a project with spec directory
    And work units exist:
      | id       | status       | estimate |
      | AUTH-001 | backlog      | 5        |
      | AUTH-002 | specifying   | 8        |
      | AUTH-003 | testing      | 3        |
      | DASH-001 | implementing | 5        |
      | API-001  | validating   | 5        |
      | SEC-001  | done         | 3        |
    When I run "fspec board"
    Then the output should display columns for all states
    And the backlog column should show "AUTH-001 [5 pts]"
    And the specifying column should show "AUTH-002 [8 pts]"
    And the testing column should show "AUTH-003 [3 pts]"
    And the implementing column should show "DASH-001 [5 pts]"
    And the validating column should show "API-001 [5 pts]"
    And the done column should show "SEC-001 [3 pts]"
    And the summary should show "26 points in progress" and "3 points completed"

  @query
  @board-view
  Scenario: Limit displayed work units per column with --limit option
    Given I have a project with spec directory
    And work units exist in backlog:
      | id       | estimate |
      | AUTH-001 | 5        |
      | AUTH-002 | 3        |
      | AUTH-003 | 8        |
      | AUTH-004 | 5        |
      | AUTH-005 | 2        |
    When I run "fspec board --limit=2"
    Then the backlog column should show first 2 work units
    And the backlog column should show "AUTH-001 [5 pts]"
    And the backlog column should show "AUTH-002 [3 pts]"
    And the backlog column should show "... 3 more"
    And the other empty columns should not show overflow indicators

  @query
  @board-view
  @json-output
  Scenario: Export Kanban board as JSON for programmatic access
    Given I have a project with spec directory
    And work units exist:
      | id       | status       | estimate |
      | AUTH-001 | backlog      | 5        |
      | AUTH-002 | specifying   | 8        |
      | DASH-001 | implementing | 5        |
    When I run "fspec board --format=json"
    Then the output should be valid JSON
    And the JSON should have a "columns" object
    And the JSON should have a "board" object
    And the JSON should have a "summary" string
    And the columns.backlog array should contain work unit with id "AUTH-001"
    And the columns.specifying array should contain work unit with id "AUTH-002"
    And the board.backlog array should contain "AUTH-001"
    And the board.specifying array should contain "AUTH-002"

  @validation
  @json-schema
  Scenario: Validate work unit state is one of allowed values
    Given I have a project with spec directory
    And I attempt to manually edit work-units.json
    And I set work unit "AUTH-001" status to "invalid-state"
    When I run "fspec validate-work-units"
    Then the command should fail
    And the error should contain "Invalid status value"
    And the error should list allowed values: backlog, specifying, testing, implementing, validating, done, blocked

  @consistency
  @validation
  Scenario: Detect work unit in wrong state array
    Given I have a project with spec directory
    And work unit "AUTH-001" has status "implementing"
    But states.backlog array contains "AUTH-001"
    And states.implementing array does not contain "AUTH-001"
    When I run "fspec validate-work-units"
    Then the command should fail
    And the error should contain "State consistency error"
    And the error should contain "AUTH-001 has status 'implementing' but is in 'backlog' array"
    And the error should suggest "Run 'fspec repair-work-units' to fix inconsistencies"

  @consistency
  @repair
  Scenario: Repair work units state index inconsistencies
    Given I have a project with spec directory
    And work unit "AUTH-001" has status "implementing"
    But states.backlog array contains "AUTH-001"
    And states.implementing does not contain "AUTH-001"
    When I run "fspec repair-work-units"
    Then the command should succeed
    And the command should report "Moved AUTH-001 from backlog to implementing"
    And the states.implementing array should contain "AUTH-001"
    And the states.backlog array should not contain "AUTH-001"
