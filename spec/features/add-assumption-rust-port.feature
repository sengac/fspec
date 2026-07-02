@done
@RPC-169
Feature: Port add-assumption command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_assumption.rs. Reuses io::ensure::ensure_work_units_file,
  io::locked_file::write_json_atomic, io::time::iso8601_now. The assumptions array lives in WorkUnit.extra
  as Vec<String> (plain strings, NOT a stable-id item shape). Two-front-doors: bridge marshals JSON
  {workUnitId, assumption} only.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-assumption` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both append assumption strings to a work unit during specification without falling back to the TS implementation

  Scenario: First add seeds assumptions array on a clean specifying work unit
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no assumptions field
    When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Users have valid email addresses'
    Then the dispatcher returns success=true
    And the returned data contains assumptionCount=1
    And spec/work-units.json on disk shows AUTH-001.assumptions[0]='Users have valid email addresses'
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Second add preserves insertion order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and assumptions=['A1']
    When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='A2'
    Then the dispatcher returns success=true
    And the returned data contains assumptionCount=2
    And spec/work-units.json on disk shows AUTH-001.assumptions=['A1', 'A2']

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-assumption with workUnitId='NOPE-001' and assumption='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Non-specifying status is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
