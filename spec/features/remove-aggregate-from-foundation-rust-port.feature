@done
@RPC-266
@rust
@cli
@mutation
Feature: Port remove-aggregate-from-foundation command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/remove_aggregate_from_foundation.rs uses
  crate::io::ensure::ensure_foundation_file to load (or auto-create) spec/foundation.json,
  locates the bounded_context Event Storm item (type='bounded_context', matching text,
  deleted=false), then locates the aggregate within that context (type='aggregate', matching
  text, deleted=false, boundedContextId equal to the bounded context id) and soft-deletes it
  by setting its deleted flag to true. The item remains in eventStorm.items. Persistence uses
  crate::io::locked_file::write_json_atomic so other top-level fields round-trip losslessly.

  Framing A divergence — FOUNDATION.md regeneration: generate-foundation-md (RPC-233) is
  itself unported, so the Rust core does NOT regenerate spec/FOUNDATION.md.

  Args shape (camelCase JSON): { contextName: String, aggregateName: String }.

  Two-front-doors: clap CLI and LLM dispatcher both call
  commands::remove_aggregate_from_foundation::run(args_json, project_root).

  SHARED-FILE REQUEST: dispatch.rs (supervisor-owned) currently calls
  remove_aggregate_from_foundation::run(args_json) — it must be updated to
  run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the remove-aggregate-from-foundation command that matches the TypeScript behaviour exactly
    So that the standalone fspec Rust binary can remove Big Picture Event Storm aggregates without depending on Node.js

  Scenario: Dispatcher soft-deletes an existing aggregate
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And the aggregate 'Order' in eventStorm.items has deleted=true
    And the aggregate 'Order' item still exists in eventStorm.items

  Scenario: Dispatcher rejects when foundation.json has no eventStorm section
    Given spec/foundation.json exists with no eventStorm section
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Sales' not found (no Event Storm data)"

  Scenario: Dispatcher rejects a non-existent bounded context
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    When I dispatch remove-aggregate-from-foundation with contextName='Unknown' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Unknown' not found"

  Scenario: Dispatcher rejects a non-existent aggregate within an existing context
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Aggregate 'Ghost' not found in bounded context 'Sales'"

  Scenario: Dispatcher treats an already soft-deleted aggregate as not found
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' that is already deleted=true
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Aggregate 'Order' not found in bounded context 'Sales'"

  Scenario: Dispatcher treats an already soft-deleted bounded context as not found
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 that is already deleted=true
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Sales' not found"

  Scenario: Dispatcher only removes the aggregate scoped to the named context
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and 'Billing' with id=1, each with an aggregate 'Order'
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And the aggregate 'Order' with boundedContextId=0 has deleted=true
    And the aggregate 'Order' with boundedContextId=1 still has deleted=false

  Scenario: Dispatcher preserves unknown top-level fields on write
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0, an aggregate 'Order', and a custom top-level 'experiments' key
    When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-aggregate-from-foundation with no contextName field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command remove-aggregate-from-foundation'
