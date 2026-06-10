@done
@querying
@cli
@RPC-303
Feature: Port show-event-storm command to Rust

  """
  Uses ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults `{ version: '1.0.0', workUnits: {}, states: {} }` when missing) — TS source-of-truth at src/commands/show-event-storm.ts:38-45 calls fileManager.readJSON with the same default payload, and fileManager.readJSON auto-creates on ENOENT, so the Rust port matches that auto-create behaviour exactly.

  Reads the eventStorm.items array from the work unit's typed flatten-extra map (commands::show_event_storm::run does NOT promote eventStorm to a typed field on shared types::work_unit::WorkUnit — it walks WorkUnit.extra.get("eventStorm").get("items") and treats anything other than an array as 'no Event Storm data').

  Items are emitted to stdout as a serde_json::Value array (pretty-printed, 2-space indent), preserving every field on every retained item exactly as stored — Rust does NOT promote known fields onto typed structs because the TS source just JSON.stringifies the raw filtered array.

  Filter rule: each item is retained iff `!item.deleted` — Rust mirrors TS truthy-coercion by using `serde_json::Value::Bool(true)` membership tests with a `Some(true)` short-circuit, so a missing `deleted` field is treated as `false` (kept).

  All recoverable errors (work unit missing, missing eventStorm data, missing eventStorm.items array) live in the success/error envelope on the dispatcher path and surface as stderr 'Error:' + exit 1 on the CLI path. Only args_json parse failures escalate to FspecCoreError::InvalidArgs.

  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::show_event_storm::run function; CLI bridge does only JSON arg marshalling and stdout rendering.

  The dispatcher payload shape is `{ workUnitId: String (REQUIRED) }`. The clap variant exposes only a required positional `<work-unit-id>` argument and no flags — matching TS Commander.js registration at src/commands/show-event-storm.ts:107-115.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json via the shared ensure_work_units_file helper (auto-creating the file with canonical defaults when missing)
  #   2. Looks up workUnitsData.workUnits[workUnitId]; if the key is absent the command returns success=false with error 'Work unit <id> not found'
  #   3. If the work unit exists but lacks an eventStorm.items array, returns success=false with error 'Work unit <id> has no Event Storm data'
  #   4. Filters eventStorm.items to only items where item.deleted is NOT truthy — soft-deleted items are excluded from output
  #   5. The output is the JSON-pretty-printed array of active items (2-space indentation), preserving every field on every item exactly as stored
  #   6. The CLI exit code is 0 on success and 1 on any error (work unit missing, missing eventStorm data, or filesystem failure)
  #   7. Error messages are written to stderr prefixed 'Error:' and the JSON array is written to stdout
  #   8. The CLI subcommand exposes a single required positional <work-unit-id> argument and no flags
  #   9. The dispatcher payload shape is { workUnitId: String (REQUIRED) }; both invocation paths call fspec_core::commands::show_event_storm::run
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of show-event-storm wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one Event Storm display implementation

  Scenario: Returns Work unit not found error when spec/work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: Returns Work unit not found error when workUnitId is not a key in workUnits
    Given spec/work-units.json contains BUG-001 but not AUTH-001
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'

  Scenario: Returns no Event Storm data error when the unit has no eventStorm field
    Given spec/work-units.json contains AUTH-001 with no eventStorm field
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'

  Scenario: Returns no Event Storm data error when eventStorm exists but has no items array
    Given spec/work-units.json contains AUTH-001 with eventStorm={} (no items field)
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'

  Scenario: Returns the active items as a pretty-printed JSON array
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), command(id=1, deleted=false)]
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as a JSON array of length 2
    Then the parsed array[0] has id=0 and type='event'
    Then the parsed array[1] has id=1 and type='command'
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Filters out soft-deleted items
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), event(id=1, deleted=true), command(id=2, deleted=false)]
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as a JSON array of length 2
    Then the parsed array[0] has id=0
    Then the parsed array[1] has id=2

  Scenario: Treats missing deleted field as retained
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0) (no deleted field)]
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as a JSON array of length 1

  Scenario: Returns an empty array when eventStorm.items is empty
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[]
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as the empty JSON array

  Scenario: Preserves every field on every retained item
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[policy(id=0, deleted=false, when='UserRegistered', then='SendWelcomeEmail', color='purple', type='policy', text='Send welcome email')]
    When I dispatch show-event-storm with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the parsed array[0] has type='policy', text='Send welcome email', when='UserRegistered', then='SendWelcomeEmail', color='purple'

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'

  Scenario: Missing workUnitId argument surfaces a structured InvalidArgs error
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0)]
    When I dispatch show-event-storm with no workUnitId argument
    Then the dispatcher returns success=false with an error message containing the substring 'failed to parse args'
