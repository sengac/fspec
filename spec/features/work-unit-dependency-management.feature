@wip
@phase7
@cli
@project-management
@dependencies
@workflow
Feature: Work Unit Dependency Management
  """
  Architecture notes:
  - Work units can have four types of relationships: blocks, blockedBy, dependsOn, relatesTo
  - blocks: This work blocks other work from starting (stronger than dependsOn)
  - blockedBy: This work cannot start until blocker is done (triggers blocked state)
  - dependsOn: This work should wait for dependency to complete (soft dependency)
  - relatesTo: Related work for context (informational only)
  - Bidirectional relationships are automatically maintained
  - Circular dependencies must be detected and prevented
  - Dependency chains affect workflow and priority

  Critical implementation requirements:
  - MUST maintain bidirectional consistency (if A blocks B, then B blockedBy A)
  - MUST detect circular dependencies before creating relationships
  - MUST prevent deletion of work units that block other work
  - MUST auto-transition to blocked state when blockedBy work exists
  - MUST support dependency visualization (graph/tree view)
  - MUST validate dependency relationships during state transitions
  - MUST track dependency chains for impact analysis

  Relationship semantics:
  - blocks: Creates hard blocker, prevents dependent from starting
  - blockedBy: Cannot progress until blocker completes (auto-sets blocked state)
  - dependsOn: Soft dependency, work can start but should wait
  - relatesTo: Informational relationship, no blocking behavior

  Data model:
  - work-units.json: Each work unit has optional arrays: blocks, blockedBy, dependsOn, relatesTo
  - All relationship IDs must reference valid work units
  - Bidirectional relationships stored in both work units

  References:
  - Project Management Design: project-management.md (section 10: Dependencies)
  - Dependency graphs enable critical path analysis
  """

  Background: User Story
    As an AI agent managing complex work
    I want to define and track dependencies between work units
    So that I can understand relationships, blockers, and work sequencing

  @critical
  @happy-path
  Scenario: Add blocks relationship between work units
    Given I have a project with spec directory
    And a work unit "API-001" exists with title "Build API endpoint"
    And a work unit "AUTH-001" exists with title "OAuth integration"
    When I run "fspec add-dependency AUTH-001 --blocks=API-001"
    Then the command should succeed
    And work unit "AUTH-001" should have blocks array containing "API-001"
    And work unit "API-001" should have blockedBy array containing "AUTH-001"
    And the bidirectional relationship should be maintained

  @happy-path
  Scenario: Add blockedBy relationship (inverse of blocks)
    Given I have a project with spec directory
    And a work unit "UI-001" exists
    And a work unit "API-001" exists
    When I run "fspec add-dependency UI-001 --blocked-by=API-001"
    Then the command should succeed
    And work unit "UI-001" should have blockedBy array containing "API-001"
    And work unit "API-001" should have blocks array containing "UI-001"

  @happy-path
  Scenario: Add dependsOn relationship for soft dependency
    Given I have a project with spec directory
    And a work unit "DASH-001" exists with title "User dashboard"
    And a work unit "AUTH-001" exists with title "User authentication"
    When I run "fspec add-dependency DASH-001 --depends-on=AUTH-001"
    Then the command should succeed
    And work unit "DASH-001" should have dependsOn array containing "AUTH-001"
    And work unit "AUTH-001" should NOT have any automatic reverse relationship

  @happy-path
  Scenario: Add relatesTo relationship for informational linking
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a work unit "SEC-001" exists with title "Security audit"
    When I run "fspec add-dependency AUTH-001 --relates-to=SEC-001"
    Then the command should succeed
    And work unit "AUTH-001" should have relatesTo array containing "SEC-001"

  @happy-path
  Scenario: Add multiple relationships of same type
    Given I have a project with spec directory
    And work units exist: "API-001", "API-002", "UI-001"
    When I run "fspec add-dependency UI-001 --depends-on=API-001"
    And I run "fspec add-dependency UI-001 --depends-on=API-002"
    Then work unit "UI-001" should have dependsOn array containing "API-001" and "API-002"

  @happy-path
  Scenario: Add multiple relationship types to same work unit
    Given I have a project with spec directory
    And work units exist: "AUTH-001", "API-001", "DB-001", "SEC-001"
    When I run "fspec add-dependency AUTH-001 --blocks=API-001"
    And I run "fspec add-dependency AUTH-001 --depends-on=DB-001"
    And I run "fspec add-dependency AUTH-001 --relates-to=SEC-001"
    Then work unit "AUTH-001" should have:
      | relationship | value   |
      | blocks       | API-001 |
      | dependsOn    | DB-001  |
      | relatesTo    | SEC-001 |

  @remove
  @happy-path
  Scenario: Remove blocks relationship
    Given I have a project with spec directory
    And work unit "AUTH-001" blocks "API-001"
    When I run "fspec remove-dependency AUTH-001 --blocks=API-001"
    Then the command should succeed
    And work unit "AUTH-001" blocks array should not contain "API-001"
    And work unit "API-001" blockedBy array should not contain "AUTH-001"

  @remove
  @happy-path
  Scenario: Remove dependsOn relationship
    Given I have a project with spec directory
    And work unit "UI-001" dependsOn "API-001"
    When I run "fspec remove-dependency UI-001 --depends-on=API-001"
    Then the command should succeed
    And work unit "UI-001" dependsOn array should not contain "API-001"

  @validation
  @circular-dependency
  Scenario: Detect direct circular dependency
    Given I have a project with spec directory
    And work unit "A" blocks "B"
    When I run "fspec add-dependency B --blocks=A"
    Then the command should fail
    And the error should contain "Circular dependency detected"
    And the error should show cycle: "A → B → A"

  @validation
  @circular-dependency
  Scenario: Detect transitive circular dependency
    Given I have a project with spec directory
    And work unit "A" blocks "B"
    And work unit "B" blocks "C"
    When I run "fspec add-dependency C --blocks=A"
    Then the command should fail
    And the error should contain "Circular dependency detected"
    And the error should show cycle: "A → B → C → A"

  @validation
  @circular-dependency
  Scenario: Detect complex circular dependency chain
    Given I have a project with spec directory
    And work unit "AUTH-001" blocks "API-001"
    And work unit "API-001" blocks "DB-001"
    And work unit "DB-001" blocks "CACHE-001"
    When I run "fspec add-dependency CACHE-001 --blocks=AUTH-001"
    Then the command should fail
    And the error should contain "Circular dependency detected"
    And the error should show full cycle chain

  @validation
  @error-handling
  Scenario: Attempt to add dependency to non-existent work unit
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And no work unit "API-999" exists
    When I run "fspec add-dependency AUTH-001 --blocks=API-999"
    Then the command should fail
    And the error should contain "Work unit 'API-999' does not exist"

  @validation
  @error-handling
  Scenario: Attempt to add self as dependency
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec add-dependency AUTH-001 --blocks=AUTH-001"
    Then the command should fail
    And the error should contain "Cannot create dependency to self"

  @validation
  @duplicate
  Scenario: Attempt to add duplicate dependency
    Given I have a project with spec directory
    And work unit "AUTH-001" blocks "API-001"
    When I run "fspec add-dependency AUTH-001 --blocks=API-001"
    Then the command should fail
    And the error should contain "Dependency already exists"

  @auto-blocking
  @workflow
  Scenario: Auto-transition to blocked state when blockedBy exists
    Given I have a project with spec directory
    And a work unit "UI-001" exists with status "backlog"
    And a work unit "API-001" exists with status "implementing"
    When I run "fspec add-dependency UI-001 --blocked-by=API-001"
    Then the command should succeed
    And work unit "UI-001" status should automatically change to "blocked"
    And work unit "UI-001" blockedReason should be "Blocked by API-001"

  @auto-blocking
  @workflow
  Scenario: Auto-unblock when blocker completes
    Given I have a project with spec directory
    And work unit "UI-001" is blocked by "API-001"
    And work unit "UI-001" has status "blocked"
    And work unit "API-001" has status "validating"
    When I run "fspec update-work-unit API-001 --status=done"
    Then the command should succeed
    And work unit "UI-001" status should remain "blocked"
    And work unit "UI-001" should display notification "Blocker API-001 completed, ready to unblock"

  @auto-blocking
  @workflow
  Scenario: Manual unblock after blocker completes
    Given I have a project with spec directory
    And work unit "UI-001" is blocked by "API-001"
    And work unit "API-001" has status "done"
    When I run "fspec update-work-unit UI-001 --status=backlog"
    Then the command should succeed
    And work unit "UI-001" status should be "backlog"
    And the blockedBy relationship should still exist

  @query
  @visualization
  Scenario: Show work unit with all dependencies
    Given I have a project with spec directory
    And a work unit "AUTH-001" has dependencies:
      | type      | target  |
      | blocks    | API-001 |
      | blocks    | UI-001  |
      | dependsOn | DB-001  |
      | relatesTo | SEC-001 |
    When I run "fspec show-work-unit AUTH-001"
    Then the output should display all dependency types
    And the output should show "Blocks: API-001, UI-001"
    And the output should show "Depends On: DB-001"
    And the output should show "Related To: SEC-001"

  @query
  @visualization
  Scenario: Display dependency graph for work unit
    Given I have a project with spec directory
    And work unit "AUTH-001" blocks "API-001"
    And work unit "AUTH-001" blocks "UI-001"
    And work unit "API-001" blocks "CACHE-001"
    When I run "fspec show-dependencies AUTH-001 --graph"
    Then the output should display dependency tree:
      """
      AUTH-001
      ├─blocks→ API-001
      │         └─blocks→ CACHE-001
      └─blocks→ UI-001
      """

  @query
  @visualization
  Scenario: Show all work units blocked by specific work unit
    Given I have a project with spec directory
    And work unit "API-001" blocks "UI-001", "DASH-001", "MOBILE-001"
    When I run "fspec query work-units --blocked-by=API-001 --output=json"
    Then the output should contain 3 work units
    And the output should include "UI-001", "DASH-001", "MOBILE-001"

  @query
  @filtering
  Scenario: Find all currently blocked work units
    Given I have a project with spec directory
    And work units exist:
      | id       | status  | blockedBy |
      | UI-001   | blocked | API-001   |
      | DASH-001 | blocked | AUTH-001  |
      | MOB-001  | backlog |           |
    When I run "fspec query work-units --status=blocked --output=json"
    Then the output should contain 2 work units
    And the output should include "UI-001" and "DASH-001"

  @query
  @impact-analysis
  Scenario: Show impact analysis when completing work unit
    Given I have a project with spec directory
    And work unit "API-001" blocks "UI-001", "DASH-001", "MOBILE-001"
    When I run "fspec query impact API-001"
    Then the output should show "Completing API-001 will unblock:"
    And the output should list "UI-001", "DASH-001", "MOBILE-001"
    And the output should show total: "3 work units ready to proceed"

  @query
  @impact-analysis
  Scenario: Show dependency chain depth
    Given I have a project with spec directory
    And work unit "AUTH-001" blocks "API-001"
    And work unit "API-001" blocks "UI-001"
    And work unit "UI-001" blocks "MOBILE-001"
    When I run "fspec query dependency-chain AUTH-001"
    Then the output should show chain:
      """
      AUTH-001 → API-001 → UI-001 → MOBILE-001
      Chain depth: 4
      """

  @critical-path
  @query
  Scenario: Calculate critical path through dependencies
    Given I have a project with spec directory
    And work units with estimates and dependencies:
      | id       | estimate | blocks   |
      | AUTH-001 | 8        | API-001  |
      | API-001  | 5        | UI-001   |
      | UI-001   | 3        | DEPLOY-1 |
      | DB-001   | 2        | DEPLOY-1 |
    When I run "fspec query critical-path --from=AUTH-001 --to=DEPLOY-1"
    Then the output should show critical path: "AUTH-001 → API-001 → UI-001 → DEPLOY-1"
    And the output should show total estimate: "16 story points"

  @validation
  @state-transition
  Scenario: Prevent starting work that is blocked
    Given I have a project with spec directory
    And work unit "UI-001" is blocked by "API-001"
    And work unit "API-001" has status "implementing"
    And work unit "UI-001" has status "blocked"
    When I run "fspec update-work-unit UI-001 --status=specifying"
    Then the command should fail
    And the error should contain "Cannot start work that is blocked"
    And the error should list blocker: "API-001 (status: implementing)"

  @validation
  @delete
  Scenario: Prevent deleting work unit that blocks others
    Given I have a project with spec directory
    And work unit "API-001" blocks "UI-001", "DASH-001"
    When I run "fspec delete-work-unit API-001 --force"
    Then the command should fail
    And the error should contain "Cannot delete work unit that blocks other work"
    And the error should suggest "Remove blocking relationships first"

  @cascade
  @delete
  Scenario: Cascade delete dependencies when removing work unit
    Given I have a project with spec directory
    And work unit "AUTH-001" has dependencies:
      | type      | target  |
      | blocks    | API-001 |
      | dependsOn | DB-001  |
    When I run "fspec delete-work-unit AUTH-001 --cascade-dependencies --force"
    Then the command should succeed
    And work unit "API-001" blockedBy should not contain "AUTH-001"
    And work unit "AUTH-001" should not exist

  @bulk
  @operations
  Scenario: Add multiple dependencies in one command
    Given I have a project with spec directory
    And work units exist: "AUTH-001", "API-001", "UI-001", "DB-001"
    When I run "fspec add-dependencies AUTH-001 --blocks=API-001,UI-001 --depends-on=DB-001"
    Then the command should succeed
    And work unit "AUTH-001" should block "API-001" and "UI-001"
    And work unit "AUTH-001" should depend on "DB-001"

  @bulk
  @operations
  Scenario: Remove all dependencies from work unit
    Given I have a project with spec directory
    And work unit "AUTH-001" has dependencies:
      | type      | target  |
      | blocks    | API-001 |
      | dependsOn | DB-001  |
      | relatesTo | SEC-001 |
    When I run "fspec clear-dependencies AUTH-001"
    Then the command should prompt for confirmation
    When I confirm
    Then all dependencies should be removed from "AUTH-001"
    And bidirectional relationships should be cleaned up

  @validation
  @json-schema
  Scenario: Validate dependency data structure
    Given I have a project with spec directory
    And work units exist with dependencies
    When I run "fspec validate-work-units"
    Then the validation should check dependency arrays contain valid work unit IDs
    And the validation should check bidirectional consistency
    And the validation should detect orphaned dependency references

  @consistency
  @repair
  Scenario: Repair broken bidirectional dependencies
    Given I have a project with spec directory
    And work unit "AUTH-001" has blocks array containing "API-001"
    But work unit "API-001" blockedBy array does not contain "AUTH-001"
    When I run "fspec repair-work-units"
    Then the command should detect inconsistency
    And the command should add "AUTH-001" to "API-001" blockedBy array
    And the command should report "Repaired 1 bidirectional dependency"

  @reporting
  @metrics
  Scenario: Show dependency statistics
    Given I have a project with spec directory
    And work units exist with various dependencies
    When I run "fspec query dependency-stats --output=json"
    Then the output should show:
      | metric                            | value |
      | work units with blockers          | 5     |
      | work units blocking others        | 3     |
      | work units with soft dependencies | 8     |
      | average dependencies per unit     | 2.3   |
      | max dependency chain depth        | 4     |

  @visualization
  @mermaid
  Scenario: Generate Mermaid diagram of dependencies
    Given I have a project with spec directory
    And work units with dependencies:
      | id       | blocks  |
      | AUTH-001 | API-001 |
      | API-001  | UI-001  |
      | DB-001   | API-001 |
    When I run "fspec export-dependencies --format=mermaid --output=deps.mmd"
    Then the file should contain valid Mermaid syntax:
      """
      graph TD
        AUTH-001 -->|blocks| API-001
        API-001 -->|blocks| UI-001
        DB-001 -->|blocks| API-001
      """

  @warning
  @soft-dependency
  Scenario: Warn when starting work with incomplete dependencies
    Given I have a project with spec directory
    And work unit "UI-001" depends on "API-001"
    And work unit "API-001" has status "implementing"
    When I run "fspec update-work-unit UI-001 --status=specifying"
    Then the command should display warning "Dependency API-001 not yet complete"
    But the transition should succeed
    And the warning should be informational only
