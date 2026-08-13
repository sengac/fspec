@done
@RPC-189
Feature: Port add-rule command to Rust
  """
  Core impl at rust/fspec-core/src/commands/add_rule.rs. Reuses io::ensure::ensure_work_units_file (auto-creates),
  io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now (timestamps). RuleItem and nextRuleId live
  in WorkUnit.extra (round-tripped via #[serde(flatten)]). Two-front-doors: bridge marshals JSON {workUnitId, rule} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-rule` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append rules to a work unit during Example Mapping without falling back to the TS implementation

  Scenario: First add seeds rules array and nextRuleId on a clean specifying work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no rules field
    When I dispatch add-rule with workUnitId='AUTH-001' and rule='Email must be valid format'
    Then the dispatcher returns success=true
    And the returned data contains ruleCount=1
    And spec/work-units.json on disk shows AUTH-001.rules[0].id=0
    And spec/work-units.json on disk shows AUTH-001.rules[0].text='Email must be valid format'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[0].createdAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.nextRuleId=1
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Second add appends with auto-incremented id
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with rules=[{id:0,text:'r1',deleted:false,createdAt:'x'}] and nextRuleId=1
    When I dispatch add-rule with workUnitId='AUTH-001' and rule='Password must be 8+ chars'
    Then the dispatcher returns success=true
    And the returned data contains ruleCount=2
    And spec/work-units.json on disk shows AUTH-001.rules has length 2
    And spec/work-units.json on disk shows AUTH-001.rules[1].id=1
    And spec/work-units.json on disk shows AUTH-001.rules[1].text='Password must be 8+ chars'
    And spec/work-units.json on disk shows AUTH-001.nextRuleId=2

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-rule with workUnitId='NOPE-001' and rule='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Non-specifying status is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I dispatch add-rule with workUnitId='AUTH-001' and rule='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-rule with workUnitId='AUTH-001' and rule='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
