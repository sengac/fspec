@done
@RPC-270
Feature: Port remove-command-from-foundation command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/remove_command_from_foundation.rs — Rust parity port of
  src/commands/remove-command-from-foundation.ts. Soft-deletes (sets deleted=true) a `command` Event
  Storm item in spec/foundation.json's eventStorm.items array; the item is NOT spliced out. Loads
  foundation.json via io::ensure::ensure_foundation_file, mutates a round-tripped serde_json::Value to
  preserve unknown top-level keys and field order, then writes atomically via
  io::locked_file::write_json_atomic.

  Validation order (parity with TS):
  1. eventStorm absent → "Bounded context '<contextName>' not found (no Event Storm data)" (no write).
  2. bounded_context matched by type='bounded_context' AND text===contextName AND !deleted; absent →
     "Bounded context '<contextName>' not found".
  3. command matched by type='command' AND text===commandName AND !deleted AND
     boundedContextId===context.id; absent → "Command '<commandName>' not found in bounded context
     '<contextName>'".
  An already soft-deleted command is therefore treated as not-found (operation is non-idempotent on a
  second call). On match the item's `deleted` flag is set to true; all other items/fields are
  preserved. Dispatcher result is {success:true, message:'Removed command "<commandName>" from
  "<contextName>" bounded context'}. Two-front-doors: the CLI bridge marshals JSON
  {contextName, commandName} only. DIVERGENCE: FOUNDATION.md regeneration skipped per the add_diagram
  (RPC-178) precedent.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the remove-command-from-foundation command ported to Rust as a parity port
    So that both front doors can soft-delete an Event Storm command from a foundation bounded context without falling back to the TS implementation

  Scenario: Removing an existing command soft-deletes it and returns the success message
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0 deleted=false
    When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=true
    And the returned message is 'Removed command "CreateWorkUnit" from "Work Management" bounded context'
    And spec/foundation.json on disk shows the CreateWorkUnit command item deleted=true
    And the bounded_context item and all other items are unchanged

  Scenario: Removing when the foundation has no event storm reports the no-data error and leaves the file unchanged
    Given a project root tempdir with spec/foundation.json that has no eventStorm field
    When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Work Management' not found (no Event Storm data)"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing from a non-existent bounded context fails with the context not-found error
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0
    When I dispatch remove-command-from-foundation with contextName='Nope' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing a command name that does not exist in the matched context fails
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0
    When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='Ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring "Command 'Ghost' not found in bounded context 'Work Management'"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: A command belonging to a different context id is not matched
    Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0, bounded_context text='Specification' id=1, and a command text='CreateFeature' boundedContextId=1
    When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateFeature'
    Then the dispatcher returns success=false
    And the error message contains the substring "Command 'CreateFeature' not found in bounded context 'Work Management'"

  Scenario: Removing an already soft-deleted command fails as not-found
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0 deleted=true
    When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=false
    And the error message contains the substring "Command 'CreateWorkUnit' not found in bounded context 'Work Management'"
