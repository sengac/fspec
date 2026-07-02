@done
@feature-management
@cli
@RPC-232
Feature: Port generate-example-mapping-from-event-storm command to Rust
  """
  File layout: core impl codelet/fspec-core/src/commands/generate_example_mapping_from_event_storm.rs (rewrite stub); CLI bridge codelet/fspec/src/generate_example_mapping_from_event_storm.rs; help config codelet/fspec-core/src/help/configs/generate_example_mapping_from_event_storm.rs; help fixture codelet/fspec/tests/fixtures/help/generate-example-mapping-from-event-storm.txt; core test codelet/fspec-core/tests/generate_example_mapping_from_event_storm.rs; CLI test codelet/fspec/tests/cli_generate_example_mapping_from_event_storm.rs. Module already registered as a stub in commands/mod.rs (do not edit).
  Shared types reused: crate::types::work_unit::WorkUnitsData (rules/examples/questions/nextXId all read+written via WorkUnit.extra, same pattern as add_rule.rs / add_example.rs / add_question.rs); eventStorm.items walked via WorkUnit.extra.get('eventStorm') (same as show_event_storm.rs). Meta.last_updated is a typed field on WorkUnitsData::meta (Meta::last_updated). Reuses crate::io::locked_file::write_json_atomic and crate::io::time::iso8601_now. Missing-file Option B (inline path.exists()).
  SHARED-FN REQUEST (supervisor): pascalCaseToSentence (src/utils/text-formatting.ts) — inserts a space before each uppercase letter, trims, lowercases. No Rust equivalent exists in fspec-core. Need a shared crate::text_format::pascal_case_to_sentence (also reused by future event-storm transforms). Will ASK before inlining. Also note: TS question object writes answer:undefined which JSON.stringify omits, so on-disk question shape is { id, text, deleted, createdAt } (no answer key) — matches add_question.rs output.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both the LLM dispatcher and the clap subcommand call the single fspec_core::commands::generate_example_mapping_from_event_storm::run function (two-front-doors); the CLI bridge does only JSON marshalling and stdout/stderr rendering
  #   2. Reads spec/work-units.json with NO auto-create (Option B): if the file does not exist the command fails with 'spec/work-units.json not found. Run fspec init first.'
  #   3. If workUnits[workUnitId] is absent the command fails with 'Work unit <id> not found'; if the work unit has no eventStorm.items array it fails with 'Work unit <id> has no Event Storm data'
  #   4. For each non-deleted eventStorm item of type 'policy' with both when and then set, a rule is appended: text = 'System must <then> after <when>' where <when>/<then> are pascalCaseToSentence converted (space before each uppercase letter, trimmed, lowercased)
  #   5. BUG-089: NO examples are derived from events — examplesAdded is always 0, leaving the examples list for humans to populate with concrete contextual examples
  #   6. BUG-088: For each non-deleted eventStorm item of type 'hotspot' with a concern, a question is appended: text = '@human: <concern>' with the concern trimmed and a trailing '?' added only if it does not already end with '?'
  #   7. Generated rule and question items use sequential IDs from the work unit's nextRuleId/nextQuestionId counters (initialized to 0 when undefined), with deleted:false and a fresh ISO-8601 createdAt; rules/examples/questions arrays are initialized when absent
  #   8. Deleted eventStorm items (deleted:true) are skipped during transformation
  #   9. On success the work unit's updatedAt and the file's meta.lastUpdated are set to a fresh ISO-8601 timestamp, the data is persisted via a single atomic write, and the result reports rulesAdded, examplesAdded, questionsAdded
  #   10. The clap subcommand exposes exactly one required positional <work-unit-id> argument and no flags; --help is byte-for-byte identical to the captured TS help fixture; CLI exits 0 on success and 1 on any error with stderr prefixed 'Error:'
  #
  # EXAMPLES:
  #   1. Agent runs generate-example-mapping-from-event-storm on a unit whose Event Storm has 2 policies and 1 hotspot, and sees 'Rules added: 2, Examples added: 0, Questions added: 1' with the rules and questions appended to the work unit
  #   2. Agent runs the command on a unit with no eventStorm data and sees 'Work unit X has no Event Storm data' with exit code 1
  #   3. Agent runs the command on a unit with a policy when='UserRegistered' then='SendWelcomeEmail' and the generated rule reads 'System must send welcome email after user registered'
  #   4. Agent runs the command on a unit with a hotspot concern='Unclear how long to wait' and the generated question reads '@human: Unclear how long to wait?' (trailing ? added)
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to run a Rust port of generate-example-mapping-from-event-storm wired through both the LLM dispatcher and the clap subcommand
    So that Event Storm artifacts are transformed into Example Mapping by one shared implementation across the daemon and the standalone Rust binary

  Scenario: Dispatcher derives rules from policies and questions from hotspots
    Given spec/work-units.json contains AUTH-001 in specifying status with an eventStorm containing 2 policies (each with when+then) and 1 hotspot with a concern
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success with rulesAdded=2, examplesAdded=0, questionsAdded=1
    Then the work unit's rules array contains 2 new rule entries
    Then the work unit's questions array contains 1 new question entry
    Then the work unit's examples array remains empty

  Scenario: Dispatcher returns missing-file error in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'spec/work-units.json not found. Run fspec init first.'
    Then spec/work-units.json does NOT exist after the call

  Scenario: Dispatcher returns Work unit not found when id absent
    Given spec/work-units.json contains BUG-001 but not AUTH-001
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'

  Scenario: Dispatcher returns no Event Storm data error when the unit lacks eventStorm.items
    Given spec/work-units.json contains AUTH-001 with no eventStorm field
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'

  Scenario: Policy is converted to a rule using pascalCaseToSentence WHEN/THEN pattern
    Given spec/work-units.json contains AUTH-001 with an eventStorm policy when='UserRegistered' then='SendWelcomeEmail'
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then a rule is appended with text exactly 'System must send welcome email after user registered'

  Scenario: Hotspot concern is converted to an at-human question with a trailing question mark added
    Given spec/work-units.json contains AUTH-001 with an eventStorm hotspot concern='Unclear how long to wait'
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then a question is appended with text exactly '@human: Unclear how long to wait?'

  Scenario: Hotspot concern already ending in a question mark is preserved unchanged
    Given spec/work-units.json contains AUTH-001 with an eventStorm hotspot concern='How long to wait?'
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then a question is appended with text exactly '@human: How long to wait?'

  Scenario: Soft-deleted eventStorm items are skipped
    Given spec/work-units.json contains AUTH-001 with an eventStorm where the only policy and the only hotspot are marked deleted:true
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success with rulesAdded=0, examplesAdded=0, questionsAdded=0

  Scenario: Successful run bumps timestamps and persists atomically
    Given spec/work-units.json contains AUTH-001 with an eventStorm containing 1 policy with when+then
    When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    Then the work unit's updatedAt is refreshed to a new ISO-8601 timestamp
    Then the file meta.lastUpdated is refreshed to a new ISO-8601 timestamp
