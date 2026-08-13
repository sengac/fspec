@done
@RPC-271
Feature: Port remove-dependency command to Rust
  """
  Core impl file: rust/fspec-core/src/commands/remove_dependency.rs — replaces NotYetPorted stub. Public signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`. Reuses existing infrastructure: io::ensure::ensure_work_units_file, io::locked_file::write_json_atomic, types::work_unit::{WorkUnitsData, WorkUnit} with #[serde(flatten)] extra map (blocks/blockedBy/dependsOn/relatesTo arrays live in `extra`).
  Removal semantics: filter-then-delete-when-empty on the source array; bidirectional cleanup for blocks/blockedBy/relatesTo by mirroring the same op on the target. NO status transitions, NO state-array mutations, NO cycle detection. updatedAt bumped on source only. Single atomic write at end via write_json_atomic.
  Args shape (JSON): `{workUnitId, blocks?, blockedBy?, dependsOn?, relatesTo?}` — singular string fields (NOT arrays like add-dependencies). Returned data: `{success:true}`.
  Two-front-doors invariant: both dispatcher path and CLI bridge call commands::remove_dependency::run(args_json, project_root). Dispatcher accepts all-empty relationship args as a silent no-op (returns success:true). The CLI bridge is the ONLY place the at-least-one guard fires (matches TS where this check lives in the Commander action handler, not in the core function).
  """

  Background: User Story
    As a fspec maintainer working on RPC-003 Rust port
    I want to have the Rust remove-dependency command as a parity port of the TypeScript implementation
    So that both the LLM-facing dispatcher and the standalone Rust CLI can remove dependency relationships (blocks/blockedBy/dependsOn/relatesTo) atomically with bidirectional cleanup, without falling back to the TS-only implementation

  Scenario: Removing a blocks edge cleans both source and target arrays
    Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['AUTH-002'] and AUTH-002.blockedBy=['AUTH-001']
    When I dispatch remove-dependency with workUnitId='AUTH-001' and blocks='AUTH-002'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 has no blocks field
    And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Removing a blockedBy edge cleans both source and target arrays
    Given a project root tempdir with spec/work-units.json where UI-001.blockedBy=['API-001'] and API-001.blocks=['UI-001']
    When I dispatch remove-dependency with workUnitId='UI-001' and blockedBy='API-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-001 has no blockedBy field
    And spec/work-units.json on disk shows API-001 has no blocks field

  Scenario: Removing a dependsOn edge is unidirectional and leaves the target untouched
    Given a project root tempdir with spec/work-units.json where DASH-001.dependsOn=['AUTH-001', 'AUTH-002']
    When I dispatch remove-dependency with workUnitId='DASH-001' and dependsOn='AUTH-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows DASH-001.dependsOn=['AUTH-002']
    And spec/work-units.json on disk shows AUTH-001 has no blocks, no blockedBy, no dependsOn, no relatesTo fields

  Scenario: Removing a relatesTo edge cleans both sides of the symmetric link
    Given a project root tempdir with spec/work-units.json where AUTH-002.relatesTo=['AUTH-003'] and AUTH-003.relatesTo=['AUTH-002']
    When I dispatch remove-dependency with workUnitId='AUTH-002' and relatesTo='AUTH-003'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-002 has no relatesTo field
    And spec/work-units.json on disk shows AUTH-003 has no relatesTo field

  Scenario: Filtering removes the target id while preserving sibling entries
    Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['AUTH-002', 'AUTH-003'] and AUTH-002.blockedBy=['AUTH-001'] and AUTH-003.blockedBy=['AUTH-001']
    When I dispatch remove-dependency with workUnitId='AUTH-001' and blocks='AUTH-002'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.blocks=['AUTH-003']
    And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    And spec/work-units.json on disk shows AUTH-003.blockedBy=['AUTH-001']

  Scenario: Removing a non-existent dependency is a silent no-op
    Given a project root tempdir with spec/work-units.json where AUTH-001 has no blocks field at all
    When I dispatch remove-dependency with workUnitId='AUTH-001' and blocks='NOPE-999'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 still has no blocks field

  Scenario: Removing an edge whose target does not exist still updates the source array
    Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['GONE-999'] and no GONE-999 work unit exists
    When I dispatch remove-dependency with workUnitId='AUTH-001' and blocks='GONE-999'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 has no blocks field

  Scenario: Removal does not auto-revert a blocked status
    Given a project root tempdir with spec/work-units.json where UI-001.status='blocked' and UI-001.blockedBy=['API-001'] and states.blocked contains 'UI-001'
    When I dispatch remove-dependency with workUnitId='UI-001' and blockedBy='API-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-001.status='blocked'
    And spec/work-units.json on disk shows states.blocked still contains 'UI-001'
    And spec/work-units.json on disk shows states arrays for backlog, specifying, testing, implementing, validating, done are unchanged

  Scenario: Only the source updatedAt is bumped, target updatedAt is preserved
    Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['AUTH-002'], AUTH-002.blockedBy=['AUTH-001'], and AUTH-002.updatedAt='2025-01-01T00:00:00.000Z'
    When I dispatch remove-dependency with workUnitId='AUTH-001' and blocks='AUTH-002'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-002.updatedAt='2025-01-01T00:00:00.000Z'
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp later than 2025-01-01

  Scenario: Missing source work unit returns the canonical error and writes nothing
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I dispatch remove-dependency with workUnitId='NOPE-001' and dependsOn='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: All-empty relationship args is a silent no-op success
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I dispatch remove-dependency with workUnitId='AUTH-001' and no relationship args
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 with no blocks, no blockedBy, no dependsOn, no relatesTo fields

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-dependency with workUnitId='AUTH-001' and dependsOn='AUTH-002'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
