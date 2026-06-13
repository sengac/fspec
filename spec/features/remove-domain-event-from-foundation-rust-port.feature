@done
@event-storm
@cli
@RPC-272
Feature: Port remove-domain-event-from-foundation command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/remove_domain_event_from_foundation.rs — Rust parity port of src/commands/remove-domain-event-from-foundation.ts. COPY SHAPE of remove_command_from_foundation.rs (RPC-270) twin: load foundation.json via read_or_init_json with TS inline minimal default, error if no eventStorm field, find non-deleted bounded_context by text, find non-deleted matching item by boundedContextId, set deleted=true, write_json_atomic. KEY DIFFS vs twin: item type matched is 'event' (not 'command'), 2nd positional eventName, not-found noun 'Domain event' (not 'Command'), message noun 'domain event'.
  CLI bridge codelet/fspec/src/remove_domain_event_from_foundation.rs marshals JSON {contextName, eventName} only and forwards to commands::remove_domain_event_from_foundation::run — NO domain logic. Surface: fspec remove-domain-event-from-foundation <context-name> <event-name> (no options). Stdout success '✓ Removed domain event ...'; stderr failure 'Error: <message>' exit 1. FOUNDATION.md is regenerated after the write (matches the event-storm twins remove_command_from_foundation.rs RPC-270 / add_command_to_foundation.rs RPC-175; supervisor ruling 2026-06-13).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/foundation.json via read_or_init_json with the TS inline minimal default (version/project/problemSpace/solutionSpace)
  #   2. If the document has no eventStorm field, return "Bounded context '<contextName>' not found (no Event Storm data)" and leave foundation.json byte-equal (no write)
  #   3. Bounded context is matched by type='bounded_context' AND text===contextName AND !deleted; missing returns "Bounded context '<contextName>' not found"
  #   4. Domain event is matched by type='event' AND text===eventName AND !deleted AND boundedContextId===context.id; missing returns "Domain event '<eventName>' not found in bounded context '<contextName>'"
  #   5. On a match the domain event's deleted flag is set to true (soft-delete, not spliced); all other items/fields preserved; foundation.json written atomically
  #   6. An already soft-deleted domain event is treated as not-found (operation is non-idempotent on a second call)
  #   7. Dispatcher result is {success:true, message:'Removed domain event "<eventName>" from "<contextName>" bounded context'}
  #
  # EXAMPLES:
  #   1. Removing an existing non-deleted domain event soft-deletes it (deleted=true) and returns 'Removed domain event ... bounded context'
  #   2. Removing when foundation has no eventStorm field returns the no-data not-found error and leaves the file unchanged
  #   3. Removing an event name that does not exist in the matched context returns "Domain event 'Ghost' not found in bounded context 'Work Management'"
  #   4. A domain event belonging to a different context id is not matched (boundedContextId mismatch returns not-found)
  #   5. CLI: fspec remove-domain-event-from-foundation "Work Management" "WorkUnitCreated" exits 0 and stdout shows '✓ Removed domain event "WorkUnitCreated" from "Work Management" bounded context'; missing event exits 1 with 'Error:' prefix
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the remove-domain-event-from-foundation command to Rust as a parity port
    So that both front doors can soft-delete an Event Storm domain event from a foundation bounded context without falling back to the TS implementation

  Scenario: Removing an existing domain event soft-deletes it and returns the success message
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=false
    When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=true
    And the returned message is 'Removed domain event "WorkUnitCreated" from "Work Management" bounded context'
    And spec/foundation.json on disk shows the WorkUnitCreated event item deleted=true
    And the bounded_context item and all other items are unchanged

  Scenario: Removing when the foundation has no event storm reports the no-data error and leaves the file unchanged
    Given a project root tempdir with spec/foundation.json that has no eventStorm field
    When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Work Management' not found (no Event Storm data)"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing from a non-existent bounded context fails with the context not-found error
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    When I dispatch remove-domain-event-from-foundation with contextName='Nope' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing an event name that does not exist in the matched context fails
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='Ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Domain event 'Ghost' not found in bounded context 'Work Management'"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: A domain event belonging to a different context id is not matched
    Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0, bounded_context text='Specification' id=1, and an event text='FeatureCreated' boundedContextId=1
    When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='FeatureCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Domain event 'FeatureCreated' not found in bounded context 'Work Management'"

  Scenario: Removing an already soft-deleted domain event fails as not-found
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=true
    When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    Then the dispatcher returns success=false
    And the error message contains the substring "Domain event 'WorkUnitCreated' not found in bounded context 'Work Management'"
