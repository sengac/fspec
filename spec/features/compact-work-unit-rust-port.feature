@done
@RPC-206
@rust
@cli
@mutation
Feature: Port compact-work-unit command to Rust
  """
  Core impl at rust/fspec-core/src/commands/compact_work_unit.rs: ensure_work_units_file -> existence check -> force gate on status!='done' -> compact rules/examples/questions/architectureNotes (filter deleted:true, renumber id from 0) -> reset nextRuleId/nextExampleId/nextQuestionId/nextNoteId -> set updatedAt + meta.lastUpdated -> single write_json_atomic. Arrays and counters live in WorkUnit.extra; status is a typed field; reuse iso8601_now.
  CLI bridge rust/fspec/src/compact_work_unit.rs marshals {workUnitId, force?} JSON only. The dispatcher returns the rendered summary text. Framing A: the TS CLI action discards result.warning and the renumber-range line shown in help-doc examples; Rust mirrors the ACTUAL CLI output, not the divergent help-doc examples.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::compact_work_unit::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer
    I want to port the compact-work-unit command to the Rust fspec-core crate
    So that the standalone fspec binary can permanently remove soft-deleted Example Mapping items and renumber IDs natively without delegating to TypeScript

  Scenario: Dispatcher removes soft-deleted rules and renumbers the survivors
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 3 deleted rules and 7 live rules
    When I dispatch compact-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the AUTH-001 rules array in spec/work-units.json contains 7 items
    And the surviving AUTH-001 rules have sequential ids 0 through 6
    And nextRuleId on AUTH-001 equals 7

  Scenario: Dispatcher rejects compaction of a missing work unit
    Given spec/work-units.json contains work unit AUTH-001 with status='done'
    When I dispatch compact-work-unit with workUnitId='MISSING-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-999' does not exist"

  Scenario: Dispatcher requires force when status is not done
    Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule
    When I dispatch compact-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot compact work unit in 'specifying' status. Use --force to confirm compaction during active development."
    And the AUTH-001 rules array in spec/work-units.json still contains the deleted rule

  Scenario: Dispatcher compacts during non-done status when force is set
    Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule and 2 live rules
    When I dispatch compact-work-unit with workUnitId='AUTH-001' and force=true
    Then the dispatcher returns success=true
    And the AUTH-001 rules array in spec/work-units.json contains 2 items

  Scenario: Dispatcher resets counters when there are no deleted items
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 3 live rules and no deleted items
    When I dispatch compact-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the AUTH-001 rules array in spec/work-units.json contains 3 items
    And nextRuleId on AUTH-001 equals 3

  Scenario: Dispatcher updates the work unit and meta timestamps
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 1 deleted rule and a meta.lastUpdated value
    When I dispatch compact-work-unit with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the AUTH-001 updatedAt field in spec/work-units.json is a non-empty ISO-8601 timestamp
    And the meta.lastUpdated field in spec/work-units.json is a non-empty ISO-8601 timestamp

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch compact-work-unit with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command compact-work-unit'
