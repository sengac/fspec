@done
@RPC-172
Feature: Port add-bounded-context command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/add_bounded_context.rs. Reuses the same
  event-storm shared-util semantics as the TS addEventStormItem (src/commands/event-storm-utils.ts).
  Validation checks the work-units.json file exists FIRST (missing → "spec/work-units.json not
  found. Run fspec init first." — it does NOT auto-create). The Event Storm items array lives in
  the work unit's `eventStorm` sub-object inside WorkUnit.extra (round-tripped via serde flatten).
  On first add the sub-object is seeded as {level: 'process_modeling', items: [], nextItemId: 0}.
  A bounded_context item is shaped (in TS object-literal insertion order):
  {type: 'bounded_context', color: null, text, [description], [timestamp], [boundedContext], id,
  deleted: false, createdAt}. Note: color is JSON null (present, not absent). The CLI --description
  option maps to the item field `description`; --bounded-context maps to `boundedContext`. Validation
  rejects only done/blocked states. The missing-work-unit error reads "Work unit <id> not found".
  Dispatcher result is {success: true, boundedContextId: <id>}. Two-front-doors: bridge marshals
  JSON {workUnitId, text, description?, timestamp?, boundedContext?} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-bounded-context` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append Event Storm bounded-context items to a work unit without falling back to the TS implementation

  Scenario: First add seeds the eventStorm sub-object on a clean work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Order Management'
    Then the dispatcher returns success=true
    And the returned data contains boundedContextId=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0] has type='bounded_context', color=null, text='Order Management', id=0, deleted=false
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp

  Scenario: The color field is persisted as JSON null
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Identity'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].color is JSON null (key present with null value)

  Scenario: Optional description, timestamp and boundedContext fields are persisted in TS insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-bounded-context with workUnitId='AUTH-001', text='Inventory', description='Manages stock', timestamp=1000, boundedContext='Logistics'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].description='Manages stock'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].timestamp=1000
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Logistics'
    And the items[0] JSON key order is type, color, text, description, timestamp, boundedContext, id, deleted, createdAt

  Scenario: Second add increments nextItemId and preserves insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with an existing eventStorm bounded_context id=0 and nextItemId=1
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Shipping'
    Then the dispatcher returns success=true
    And the returned data contains boundedContextId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1] has id=1 and text='Shipping'

  Scenario: Missing work unit surfaces the canonical not-found error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-bounded-context with workUnitId='NOPE-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit NOPE-001 not found"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Done state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Blocked state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in blocked state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing spec/work-units.json reports the canonical not-found error without creating the file
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "spec/work-units.json not found. Run fspec init first."
    And spec/work-units.json does not exist on disk
