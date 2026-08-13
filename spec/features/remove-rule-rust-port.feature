@done
@RPC-279
Feature: Port remove-rule command to Rust
  """
  Core impl at rust/fspec-core/src/commands/remove_rule.rs. Reuses io::ensure::ensure_work_units_file (auto-creates),
  io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now (timestamps). The rules array lives in
  WorkUnit.extra and is mutated in place — soft-delete sets `deleted=true` plus `deletedAt`, never removes the entry.
  Two-front-doors: bridge marshals JSON {workUnitId, index} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `remove-rule` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both soft-delete a rule by stable ID without falling back to the TS implementation

  Scenario: Soft-deletes a rule by stable id and bumps remainingCount
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'},{id:1,text:'r1',deleted:false,createdAt:'x'}]
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And the returned data contains removedRule='r0'
    And the returned data contains remainingCount=1
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true
    And spec/work-units.json on disk shows AUTH-001.rules[0].deletedAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Already-deleted rule is idempotent and does not write to disk
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:true,createdAt:'x',deletedAt:'x'}]
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And the returned data contains removedRule='r0'
    And the returned data contains remainingCount=0
    And the returned data contains message='Item ID 0 already deleted'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Stable-id semantics — id 1 is removed regardless of array position
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:true,createdAt:'x',deletedAt:'x'},{id:1,text:'r1',deleted:false,createdAt:'x'}]
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns success=true
    And the returned data contains removedRule='r1'
    And the returned data contains remainingCount=0
    And spec/work-units.json on disk shows AUTH-001.rules[1].id=1 with deleted=true

  Scenario: Unknown rule id surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=99
    Then the dispatcher returns success=false
    And the error message contains the substring 'Rule with ID 99 not found'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Empty rules array surfaces the canonical 'no rules' error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no rules field
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring 'Work unit AUTH-001 has no rules'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only NOT-001 status=specifying
    When I dispatch remove-rule with workUnitId='NOPE-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Non-specifying status is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only remove rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents
