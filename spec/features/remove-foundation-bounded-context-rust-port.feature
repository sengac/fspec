@done
@RPC-274
Feature: Port remove-foundation-bounded-context command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/remove_foundation_bounded_context.rs. Signature
  run(args_json, project_root). Args struct {contextName: String, cascade: Option<bool>} (serde
  camelCase). Operates on spec/foundation.json's top-level eventStorm sub-object. Loads foundation
  via crate::io::ensure::ensure_foundation_file, round-trips the whole document as a
  serde_json::Value (preserve_order), mutates in place, and writes via
  crate::io::locked_file::write_json_atomic ONLY on success (error paths leave the file untouched).

  Removal is a soft-delete: the matched item's `deleted` field is set to true; the item is never
  physically spliced out. The target is the first item with type='bounded_context',
  text==contextName, and deleted=false. Children are non-deleted items carrying a
  boundedContextId == the target id. If children exist and --cascade is not set the command refuses
  with "Bounded context '<name>' has <n> child items. Use --cascade to remove the context and all
  its children." With --cascade the context AND all its non-deleted children are soft-deleted.

  Error cases: no eventStorm → "Bounded context '<name>' not found (no Event Storm data)";
  no match → "Bounded context '<name>' not found". Dispatcher success result is
  {success: true, message: 'Removed bounded context "<name>"<cascadeMsg> from foundation Event Storm'}
  where cascadeMsg = ' and all its children' when --cascade is set, otherwise empty.

  Framing A divergence: the Rust core does NOT regenerate spec/FOUNDATION.md (generate-foundation-md
  is an unported stub, RPC-233); the CLI bridge prints the regeneration parity line. Two-front-doors:
  the bridge marshals JSON {contextName, cascade?} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `remove-foundation-bounded-context` command ported to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both soft-delete a bounded_context (optionally cascading to its children) from the foundation-level Big Picture Event Storm without falling back to the TS implementation

  Scenario: Remove a childless bounded context soft-deletes it
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false and no children
    When I dispatch remove-foundation-bounded-context with contextName='Identity'
    Then the dispatcher returns success=true
    And the returned data contains message='Removed bounded context "Identity" from foundation Event Storm'
    And spec/foundation.json on disk shows the 'Identity' bounded_context item has deleted=true
    And the 'Identity' bounded_context item still exists in eventStorm.items (soft-delete, not spliced)

  Scenario: Refuse to remove a non-empty bounded context without --cascade
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    When I dispatch remove-foundation-bounded-context with contextName='Sales'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Remove a non-empty bounded context with --cascade soft-deletes context and children
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    When I dispatch remove-foundation-bounded-context with contextName='Sales' and cascade=true
    Then the dispatcher returns success=true
    And the returned data contains message='Removed bounded context "Sales" and all its children from foundation Event Storm'
    And spec/foundation.json on disk shows the 'Sales' bounded_context item has deleted=true
    And spec/foundation.json on disk shows both child items have deleted=true

  Scenario: Removing a name with no matching non-deleted bounded context errors
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false
    When I dispatch remove-foundation-bounded-context with contextName='Nope'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: An already soft-deleted bounded context is treated as not found
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Legacy' deleted=true
    When I dispatch remove-foundation-bounded-context with contextName='Legacy'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Legacy' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing against a foundation with no eventStorm field errors
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I dispatch remove-foundation-bounded-context with contextName='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Bounded context 'Anything' not found (no Event Storm data)"
