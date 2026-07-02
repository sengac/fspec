@wip
@RPC-165
Feature: Port add-aggregate command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_aggregate.rs. INLINE style: the TS source
  (src/commands/add-aggregate.ts) inlines the Event Storm mutation directly (it does NOT use the
  shared addEventStormItem util). The eventStorm section lives in WorkUnit.extra under key
  "eventStorm" with shape {level, items[], nextItemId}; round-tripped via #[serde(flatten)].
  Each item is appended to items with id=nextItemId, then nextItemId is post-incremented.
  Aggregate item field order: id, type ('aggregate'), color ('yellow'), text, deleted (false),
  createdAt (ISO-8601 now), then optional responsibilities (CSV split/trim/filter-empty array),
  timestamp (int ms), boundedContext — appended in that TS order only when present.
  Reuses io::time::iso8601_now and io::locked_file::write_json_atomic. Two-front-doors: the CLI
  bridge marshals JSON only.
  Validation parity: missing spec/work-units.json -> 'spec/work-units.json not found. Run fspec
  init first.'; missing work unit -> 'Work unit <id> not found'; done/blocked status ->
  'Cannot add Event Storm items to work unit in <status> state' with disk left unchanged.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the `add-aggregate` command as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append aggregate items to a work unit's Event Storm during Process Modeling without falling back to the TS implementation

  Scenario: First add seeds eventStorm and appends aggregate id 0 on a clean specifying work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Order'
    Then the dispatcher returns success=true
    And the returned data contains aggregateId=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].id=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].type='aggregate'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].color='yellow'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Order'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Second add appends with auto-incremented id
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with eventStorm having one item and nextItemId=1
    When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Customer'
    Then the dispatcher returns success=true
    And the returned data contains aggregateId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1].id=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1].text='Customer'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2

  Scenario: Responsibilities CSV is split, trimmed and empty-filtered into an array
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-aggregate with workUnitId='AUTH-001' text='User' and responsibilities='Manage credentials, Track sessions, '
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].responsibilities equals the array ["Manage credentials","Track sessions"]

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-aggregate with workUnitId='NOPE-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit NOPE-001 not found"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Done or blocked status is rejected verbatim and disk is untouched
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing spec/work-units.json reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "spec/work-units.json not found. Run fspec init first."
