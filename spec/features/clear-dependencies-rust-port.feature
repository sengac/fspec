@done
@RPC-204
Feature: Port clear-dependencies command to Rust

  """
  Core impl file: codelet/fspec-core/src/commands/clear_dependencies.rs — replaces NotYetPorted stub. Public signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (parity with remove_dependency::run).
  Reuses existing shared infrastructure: io::ensure::ensure_work_units_file (load-or-init), io::locked_file::write_json_atomic (atomic write), types::work_unit::WorkUnitsData with #[serde(flatten)] extra map preserving unknown fields (blocks, blockedBy, dependsOn, relatesTo live in `extra`).
  Iteration order: blocks → blockedBy → dependsOn → relatesTo (mirrors src/commands/clear-dependencies.ts:40-87). Bidirectional cleanup filters reverse-edge entries and deletes the reverse field if empty.
  Persistence strategy: load via ensure_work_units_file, apply all mutations in memory, then a SINGLE write_json_atomic at the end. Mirrors fileManager.transaction() in TS.
  NO status change, NO state-array mutation, NO cycle detection (removal cannot create cycles). Source updatedAt is bumped; target updatedAt is NOT.
  Two-front-doors: dispatcher and clap CLI both call commands::clear_dependencies::run(args_json, project_root). CLI bridge marshals clap CliArgs { work_unit_id, confirm } to canonical JSON {workUnitId, confirm}.
  """

  Background: User Story
    As a fspec maintainer porting the TypeScript implementation to Rust
    I want to port the `clear-dependencies` command to fspec-core with two-front-doors parity
    So that shell users and the LLM dispatcher share a single canonical implementation that atomically removes every dependency edge from a work unit

  Scenario: Missing confirm flag fails before any file IO
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with blocks=['AUTH-002']
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=false
    Then the dispatcher returns success=false
    And the error message contains the substring 'Must confirm clearing all dependencies with --confirm flag'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing source work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I dispatch clear-dependencies with workUnitId='UNKNOWN-001' and confirm=true
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'UNKNOWN-001' does not exist"

  Scenario: Mixed blocks and dependsOn are removed with bidirectional cleanup on blocks
    Given a project root tempdir with spec/work-units.json containing AUTH-001 with blocks=['AUTH-002'] dependsOn=['API-001'], AUTH-002 with blockedBy=['AUTH-001'], and API-001
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 has no blocks field and no dependsOn field
    And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    And spec/work-units.json on disk shows API-001 has no blocks field and no blockedBy field
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: relatesTo edges are symmetrically dropped from both sides
    Given a project root tempdir with spec/work-units.json where AUTH-001.relatesTo=['UI-001', 'UI-002'], UI-001.relatesTo=['AUTH-001'], UI-002.relatesTo=['AUTH-001', 'OTHER-001']
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 has no relatesTo field
    And spec/work-units.json on disk shows UI-001 has no relatesTo field
    And spec/work-units.json on disk shows UI-002.relatesTo=['OTHER-001']

  Scenario: Clearing never changes a blocked work unit's status or state array
    Given a project root tempdir with spec/work-units.json where AUTH-001 has status='blocked' blockedBy=['API-001'] and states.blocked=['AUTH-001']
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.status='blocked'
    And spec/work-units.json on disk shows states.blocked still contains 'AUTH-001'
    And spec/work-units.json on disk shows states.backlog does NOT contain 'AUTH-001'

  Scenario: Reverse-edge cleanup is silently skipped when the target is missing
    Given a project root tempdir with spec/work-units.json where AUTH-001 has blocks=['GHOST-999'] and GHOST-999 does not exist as a work unit
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 has no blocks field
    And no error is raised for the missing GHOST-999 work unit

  Scenario: No dependency arrays still succeeds and only bumps updatedAt
    Given a project root tempdir with spec/work-units.json containing AUTH-001 with no blocks, blockedBy, dependsOn, or relatesTo fields
    When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001 still has no blocks, blockedBy, dependsOn, or relatesTo fields
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: blockedBy clearing reverse-removes source from each target's blocks array
    Given a project root tempdir with spec/work-units.json where UI-001.blockedBy=['API-001', 'DB-001'], API-001.blocks=['UI-001'], DB-001.blocks=['UI-001', 'UI-002']
    When I dispatch clear-dependencies with workUnitId='UI-001' and confirm=true
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-001 has no blockedBy field
    And spec/work-units.json on disk shows API-001 has no blocks field
    And spec/work-units.json on disk shows DB-001.blocks=['UI-002']
