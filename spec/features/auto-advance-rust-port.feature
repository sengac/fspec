@done
@workflow-automation
@cli
@RPC-198
@BUG-661
Feature: Port auto-advance command to Rust
  """
  File layout: core rust/fspec-core/src/commands/auto_advance.rs (rewrite stub) is single source of truth run(args_json, project_root); CLI bridge rust/fspec/src/auto_advance.rs (Framing A, marshals {} ignoring --dry-run); help config rust/fspec-core/src/help/configs/auto_advance.rs; help fixture rust/fspec/tests/fixtures/help/auto-advance.txt; integration test rust/fspec/tests/cli_auto_advance.rs; core test rust/fspec-core/tests/auto_advance.rs
  Reuses shared types/helpers: WorkUnitsData/WorkUnitStatus (types/work_unit.rs), write_json_atomic (io/locked_file.rs), iso8601_now (io/time.rs). Mutation uses raw serde_json::Map round-trip (like update_work_unit.rs) to preserve on-disk key order + unmodelled fields. No new shared files required.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Core run(args_json, project_root) mirrors autoAdvance: missing work unit throws 'Work unit <id> not found'; no matching transition throws 'No transition defined for <from> + <event>'; status mismatch throws 'Work unit is in <status> state, expected <from>'
  #   2. Two transitions only: testing+tests-pass -> implementing; validating+validation-pass -> done (sets doneAt). Every move appends a stateHistory entry {state, timestamp, reason: auto-advance} (BUG-661). Mutates states arrays (remove from `from`, push to `to`), sets status, updatedAt, persists via atomic write, returns {success:true,newState:<to>}
  #   3. All errors wrapped with 'Failed to auto-advance: ' prefix. The CLI shell is BROKEN (Framing A): .action() calls autoAdvance({dryRun}) without workUnitId/from/event, so it always fails with 'Work unit undefined not found'; the Rust CLI bridge must reproduce this (send no workUnitId, print '✗ Failed to auto-advance: Work unit undefined not found' to stderr, exit 1)
  #   4. Both front doors converge on one async run(args_json, project_root) in fspec-core; help is intercepted byte-for-byte against tests/fixtures/help/auto-advance.txt (captured from node dist/index.js auto-advance --help); --help must exit 0
  #
  # EXAMPLES:
  #   1. Dispatcher call with {workUnitId:'AUTH-001', from:'testing', event:'tests-pass'} where AUTH-001 status=testing advances it to implementing and returns {success:true,newState:'implementing'}
  #   2. Dispatcher call with from:'validating', event:'validation-pass' on a validating unit advances to done and sets the doneAt timestamp
  #   3. Dispatcher call with from:'testing', event:'bogus' returns error containing 'No transition defined for testing + bogus'
  #   4. Dispatcher call where unit status is implementing but from:'testing' returns error 'Work unit is in implementing state, expected testing'
  #   5. Running `fspec auto-advance` (or with --dry-run) from a shell exits 1 and prints '✗ Failed to auto-advance: Work unit undefined not found' to stderr (broken Framing-A shell)
  #
  # ========================================
  Background: User Story
    Given the auto-advance command is ported to Rust in rust/fspec-core/src/commands/auto_advance.rs
    And both the LLM dispatcher and the standalone Rust binary call the same fspec_core::commands::auto_advance::run function

  Scenario: Dispatcher advances a testing work unit to implementing on tests-pass
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    When I dispatch auto-advance through fspec_core::dispatch::dispatch_command with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    Then the dispatch result succeeds
    And the returned JSON shows success true and newState 'implementing'
    And the persisted AUTH-001 status is 'implementing'
    And AUTH-001 is removed from the states.testing array and present in the states.implementing array

  Scenario: Dispatcher advances a validating work unit to done and records completion
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'validating'
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'validating', and event 'validation-pass'
    Then the dispatch result succeeds
    And the returned JSON shows success true and newState 'done'
    And the persisted AUTH-001 status is 'done'
    And the persisted AUTH-001 has a non-empty doneAt timestamp

  Scenario: Dispatcher rejects an undefined transition
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'bogus'
    Then the dispatch result fails
    And the error message contains 'No transition defined for testing + bogus'

  Scenario: Dispatcher rejects a state mismatch
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'implementing'
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    Then the dispatch result fails
    And the error message contains 'Work unit is in implementing state, expected testing'

  Scenario: Dispatcher rejects a missing work unit with the wrapped prefix
    Given a project root whose spec/work-units.json contains no work unit MISSING-001
    When I dispatch auto-advance with workUnitId 'MISSING-001', from 'testing', and event 'tests-pass'
    Then the dispatch result fails
    And the error message contains 'Work unit MISSING-001 not found'

  Scenario: auto-advance appends a stateHistory entry with reason auto-advance
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing' and one stateHistory entry for testing
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    Then the persisted AUTH-001 has 2 stateHistory entries and the first is unchanged
    And the last stateHistory entry has state 'implementing', reason 'auto-advance' and the timestamp of AUTH-001's updatedAt

  Scenario: A card auto-advanced to done is found by list-work-units --done-since
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'validating'
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'validating', and event 'validation-pass'
    Then the last stateHistory entry of AUTH-001 has state 'done', reason 'auto-advance' and the timestamp of its doneAt
    And list-work-units with doneSince '2000-01-01', fields 'doneAt' and format 'json' lists AUTH-001 with that doneAt

  Scenario: auto-advance in an isolated session records the session id on its stateHistory entry
    Given a main checkout holding AUTH-001 with status 'testing' and a worktree whose .fspec/state-root pointer names session 's1'
    When I dispatch auto-advance in the worktree with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    Then the main board's last stateHistory entry of AUTH-001 has state 'implementing' and sessionId 's1'

  Scenario: auto-advance fails with an exact error when stateHistory is not an array
    Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing' and stateHistory 'legacy'
    When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    Then the error message contains 'Failed to auto-advance: Work unit AUTH-001 has a malformed stateHistory field: expected an array. Fix spec/work-units.json.'
    And spec/work-units.json is byte-for-byte unchanged
