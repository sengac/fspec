@done
@RPC-176
Feature: Port add-dependencies command to Rust
  """
  Core impl file: rust/fspec-core/src/commands/add_dependencies.rs — replaces NotYetPorted stub. Public signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (parity with list_work_units::run).
  Reuses existing shared infrastructure: io::ensure::ensure_work_units_file (load-or-init), io::locked_file::write_json_atomic (atomic write), types::work_unit::{WorkUnitsData, WorkUnit, WorkUnitStatus} with #[serde(flatten)] extra map preserving unknown fields (blocks, blockedBy, dependsOn, relatesTo, blockedReason all live in `extra`).
  Iteration order: blocks → blockedBy → dependsOn → relatesTo (mirrors src/commands/add-dependencies.ts:32-77). Within each array, original element order is preserved.
  Persistence strategy: load via ensure_work_units_file, apply all mutations in memory, then a SINGLE write_json_atomic at the end. This differs from the TS file-manager.transaction-per-call pattern but the observable end-state is identical for successful runs. On error, no partial writes occur (cleaner Rust semantic).
  Two-front-doors: dispatcher and clap CLI both call commands::add_dependencies::run(args_json, project_root). CLI bridge marshals clap flags into JSON object {workUnitId, dependencies: {blocks, blockedBy, dependsOn, relatesTo}}. NO logic in bridge — JSON marshalling only.
  """

  Background: User Story
    As a fspec maintainer working on RPC-003 Rust port
    I want to have the Rust `add-dependencies` command added as a parity port of the TypeScript implementation
    So that the LLM-facing dispatcher and the standalone Rust CLI can both apply multiple dependency relationships atomically without falling back to the TS-only implementation

  Scenario: Bulk blocks adds bidirectional edges and auto-transitions targets to blocked
    Given a project root tempdir with spec/work-units.json containing AUTH-001, AUTH-002, AUTH-003 all status=backlog with empty dependency arrays
    When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002', 'AUTH-003']
    Then the dispatcher returns success=true
    And the returned data contains added=2
    And spec/work-units.json on disk shows AUTH-001.blocks=['AUTH-002', 'AUTH-003']
    And spec/work-units.json on disk shows AUTH-002.blockedBy contains 'AUTH-001'
    And spec/work-units.json on disk shows AUTH-002.status='blocked'
    And spec/work-units.json on disk shows AUTH-003.blockedBy contains 'AUTH-001'
    And spec/work-units.json on disk shows AUTH-003.status='blocked'
    And spec/work-units.json on disk shows states.backlog does NOT contain 'AUTH-002' or 'AUTH-003'
    And spec/work-units.json on disk shows states.blocked contains 'AUTH-002' and 'AUTH-003'
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Bulk blockedBy auto-transitions the SOURCE to blocked with blockedReason
    Given a project root tempdir with spec/work-units.json containing UI-001 status=specifying and API-001 status=backlog
    When I dispatch add-dependencies with workUnitId='UI-001' and dependencies.blockedBy=['API-001']
    Then the dispatcher returns success=true
    And the returned data contains added=1
    And spec/work-units.json on disk shows UI-001.blockedBy=['API-001']
    And spec/work-units.json on disk shows API-001.blocks contains 'UI-001'
    And spec/work-units.json on disk shows UI-001.status='blocked'
    And spec/work-units.json on disk shows UI-001.blockedReason='Blocked by API-001'
    And spec/work-units.json on disk shows states.specifying does NOT contain 'UI-001'
    And spec/work-units.json on disk shows states.blocked contains 'UI-001'

  Scenario: Bulk dependsOn writes only the source array with no reverse edge or status change
    Given a project root tempdir with spec/work-units.json containing DASH-001 status=backlog, AUTH-001 status=backlog, AUTH-002 status=backlog
    When I dispatch add-dependencies with workUnitId='DASH-001' and dependencies.dependsOn=['AUTH-001', 'AUTH-002']
    Then the dispatcher returns success=true
    And the returned data contains added=2
    And spec/work-units.json on disk shows DASH-001.dependsOn=['AUTH-001', 'AUTH-002']
    And spec/work-units.json on disk shows AUTH-001 has no blocks, no blockedBy, no dependsOn, no relatesTo fields
    And spec/work-units.json on disk shows AUTH-002 has no blocks, no blockedBy, no dependsOn, no relatesTo fields
    And spec/work-units.json on disk shows AUTH-001.status='backlog' and AUTH-002.status='backlog'

  Scenario: Bulk relatesTo creates a symmetric edge on both sides with no status change
    Given a project root tempdir with spec/work-units.json containing AUTH-002 and AUTH-003 both status=backlog
    When I dispatch add-dependencies with workUnitId='AUTH-002' and dependencies.relatesTo=['AUTH-003']
    Then the dispatcher returns success=true
    And the returned data contains added=1
    And spec/work-units.json on disk shows AUTH-002.relatesTo=['AUTH-003']
    And spec/work-units.json on disk shows AUTH-003.relatesTo=['AUTH-002']
    And spec/work-units.json on disk shows AUTH-002.status='backlog' and AUTH-003.status='backlog'

  Scenario: Missing target id surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog and no NOPE-999 work unit
    When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['NOPE-999']
    Then the dispatcher returns success=false
    And the error message contains the substring "Target work unit 'NOPE-999' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Self-dependency is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-001']
    Then the dispatcher returns success=false
    And the error message contains the substring 'Cannot create self-dependency'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Circular blocks chain is detected and rejected
    Given a project root tempdir with spec/work-units.json where AUTH-002 already has blocks=['AUTH-001']
    When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002']
    Then the dispatcher returns success=false
    And the error message contains the substring 'Circular dependency detected: AUTH-001 -> AUTH-002 -> AUTH-001'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing source work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I dispatch add-dependencies with workUnitId='NOPE-001' and dependencies.blocks=['AUTH-001']
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002']
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
