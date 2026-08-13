@done
@event-storm
@cli
@RPC-180
Feature: Port add-domain-event-to-foundation command to Rust
  """
  Core impl at rust/fspec-core/src/commands/add_domain_event_to_foundation.rs — Rust parity port of src/commands/add-domain-event-to-foundation.ts. COPY SHAPE of add_command_to_foundation.rs (RPC-175) twin: load foundation.json via read_or_init_json with the TS inline minimal default (version 2.0.0/project/problemSpace/solutionSpace), seed eventStorm {level:'big_picture',items:[],nextItemId:1} if absent, find bounded_context by type+text (no !deleted filter), append item in key order id,type,text,boundedContextId,color,deleted,createdAt,[description] then write_json_atomic. KEY DIFFS vs twin: type='event' (not 'command'), color='orange' (not 'blue'), 2nd positional eventName, message noun 'domain event'.
  CLI bridge rust/fspec/src/add_domain_event_to_foundation.rs marshals JSON {contextName, eventName, description?} only and forwards to commands::add_domain_event_to_foundation::run — NO domain logic. Stdout success '✓ Added domain event ...'; stderr failure 'Error: <message>' exit 1. DIVERGENCE: none — FOUNDATION.md is regenerated after the write (matches the event-storm twins add_command_to_foundation.rs RPC-175 / remove_command_from_foundation.rs RPC-270; supervisor ruling 2026-06-13).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/foundation.json via read_or_init_json with the TS inline minimal default (version/project/problemSpace/solutionSpace)
  #   2. If eventStorm is absent it is seeded {level:'big_picture', items:[], nextItemId:1} before the bounded context lookup
  #   3. Bounded context is matched by type='bounded_context' AND text===contextName (no !deleted filter on the add path)
  #   4. A missing bounded context returns "Bounded context '<contextName>' not found" and leaves foundation.json byte-equal (no write)
  #   5. On success a domain event item is appended with id=nextItemId, type='event', color='orange', deleted=false, createdAt=fresh ISO-8601, in TS key order id,type,text,boundedContextId,color,deleted,createdAt,[description]
  #   6. nextItemId is post-incremented after each add; the optional --description maps to a trailing description field
  #
  # EXAMPLES:
  #   1. Adding a command to an existing bounded context appends an item with type='event', color='orange', id=nextItemId, deleted=false then post-increments nextItemId
  #   2. A missing bounded context returns "Bounded context 'Nope' not found" and leaves foundation.json byte-equal (no write)
  #   3. Optional --description is persisted as a trailing field after createdAt in TS key order
  #   4. CLI: fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated" exits 0 and stdout shows '✓ Added domain event "WorkUnitCreated" to "Work Management" bounded context'
  #   5. Item color is the JSON string 'orange' (not blue, not null)
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the add-domain-event-to-foundation command to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both append an event Event Storm item linked to a foundation bounded context without falling back to the TS implementation

  Scenario: Adding a domain event to an existing bounded context appends the item and increments nextItemId
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=true
    And the returned message is 'Added domain event "WorkUnitCreated" to "Work Management" bounded context'
    And spec/foundation.json on disk shows eventStorm.nextItemId=2
    And spec/foundation.json on disk shows the appended item has type='event', text='WorkUnitCreated', boundedContextId=0, id=1, deleted=false
    And the appended item createdAt is a fresh ISO-8601 timestamp

  Scenario: The color field is persisted as the JSON string 'orange'
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the appended item color='orange' (a JSON string, not blue, not null)

  Scenario: Optional description is persisted as the trailing field in TS key order
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-domain-event-to-foundation with contextName='Work Management', eventName='WorkUnitCreated', description='Signals work unit reached done status'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the appended item description='Signals work unit reached done status'
    And the appended item JSON key order is id, type, text, boundedContextId, color, deleted, createdAt, description

  Scenario: Adding a domain event to a non-existent bounded context fails and leaves the file unchanged
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-domain-event-to-foundation with contextName='Nope' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: A domain event links only to the matching context and a second add increments nextItemId
    Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0 and bounded_context text='Specification' id=1 and nextItemId=2
    When I dispatch add-domain-event-to-foundation with contextName='Specification' and eventName='FeatureCreated'
    Then the dispatcher returns success=true
    And the appended item has boundedContextId=1 and id=2
    When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=true
    And the second appended item has boundedContextId=0 and id=3
    And spec/foundation.json on disk shows eventStorm.nextItemId=4
    And both event items are present in eventStorm.items

  Scenario: A foundation with no event storm reports the canonical not-found error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Work Management' not found"
