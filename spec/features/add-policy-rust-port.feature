@done
@RPC-187
Feature: Port add-policy command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_policy.rs. Reuses io::ensure::ensure_work_units_file,
  io::locked_file::write_json_atomic, io::time::iso8601_now. The Event Storm items array lives in the
  work unit's `eventStorm` sub-object inside WorkUnit.extra (round-tripped via serde flatten). On first add
  the sub-object is seeded as {level: 'process_modeling', items: [], nextItemId: 0} matching the TS shared
  helper addEventStormItem (src/commands/event-storm-utils.ts). A policy item is shaped (in TS object-literal
  insertion order): {type: 'policy', color: 'purple', text, [when], [then], [timestamp], [boundedContext],
  id, deleted: false, createdAt}. Validation rejects only done/blocked states (NOT specifying-only). The
  missing-work-unit error reads "Work unit <id> not found" (NOT "does not exist"). Dispatcher result is
  {success: true, policyId: <id>}. Two-front-doors: bridge marshals JSON {workUnitId, text, when?, then?,
  timestamp?, boundedContext?} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-policy` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append Event Storm policy items to a work unit without falling back to the TS implementation

  Scenario: First add seeds the eventStorm sub-object on a clean work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-policy with workUnitId='AUTH-001' and text='Send welcome email'
    Then the dispatcher returns success=true
    And the returned data contains policyId=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0] has type='policy', color='purple', text='Send welcome email', id=0, deleted=false
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp

  Scenario: Optional when/then/boundedContext fields are persisted in TS insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-policy with workUnitId='AUTH-001', text='Send welcome email', when='UserRegistered', then='SendWelcomeEmail', boundedContext='Identity'
    Then the dispatcher returns success=true
    And the returned data contains policyId=0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].when='UserRegistered'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].then='SendWelcomeEmail'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Identity'
    And the items[0] JSON key order is type, color, text, when, then, boundedContext, id, deleted, createdAt

  Scenario: Optional timestamp field is persisted when provided
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    When I dispatch add-policy with workUnitId='AUTH-001', text='Send welcome email', timestamp=1000
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].timestamp=1000

  Scenario: Second add increments nextItemId and preserves insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with an existing eventStorm policy id=0 and nextItemId=1
    When I dispatch add-policy with workUnitId='AUTH-001' and text='Notify warehouse'
    Then the dispatcher returns success=true
    And the returned data contains policyId=1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1] has id=1 and text='Notify warehouse'

  Scenario: Missing work unit surfaces the canonical not-found error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-policy with workUnitId='NOPE-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit NOPE-001 not found"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Done state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Blocked state is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Cannot add Event Storm items to work unit in blocked state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing spec/work-units.json reports the canonical not-found error without creating the file
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "spec/work-units.json not found. Run fspec init first."
    And spec/work-units.json does not exist on disk
