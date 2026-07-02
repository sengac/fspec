@done
@RPC-166
@rust
@cli
@mutation
Feature: Port add-aggregate-to-foundation command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_aggregate_to_foundation.rs uses
  crate::io::ensure::ensure_foundation_file to load (or auto-create) spec/foundation.json
  (canonical generic schema v2.0.0), locates the target bounded_context Event Storm item by
  type='bounded_context' and matching text, and appends a new aggregate item to
  eventStorm.items linked via boundedContextId. The aggregate item is built in the TS
  insertion order id, type, text, boundedContextId, color, deleted, createdAt, [description]
  with the literal color 'yellow'. eventStorm.nextItemId is post-incremented (the TS seed
  value is { level: 'big_picture', items: [], nextItemId: 1 } when eventStorm is absent).
  Persistence uses crate::io::locked_file::write_json_atomic so other top-level fields
  round-trip losslessly.

  Framing A divergence — FOUNDATION.md regeneration: generate-foundation-md (RPC-233) is
  itself unported, so the Rust core does NOT regenerate spec/FOUNDATION.md. The CLI bridge
  prints the success line for stdout parity.

  Args shape (camelCase JSON): { contextName: String, aggregateName: String,
  description?: String }.

  Two-front-doors: clap CLI and LLM dispatcher both call
  commands::add_aggregate_to_foundation::run(args_json, project_root).

  SHARED-FILE REQUEST: dispatch.rs (supervisor-owned) currently calls
  add_aggregate_to_foundation::run(args_json) — it must be updated to
  run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the add-aggregate-to-foundation command that matches the TypeScript behaviour exactly
    So that the standalone fspec Rust binary can manage Big Picture Event Storm aggregates without depending on Node.js

  Scenario: Dispatcher appends a new aggregate to an existing bounded context
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And spec/foundation.json eventStorm.items contains exactly one aggregate item
    And that aggregate has type='aggregate', text='Order', color='yellow', and deleted=false
    And that aggregate has boundedContextId=0 matching the 'Sales' context
    And eventStorm.nextItemId has been incremented

  Scenario: Dispatcher assigns sequential ids and increments nextItemId per aggregate
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    And I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Shipment'
    Then spec/foundation.json eventStorm.items contains two aggregate items
    And the second aggregate has an id one greater than the first aggregate
    And eventStorm.nextItemId equals one greater than the second aggregate id

  Scenario: Dispatcher persists optional description when provided
    Given spec/foundation.json contains a bounded_context item 'Billing' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Billing' aggregateName='Invoice' description='Billing root'
    Then the dispatcher returns success=true
    And the aggregate 'Invoice' has description='Billing root'

  Scenario: Dispatcher omits the description field when no description is provided
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And the aggregate 'Order' has no 'description' field

  Scenario: Dispatcher rejects a non-existent bounded context
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Unknown' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Unknown' not found"

  Scenario: Dispatcher rejects when foundation.json has no eventStorm data
    Given spec/foundation.json exists with no eventStorm section
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Sales' not found"

  Scenario: Dispatcher auto-creates foundation.json but still fails when no bounded context exists
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    Then the file spec/foundation.json exists
    And the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Sales' not found"

  Scenario: Dispatcher links the aggregate to the correct context among multiple bounded contexts
    Given spec/foundation.json contains bounded_context items 'Sales' with id=0 and 'Billing' with id=1 in eventStorm.items
    When I dispatch add-aggregate-to-foundation with contextName='Billing' aggregateName='Invoice'
    Then the dispatcher returns success=true
    And the aggregate 'Invoice' has boundedContextId=1 matching the 'Billing' context

  Scenario: Dispatcher preserves unknown top-level fields on write
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 and a custom top-level 'experiments' key
    When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-aggregate-to-foundation with no contextName field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command add-aggregate-to-foundation'
