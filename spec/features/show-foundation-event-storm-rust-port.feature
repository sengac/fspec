@done
@event-storming
@cli
@RPC-306
Feature: Port show-foundation-event-storm command to Rust

  """
  Reads spec/foundation.json directly (no shared ensure_foundation_file helper exists yet) — port-local helper read_foundation_or_error in commands/show_foundation_event_storm.rs returning FspecCoreError::Io or ParseJson as needed. ENOENT must NOT auto-create.
  Uses serde_json::Value to model EventStorm items because the TS discriminated union has many shapes and the command only inspects type/text/deleted/boundedContextId/id — full struct modelling is unnecessary for read-only filtering. Output JSON is the raw item values from foundation.json (round-trip preserving).
  Dispatcher returns {success, data, message?} envelope; CLI prints JSON.stringify(data, null, 2) to stdout on success (exit 0) and 'Error: <message>' to stderr on failure (exit 1). Two-front-doors: core fn returns a String already; bridge marshals --type/--context from CliArgs to JSON.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-foundation-event-storm to display foundation Event Storm artifacts as JSON filtered optionally by item type or bounded context name
    So that I can inspect strategic-level Event Storm in foundation.json without launching Node, sharing one Rust source of truth between the LLM dispatcher and the CLI

  Scenario: Missing foundation.json surfaces an error
    Given an empty temp project root with no spec/ subdirectory
    When I dispatch show-foundation-event-storm with no arguments
    Then the dispatcher returns success=false
    And the error field contains the substring 'foundation.json'

  Scenario: foundation.json with no eventStorm field returns empty data and message
    Given spec/foundation.json exists without an eventStorm field
    When I dispatch show-foundation-event-storm with no arguments
    Then the dispatcher returns success=true
    And the data field is an empty JSON array
    And the message field equals 'No Event Storm data in foundation.json'

  Scenario: Soft-deleted items are filtered out
    Given spec/foundation.json contains eventStorm.items with three active items and one item where deleted=true
    When I dispatch show-foundation-event-storm with no arguments
    Then the dispatcher returns success=true
    And the data field is a JSON array with exactly 3 items
    And no returned item has deleted=true

  Scenario: Filtering by type returns only matching items
    Given spec/foundation.json contains eventStorm.items with two aggregates, one bounded_context, and one event
    When I dispatch show-foundation-event-storm with type='aggregate'
    Then the dispatcher returns success=true
    And the data field contains exactly 2 items
    And every returned item has type='aggregate'

  Scenario: Filtering by context name returns the bounded context plus linked items
    Given spec/foundation.json contains a bounded_context with id=1 and text='Work Management' plus three items where boundedContextId=1 and two items where boundedContextId=2
    When I dispatch show-foundation-event-storm with context='Work Management'
    Then the dispatcher returns success=true
    And the data field contains exactly 4 items
    And one returned item has type='bounded_context' and text='Work Management'
    And every other returned item has boundedContextId=1

  Scenario: Filtering by an unknown context name returns an empty array
    Given spec/foundation.json contains a bounded_context with text='Work Management' and three items linked to it
    When I dispatch show-foundation-event-storm with context='Nonexistent'
    Then the dispatcher returns success=true
    And the data field is an empty JSON array

  Scenario: Combined context and type filters compose
    Given spec/foundation.json contains a bounded_context id=1 'Work Management' with two aggregates and one event linked to it plus one aggregate linked to a different bounded context
    When I dispatch show-foundation-event-storm with context='Work Management' and type='aggregate'
    Then the dispatcher returns success=true
    And the data field contains exactly 2 items
    And every returned item has type='aggregate' and boundedContextId=1

  Scenario: Shared infrastructure module is registered for show-foundation-event-storm
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/commands/show_foundation_event_storm.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes show-foundation-event-storm to the new run function
