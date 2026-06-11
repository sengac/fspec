@done
@RPC-177
Feature: Port add-dependency command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/add_dependency.rs. Reuses io::ensure::ensure_work_units_file
  (auto-creates spec/work-units.json), io::locked_file::write_json_atomic (single atomic write), and
  io::time::iso8601_now (timestamps). The blocks/blockedBy/dependsOn/relatesTo/blockedReason fields
  live in WorkUnit.extra (round-tripped via #[serde(flatten)]). Cycle detection is a DFS over the
  blocks adjacency. Per-flag validation aborts before disk write — no partial state. Two-front-doors:
  the CLI bridge resolves positional shorthand AND conflict-validates before marshalling JSON into
  the core run; the dispatcher passes args_json verbatim.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-dependency` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both add work-unit dependency relationships without falling back to the TS implementation

  Scenario: dependsOn shorthand seeds the dependsOn array on source
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and AUTH-002 status=specifying
    When I dispatch add-dependency with workUnitId='AUTH-002' and dependsOn='AUTH-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-002.dependsOn=['AUTH-001']
    And spec/work-units.json on disk shows AUTH-001 has no blocks or blockedBy edge added
    And spec/work-units.json on disk shows AUTH-002.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: blocks creates bidirectional edge and auto-transitions target to blocked
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and API-001 status=specifying
    When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='API-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.blocks=['API-001']
    And spec/work-units.json on disk shows API-001.blockedBy=['AUTH-001']
    And spec/work-units.json on disk shows API-001.status='blocked'
    And spec/work-units.json on disk shows states.specifying no longer contains 'API-001'
    And spec/work-units.json on disk shows states.blocked contains 'API-001'

  Scenario: blocks targeting a done work unit does not transition its status
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and API-001 status=done
    When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='API-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows API-001.status='done'
    And spec/work-units.json on disk shows states.done still contains 'API-001'
    And spec/work-units.json on disk shows states.blocked does not contain 'API-001'

  Scenario: blockedBy creates bidirectional edge and auto-transitions source with blockedReason
    Given a project root tempdir with spec/work-units.json containing UI-001 status=specifying and API-001 status=specifying
    When I dispatch add-dependency with workUnitId='UI-001' and blockedBy='API-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-001.blockedBy=['API-001']
    And spec/work-units.json on disk shows API-001.blocks=['UI-001']
    And spec/work-units.json on disk shows UI-001.status='blocked'
    And spec/work-units.json on disk shows UI-001.blockedReason='Blocked by API-001'
    And spec/work-units.json on disk shows states.specifying no longer contains 'UI-001'
    And spec/work-units.json on disk shows states.blocked contains 'UI-001'

  Scenario: dependsOn flag creates unidirectional edge only
    Given a project root tempdir with spec/work-units.json containing DASH-001 status=specifying and AUTH-001 status=specifying
    When I dispatch add-dependency with workUnitId='DASH-001' and dependsOn='AUTH-001'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows DASH-001.dependsOn=['AUTH-001']
    And spec/work-units.json on disk shows AUTH-001 has no blocks edge added
    And spec/work-units.json on disk shows AUTH-001.status remains unchanged

  Scenario: relatesTo creates symmetric edges
    Given a project root tempdir with spec/work-units.json containing UI-005 status=specifying and UI-004 status=specifying
    When I dispatch add-dependency with workUnitId='UI-005' and relatesTo='UI-004'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-005.relatesTo=['UI-004']
    And spec/work-units.json on disk shows UI-004.relatesTo=['UI-005']
    And spec/work-units.json on disk shows neither UI-005 nor UI-004 changed status

  Scenario: relatesTo reverse edge is idempotent when already present
    Given a project root tempdir with spec/work-units.json containing UI-005 status=specifying and UI-004 status=specifying with UI-004.relatesTo already containing 'UI-005'
    When I dispatch add-dependency with workUnitId='UI-005' and relatesTo='UI-004'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows UI-005.relatesTo=['UI-004']
    And spec/work-units.json on disk shows UI-004.relatesTo=['UI-005']

  Scenario: Missing source work unit surfaces canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-dependency with workUnitId='NOPE-001' and dependsOn='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing target work unit surfaces canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='MISS-001'
    Then the dispatcher returns success=false
    And the error message contains the substring "Target work unit 'MISS-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Self-dependency is rejected for every flag
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Cannot create self-dependency'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Duplicate edge is rejected
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with dependsOn=['AUTH-000'] and AUTH-000 status=specifying
    When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='AUTH-000'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Dependency already exists'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Circular blocks chain is rejected before disk write
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with blocks=['AUTH-002'] and AUTH-002 status=blocked with blockedBy=['AUTH-001']
    When I dispatch add-dependency with workUnitId='AUTH-002' and blocks='AUTH-001'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Circular dependency detected: AUTH-002 -> '
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='AUTH-000'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
