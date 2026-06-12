@done
@RPC-182
Feature: Port add-external-system command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/add_external_system.rs. Reuses the same event-storm
  shared-util semantics as the TS addEventStormItem (src/commands/event-storm-utils.ts). Validation
  checks the work-units.json file exists FIRST (missing → "spec/work-units.json not found. Run fspec
  init first." — it does NOT auto-create). The Event Storm items array lives in the work unit's
  `eventStorm` sub-object inside WorkUnit.extra (round-tripped via serde flatten). On first add the
  sub-object is seeded as {level: 'process_modeling', items: [], nextItemId: 0} matching the TS shared
  helper addEventStormItem. An external_system item is shaped (in TS object-literal insertion order):
  {type: 'external_system', color: 'pink', text, [integrationType], [timestamp], [boundedContext], id,
  deleted: false, createdAt}. Note: the CLI --type option maps to the item field `integrationType`.
  Validation rejects only done/blocked states (NOT specifying-only). The missing-work-unit error reads
  "Work unit <id> not found". Dispatcher result is {success: true, externalSystemId: <id>}.
  Two-front-doors: bridge marshals JSON {workUnitId, text, type?, timestamp?, boundedContext?} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-external-system` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append Event Storm external-system items to a work unit without falling back to the TS implementation

  Scenario: First add seeds the eventStorm sub-object on a clean work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-external-system with workUnitId='AUTH-001' and text='Payment Gateway'
    Then the dispatcher returns success=true
    And the returned data contains externalSystemId=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0] has type='external_system', color='pink', text='Payment Gateway', id=0, deleted=false
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp

  Scenario: The --type option maps to the integrationType item field
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-external-system with workUnitId='AUTH-001', text='Stripe API', type='REST_API'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].integrationType='REST_API'

  Scenario: Optional timestamp and boundedContext fields are persisted in TS insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-external-system with workUnitId='AUTH-001', text='User Database', type='DATABASE', timestamp=1000, boundedContext='Identity'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].timestamp=1000
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Identity'
    And the items[0] JSON key order is type, color, text, integrationType, timestamp, boundedContext, id, deleted, createdAt

  Scenario: Second add increments nextItemId and preserves insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with an existing eventStorm external_system id=0 and nextItemId=1
    When I dispatch add-external-system with workUnitId='AUTH-001' and text='RabbitMQ'
    Then the dispatcher returns success=true
    And the returned data contains externalSystemId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1] has id=1 and text='RabbitMQ'

  Scenario: Missing work unit surfaces the canonical not-found error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-external-system with workUnitId='NOPE-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit NOPE-001 not found"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Done state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I dispatch add-external-system with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Blocked state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    When I dispatch add-external-system with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in blocked state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing spec/work-units.json reports the canonical not-found error without creating the file
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-external-system with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "spec/work-units.json not found. Run fspec init first."
    And spec/work-units.json does not exist on disk
