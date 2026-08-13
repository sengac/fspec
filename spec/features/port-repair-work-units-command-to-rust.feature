@done
@rust
@work-management
@cli
@RPC-284
Feature: Port repair-work-units command to Rust
  """
  Core impl signature: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> in rust/fspec-core/src/commands/repair_work_units.rs (replaces NotYetPorted stub).
  Args struct (camelCase, serde default): dryRun: Option<bool> (accepted but ignored — file always written, matching TS bug). Loads via ensure_work_units_file; writes via write_json_atomic on the whole WorkUnitsData.
  States rebuilt into a fresh WorkUnitStates (fixed field order). blocks/blockedBy/relatesTo read from WorkUnit.extra as Value arrays. Borrow-checker: collect a mutation plan (target_id, field, source_id, message) in source-insertion order first, then apply to targets' extra arrays, to avoid simultaneous &mut over the IndexMap.
  Dispatcher returns pretty JSON { success, repairs, repaired }. CLI bridge rust/fspec/src/repair_work_units.rs marshals {dryRun?} only and prints '✓ Repaired <repaired> issues' on success (exit 0), '✗ Failed to repair work units: <msg>' on error (exit 1). The buggy result.details loop is omitted since details is always undefined in TS.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both invocation paths (LLM dispatcher and standalone CLI) converge on a single fspec_core::commands::repair_work_units::run(args_json, project_root) function (two-front-doors invariant).
  #   2. The states index is rebuilt from scratch: each work unit is placed into states.<its-status>, iterating workUnits in insertion order.
  #   3. When a work unit was found in a states array that does not match its status, a repair message 'Moved <id> from <oldState> to <status>' is recorded.
  #   4. Bidirectional dependency links are repaired: for blocks/blockedBy/relatesTo, the reverse link is added to the target work unit if the target exists and the reverse link is missing, recording a 'Repaired bidirectional link' message.
  #   5. The dispatcher returns JSON { success: true, repairs: string[], repaired: number }; the CLI prints '✓ Repaired <n> issues'.
  #   6. The --dry-run flag is accepted but has NO effect: the file is always written (preserving the TS behaviour where dryRun is ignored by the implementation).
  #   7. Repair messages are ordered deterministically: state-move messages computed during the rebuild pass (insertion order), then dependency-link messages in a second pass (insertion order of source, array order of targets).
  #
  # EXAMPLES:
  #   1. A work unit AUTH-001 with status 'specifying' is listed only in states.testing; running repair-work-units moves it to states.specifying and records 'Moved AUTH-001 from testing to specifying'.
  #   2. AUTH-001 has blocks:[AUTH-002] but AUTH-002 lacks blockedBy; repair adds AUTH-001 to AUTH-002.blockedBy and records 'Repaired bidirectional link: AUTH-001 blocks AUTH-002'.
  #   3. AUTH-001 has relatesTo:[AUTH-002] but AUTH-002 lacks the reverse; repair adds AUTH-001 to AUTH-002.relatesTo and records 'Repaired bidirectional link: AUTH-001 relates to AUTH-002'.
  #   4. A fully-consistent work-units.json yields repaired:0 and an empty repairs array, and the file content is functionally unchanged.
  #   5. Running repair-work-units --dry-run still writes the rebuilt states to disk (dry-run is a no-op flag) and prints '✓ Repaired <n> issues'.
  #   6. CLI: running 'fspec repair-work-units' on a corrupted file exits 0 and prints '✓ Repaired 1 issues'.
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to run the Rust repair-work-units command to fix data integrity issues in work-units.json
    So that the Rust CLI behaves identically to the TypeScript implementation

  Scenario: Move a work unit into the states array matching its status
    Given AUTH-001 has status specifying but is listed only in states.testing
    When I dispatch repair-work-units
    Then states.specifying contains AUTH-001 and states.testing does not
    And the repairs array contains "Moved AUTH-001 from testing to specifying"

  Scenario: Repair a missing blockedBy reverse link
    Given AUTH-001 has blocks AUTH-002 but AUTH-002 has no blockedBy entry
    When I dispatch repair-work-units
    Then AUTH-002.blockedBy contains AUTH-001
    And the repairs array contains "Repaired bidirectional link: AUTH-001 blocks AUTH-002"

  Scenario: Repair a missing relatesTo reverse link
    Given AUTH-001 has relatesTo AUTH-002 but AUTH-002 has no reverse relatesTo entry
    When I dispatch repair-work-units
    Then AUTH-002.relatesTo contains AUTH-001
    And the repairs array contains "Repaired bidirectional link: AUTH-001 relates to AUTH-002"

  Scenario: Fully consistent data yields zero repairs
    Given spec/work-units.json is fully consistent
    When I dispatch repair-work-units
    Then the result reports repaired 0 with an empty repairs array

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/repair_work_units.rs
    Then the source references the shared io helpers
    Then the source is no longer a NotYetPorted stub
