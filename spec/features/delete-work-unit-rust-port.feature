@done
@RPC-223
@rust
@cli
@mutation
Feature: Port delete-work-unit command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/delete_work_unit.rs: ensure_work_units_file -> existence/children/dependency checks -> optional cascade dereference (blocks/blockedBy/relatesTo, NOT dependsOn) -> parent.children filter -> states filter -> remove from work_units IndexMap -> single write_json_atomic. children/blocks/blockedBy/dependsOn/relatesTo/parent read from WorkUnit.extra as JSON arrays/strings.
  CLI bridge codelet/fspec/src/delete_work_unit.rs marshals {workUnitId, cascadeDependencies?} JSON only; --force/--skip-confirmation parsed by clap for parity but NOT forwarded. Success prints '✓ Work unit <id> deleted successfully' + '⚠ <warning>' lines; errors to stderr prefixed '✗ Failed to delete work unit:'; exit 1 on error.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::delete_work_unit::run(args_json, project_root). The CLI bridge marshals only — no validation or rendering logic is duplicated.
  """

  Background: User Story
    As a fspec maintainer
    I want to port the delete-work-unit command to the Rust fspec-core crate
    So that the standalone fspec binary can delete work units natively (with cascade-dependency cleanup) without delegating to TypeScript

  Scenario: Dispatcher deletes an existing leaf work unit with no dependencies
    Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    When I dispatch delete-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the line '✓ Work unit AUTH-001 deleted successfully'
    And spec/work-units.json no longer contains the AUTH-001 work unit

  Scenario: Dispatcher rejects deletion of a missing work unit
    Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    When I dispatch delete-work-unit with workUnitId='MISSING-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-999' does not exist"
    And spec/work-units.json still contains the AUTH-001 work unit

  Scenario: Dispatcher refuses to delete a work unit that has children
    Given spec/work-units.json contains work unit AUTH-999 with children AUTH-002 and AUTH-003
    When I dispatch delete-work-unit with workUnitId='AUTH-999'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Cannot delete work unit with children: AUTH-002, AUTH-003. Delete children first or remove parent relationship.'
    And spec/work-units.json still contains the AUTH-999 work unit

  Scenario: Dispatcher refuses to delete a work unit with dependencies without cascade
    Given spec/work-units.json contains work unit AUTH-001 with dependsOn AUTH-000
    When I dispatch delete-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete."
    And spec/work-units.json still contains the AUTH-001 work unit

  Scenario: Dispatcher cascades blocks references and emits a blocks warning
    Given spec/work-units.json contains work unit AUTH-001 with blocks API-001 and work unit API-001 with blockedBy AUTH-001
    When I dispatch delete-work-unit with workUnitId='AUTH-001' and cascadeDependencies=true
    Then the dispatcher returns success=true
    And the API-001 work unit in spec/work-units.json no longer lists AUTH-001 in its blockedBy
    And the DispatchResult.data contains the substring '⚠ This work unit blocks 1 work unit(s): API-001'
    And spec/work-units.json no longer contains the AUTH-001 work unit

  Scenario: Dispatcher does NOT cascade dependsOn references
    Given spec/work-units.json contains work unit AUTH-001 with dependsOn AUTH-000 and work unit AUTH-000 with no references
    When I dispatch delete-work-unit with workUnitId='AUTH-001' and cascadeDependencies=true
    Then the dispatcher returns success=true
    And the AUTH-000 work unit in spec/work-units.json is unchanged
    And spec/work-units.json no longer contains the AUTH-001 work unit

  Scenario: Dispatcher removes the unit from its parent's children array
    Given spec/work-units.json contains work unit AUTH-PARENT with children AUTH-CHILD and work unit AUTH-CHILD with parent AUTH-PARENT
    When I dispatch delete-work-unit with workUnitId='AUTH-CHILD'
    Then the dispatcher returns success=true
    And the AUTH-PARENT work unit in spec/work-units.json no longer lists AUTH-CHILD in its children
    And spec/work-units.json no longer contains the AUTH-CHILD work unit

  Scenario: Dispatcher removes the unit from its state index array
    Given spec/work-units.json contains work unit AUTH-001 with status='specifying' listed in states.specifying
    When I dispatch delete-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And states.specifying in spec/work-units.json no longer contains AUTH-001

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch delete-work-unit with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command delete-work-unit'
