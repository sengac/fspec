@done
@RPC-175
Feature: Port add-command-to-foundation command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_command_to_foundation.rs — Rust parity port of
  src/commands/add-command-to-foundation.ts. Appends a `command` Event Storm item to
  spec/foundation.json's eventStorm.items array, linked to a named bounded context via
  boundedContextId. Loads foundation.json via io::ensure::ensure_foundation_file (auto-creates the
  generic schema v2.0.0 default when missing), mutates a round-tripped serde_json::Value to preserve
  unknown top-level keys and field order, then writes atomically via io::locked_file::write_json_atomic.
  createdAt uses io::time::iso8601_now.

  The target bounded context is matched by type='bounded_context' AND text===contextName (no !deleted
  filter on add). Missing context → "Bounded context '<contextName>' not found" and foundation.json is
  left byte-equal. If eventStorm is absent it is seeded {level:'big_picture', items:[], nextItemId:1}
  BEFORE the lookup (so a seeded-but-empty foundation always fails not-found).

  The command item is shaped, in TS object-literal insertion order (serde_json::Map preserve_order):
  {id, type:'command', text, boundedContextId, color:'blue', deleted:false, createdAt, [description]}.
  NOTE: color is the JSON string 'blue' (NOT null, unlike bounded_context items). id = current
  nextItemId, then nextItemId is post-incremented. The optional --description maps to a trailing
  `description` field. Dispatcher result is {success:true, message:'Added command "<commandName>" to
  "<contextName>" bounded context'}. Two-front-doors: the CLI bridge marshals JSON
  {contextName, commandName, description?} only. DIVERGENCE: FOUNDATION.md regeneration is skipped per
  the add_diagram (RPC-178) precedent.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the add-command-to-foundation command ported to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both append a command Event Storm item linked to a foundation bounded context without falling back to the TS implementation

  Scenario: Adding a command to an existing bounded context appends the item and increments nextItemId
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=true
    And the returned message is 'Added command "CreateWorkUnit" to "Work Management" bounded context'
    And spec/foundation.json on disk shows eventStorm.nextItemId=2
    And spec/foundation.json on disk shows the appended item has type='command', text='CreateWorkUnit', boundedContextId=0, id=1, deleted=false
    And the appended item createdAt is a fresh ISO-8601 timestamp

  Scenario: The color field is persisted as the JSON string 'blue'
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the appended item color='blue' (a JSON string, not null)

  Scenario: Optional description is persisted as the trailing field in TS key order
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-command-to-foundation with contextName='Work Management', commandName='CreateWorkUnit', description='Creates a work unit'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the appended item description='Creates a work unit'
    And the appended item JSON key order is id, type, text, boundedContextId, color, deleted, createdAt, description

  Scenario: Adding a command to a non-existent bounded context fails and leaves the file unchanged
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-command-to-foundation with contextName='Nope' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: A command links only to the matching context and a second add increments nextItemId
    Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0 and bounded_context text='Specification' id=1 and nextItemId=2
    When I dispatch add-command-to-foundation with contextName='Specification' and commandName='CreateFeature'
    Then the dispatcher returns success=true
    And the appended item has boundedContextId=1 and id=2
    When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=true
    And the second appended item has boundedContextId=0 and id=3
    And spec/foundation.json on disk shows eventStorm.nextItemId=4
    And both command items are present in eventStorm.items

  Scenario: A foundation with no event storm reports the canonical not-found error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Work Management' not found"
