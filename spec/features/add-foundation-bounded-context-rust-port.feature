@done
@RPC-183
Feature: Port add-foundation-bounded-context command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_foundation_bounded_context.rs. Signature
  run(args_json, project_root) mirroring add_diagram.rs. Operates on spec/foundation.json's
  top-level eventStorm sub-object at the big_picture level (NOT work-units.json). Uses
  crate::io::ensure::ensure_foundation_file (auto-creates the canonical generic schema v2.0.0
  default when missing) + crate::io::locked_file::write_json_atomic. The whole foundation
  document is round-tripped as a serde_json::Value so unknown top-level fields and existing key
  order are preserved byte-for-byte (the workspace builds serde_json with preserve_order).

  When eventStorm is absent it is seeded as {level: 'big_picture', items: [], nextItemId: 1} —
  note foundation starts nextItemId at 1, UNLIKE the work-unit event storm which starts at 0.
  The new bounded_context item is shaped in TS object-literal insertion order:
  {id, type: 'bounded_context', text, color: null, deleted: false, createdAt}. color is the JSON
  literal null (key present, not absent). The assigned id equals nextItemId before a
  post-increment (first add → id=1, nextItemId becomes 2).

  Framing A divergence: the Rust core does NOT regenerate spec/FOUNDATION.md (generate-foundation-md
  is an unported stub, RPC-233); the CLI bridge prints the regeneration parity line. Dispatcher
  result is {success: true, message: 'Added bounded context "<text>" to foundation Event Storm'}.
  Two-front-doors: the bridge marshals JSON {text} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-foundation-bounded-context` command ported to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both append a bounded_context item to the foundation-level Big Picture Event Storm without falling back to the TS implementation

  Scenario: First add seeds the eventStorm sub-object on a foundation with no eventStorm field
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I dispatch add-foundation-bounded-context with text='Order Management'
    Then the dispatcher returns success=true
    And the returned data contains message='Added bounded context "Order Management" to foundation Event Storm'
    And spec/foundation.json on disk shows eventStorm.level='big_picture'
    And spec/foundation.json on disk shows eventStorm.nextItemId=2
    And spec/foundation.json on disk shows eventStorm.items[0] has id=1, type='bounded_context', text='Order Management', deleted=false
    And spec/foundation.json on disk shows eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp

  Scenario: The color field is persisted as JSON null
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I dispatch add-foundation-bounded-context with text='Identity'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows eventStorm.items[0].color is JSON null (key present with null value)

  Scenario: The persisted item key order matches TS insertion order
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I dispatch add-foundation-bounded-context with text='Catalog'
    Then the dispatcher returns success=true
    And the eventStorm.items[0] JSON key order is exactly id, type, text, color, deleted, createdAt

  Scenario: Second add increments nextItemId and assigns the next id
    Given a project root tempdir with spec/foundation.json containing an existing eventStorm bounded_context id=1 and nextItemId=2
    When I dispatch add-foundation-bounded-context with text='Shipping'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows eventStorm.nextItemId=3
    And spec/foundation.json on disk shows eventStorm.items[1] has id=2 and text='Shipping'

  Scenario: Missing foundation.json is auto-created with the default schema before appending
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-foundation-bounded-context with text='Billing'
    Then the dispatcher returns success=true
    And spec/foundation.json exists on disk
    And spec/foundation.json on disk shows eventStorm.items[0] has id=1 and text='Billing'

  Scenario: Unknown top-level foundation fields are preserved byte-for-byte
    Given a project root tempdir with spec/foundation.json containing a custom top-level field extraField='keep-me' and no eventStorm field
    When I dispatch add-foundation-bounded-context with text='Payments'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk still contains extraField='keep-me'
    And spec/foundation.json on disk shows eventStorm.items[0].text='Payments'
