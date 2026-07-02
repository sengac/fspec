@done
@workflow-automation
@cli
@RPC-326
Feature: Port workflow-automation command to Rust
  """
  File layout: core codelet/fspec-core/src/commands/workflow_automation.rs (rewrite stub) single source of truth run(args_json, project_root); CLI bridge codelet/fspec/src/workflow_automation.rs (positional action + work-unit-id, --event/--from-state); help config codelet/fspec-core/src/help/configs/workflow_automation.rs; help fixture codelet/fspec/tests/fixtures/help/workflow-automation.txt; integration test codelet/fspec/tests/cli_workflow_automation.rs; core test codelet/fspec-core/tests/workflow_automation.rs
  Reuses shared helpers: WorkUnitsData (types/work_unit.rs), write_json_atomic (io/locked_file.rs), iso8601_now (io/time.rs), glob_feature_files (io/feature_glob.rs) for validate-alignment. Mutating actions use raw serde_json::Map round-trip (like update_work_unit.rs) for key-order + unmodelled-field preservation; tag matching hand-rolled (word-boundary check) to avoid new regex dep. No new shared files required.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Core run(args_json, project_root) dispatches on `action`: record-iteration | auto-advance | validate-alignment. Unknown action (or auto-advance missing event/fromState) returns 'Invalid action: <action>'. All actions first check the work unit exists, else 'Work unit '<id>' does not exist'
  #   2. record-iteration increments the NESTED workUnit.metrics.iterations (defaulting absent metrics/iterations to 0), bumps updatedAt, persists atomically. This is distinct from RPC-264 record-iteration which writes a top-level iterations field
  #   3. auto-advance action requires both --event and --from-state; if workUnit.status !== fromState throws "Work unit '<id>' is in state '<status>', expected '<fromState>'". Valid transitions: tests-pass+testing->implementing, validation-pass+validating->done, specs-complete+specifying->testing; else 'Invalid transition: <event> from <fromState>'. Sets status, appends {state,timestamp} to stateHistory, updates states index, bumps updatedAt. NO completedAt (unlike RPC-198 auto-advance)
  #   4. validate-alignment is READ-ONLY: globs spec/features/**/*.feature, counts @<workUnitId> tag matches (word-boundary), returns {aligned: count>0, scenariosFound: count, features: [filenames]}. Does not write work-units.json
  #   5. Both front doors converge on one async run(args_json, project_root) in fspec-core; CLI bridge marshals positional <action> + <work-unit-id> and --event/--from-state flags; help intercepted byte-for-byte against tests/fixtures/help/workflow-automation.txt; --help exits 0
  #
  # EXAMPLES:
  #   1. Dispatcher: {action:'record-iteration', workUnitId:'AUTH-001'} increments AUTH-001.metrics.iterations from absent to 1 and returns {success:true, iterations:1}
  #   2. Dispatcher: {action:'auto-advance', workUnitId:'AUTH-001', event:'tests-pass', fromState:'testing'} on a testing unit advances to implementing, appends to stateHistory, returns {success:true,newState:'implementing'}
  #   3. Dispatcher: {action:'auto-advance', workUnitId:'AUTH-001', event:'specs-complete', fromState:'specifying'} advances a specifying unit to testing (transition absent from RPC-198 auto-advance)
  #   4. Dispatcher: {action:'validate-alignment', workUnitId:'AUTH-001'} with two feature files tagged @AUTH-001 returns {aligned:true, scenariosFound:<n>, features:[...]} without modifying work-units.json
  #   5. Dispatcher: {action:'frobnicate', workUnitId:'AUTH-001'} returns error 'Invalid action: frobnicate'; and auto-advance on a unit whose status differs from fromState returns "Work unit 'AUTH-001' is in state 'implementing', expected 'testing'"
  #
  # ========================================
  Background: User Story
    Given the workflow-automation command is ported to Rust in codelet/fspec-core/src/commands/workflow_automation.rs
    And both the LLM dispatcher and the standalone Rust binary call the same fspec_core::commands::workflow_automation::run function

  Scenario: record-iteration increments the nested metrics counter
    Given a project root whose spec/work-units.json contains AUTH-001 with no metrics
    When I dispatch workflow-automation with action 'record-iteration' and workUnitId 'AUTH-001'
    Then the dispatch result succeeds
    And the returned JSON shows success true and iterations 1
    And the persisted AUTH-001 has metrics.iterations equal to 1

  Scenario: auto-advance action advances a testing unit and records state history
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'tests-pass', and fromState 'testing'
    Then the dispatch result succeeds
    And the returned JSON shows success true and newState 'implementing'
    And the persisted AUTH-001 status is 'implementing'
    And the persisted AUTH-001 stateHistory contains an entry with state 'implementing'

  Scenario: auto-advance action supports the specs-complete transition
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'specifying'
    When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'specs-complete', and fromState 'specifying'
    Then the dispatch result succeeds
    And the returned JSON shows success true and newState 'testing'
    And the persisted AUTH-001 status is 'testing'

  Scenario: validate-alignment counts tagged scenarios without writing
    Given a project root whose spec/work-units.json contains AUTH-001
    And two feature files under spec/features tagged @AUTH-001
    When I dispatch workflow-automation with action 'validate-alignment' and workUnitId 'AUTH-001'
    Then the dispatch result succeeds
    And the returned JSON shows aligned true and scenariosFound greater than 0
    And spec/work-units.json is left byte-for-byte unchanged

  Scenario: Unknown action is rejected
    Given a project root whose spec/work-units.json contains AUTH-001
    When I dispatch workflow-automation with action 'frobnicate' and workUnitId 'AUTH-001'
    Then the dispatch result fails
    And the error message contains 'Invalid action: frobnicate'

  Scenario: auto-advance action rejects a state mismatch
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'implementing'
    When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'tests-pass', and fromState 'testing'
    Then the dispatch result fails
    And the error message contains "Work unit 'AUTH-001' is in state 'implementing', expected 'testing'"
