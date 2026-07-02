@done
@rust
@work-management
@cli
@RPC-255
Feature: Port prioritize-work-unit command to Rust
  """
  Core impl signature: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> in codelet/fspec-core/src/commands/prioritize_work_unit.rs (replaces NotYetPorted stub).
  Args struct (camelCase, serde default): workUnitId: String (required); position: optional accepting 'top'/'bottom' string OR a JSON number (use serde_json::Value or untagged enum); before: Option<String>; after: Option<String>.
  Loads via ensure_work_units_file; writes via write_json_atomic on the whole WorkUnitsData. Only states.<status> Vec reordered. Vec::insert index clamped to len to mirror JS splice-beyond-length.
  CLI bridge codelet/fspec/src/prioritize_work_unit.rs: CliArgs { work_unit_id, position: Option<String>, before, after }. Parse position string -> 'top'/'bottom' string or numeric JSON. Success line: '✓ Work unit <id> prioritized successfully'; error to stderr '✗ Failed to prioritize work unit: <msg>', exit 1.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both invocation paths (LLM dispatcher and standalone CLI) converge on a single fspec_core::commands::prioritize_work_unit::run(args_json, project_root) function (two-front-doors invariant).
  #   2. When workUnitId does not exist, error "Work unit '<id>' does not exist" and disk is untouched.
  #   3. When the work unit is in 'done' status, reordering is rejected with the canonical done-column error message.
  #   4. If the work unit is absent from its own states.<status> array, error "Data integrity error: ... Run 'fspec repair-work-units' to fix data corruption."
  #   5. Numeric position < 1 (e.g. 0 or -1) is rejected with "Invalid position: <n>. Position must be >= 1 (1-based index)".
  #   6. On success only the states.<status> array is reordered; workUnits map insertion order and all other fields are preserved (no updatedAt bump).
  #   7. before/after targets must exist; otherwise error "Work unit '<target>' does not exist".
  #   8. before/after target must be in the SAME column (status) as the work unit; cross-column reorder errors with the canonical 'Cannot prioritize across columns' message naming both statuses.
  #   9. Position top moves to index 0; position bottom moves to end; numeric position is 1-based (position N goes to index N-1, clamped to end if beyond length).
  #   10. Relative placement: before places the work unit at the target's index; after places it immediately following the target.
  #
  # EXAMPLES:
  #   1. Backlog [AUTH-002, AUTH-003, AUTH-001]; prioritize AUTH-001 --position top -> [AUTH-001, AUTH-002, AUTH-003].
  #   2. Backlog [AUTH-002, AUTH-003, AUTH-004, AUTH-001]; prioritize AUTH-001 --position 3 -> [AUTH-002, AUTH-003, AUTH-001, AUTH-004].
  #   3. prioritize AUTH-001 --position 0 fails with 'Invalid position: 0. Position must be >= 1 (1-based index)'.
  #   4. AUTH-001 status 'specifying' but listed only in states.testing; prioritize AUTH-001 --position top fails with 'Data integrity error: ... states.specifying ... fspec repair-work-units'.
  #   5. FEAT-017 in specifying, AUTH-001 in testing; prioritize FEAT-017 --before AUTH-001 fails (AUTH-001 not in states.specifying / cross-column).
  #   6. prioritize MISSING-999 --position top fails with "Work unit 'MISSING-999' does not exist" and work-units.json is unchanged.
  #   7. A 'done' work unit DONE-001 prioritized --position top fails with the done-column rejection message.
  #   8. Implementing [AUTH-002, AUTH-001]; prioritize AUTH-001 --before AUTH-002 -> [AUTH-001, AUTH-002]; prioritize AUTH-001 --after AUTH-002 -> [AUTH-002, AUTH-001].
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to run the Rust prioritize-work-unit command to reorder a work unit within its Kanban column
    So that the Rust CLI behaves identically to the TypeScript implementation

  Scenario: Position top moves a work unit to the front of its column
    Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position top`
    Then the process exits with code 0
    And the backlog order becomes AUTH-001, AUTH-002, AUTH-003

  Scenario: Numeric position is 1-based
    Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-004, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position 3`
    Then the process exits with code 0
    And the backlog order becomes AUTH-002, AUTH-003, AUTH-001, AUTH-004

  Scenario: Reject numeric position below 1
    Given spec/work-units.json backlog contains AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position 0`
    Then the process exits with code 1
    And stderr contains "Invalid position: 0. Position must be >= 1 (1-based index)"

  Scenario: Detect work unit missing from its own states array
    Given AUTH-001 has status specifying but is listed only in states.testing
    When I run `fspec prioritize-work-unit AUTH-001 --position top`
    Then the process exits with code 1
    And stderr contains "Data integrity error"
    And stderr contains "states.specifying"
    And stderr contains "fspec repair-work-units"

  Scenario: Reject cross-column relative placement
    Given FEAT-017 is in states.specifying and AUTH-001 is in states.testing
    When I run `fspec prioritize-work-unit FEAT-017 --before AUTH-001`
    Then the process exits with code 1
    And stderr contains "Data integrity error"

  Scenario: Reject prioritizing a non-existent work unit
    Given spec/work-units.json does not contain MISSING-999
    When I run `fspec prioritize-work-unit MISSING-999 --position top`
    Then the process exits with code 1
    And stderr contains "Work unit 'MISSING-999' does not exist"
    And spec/work-units.json is byte-identical to its pre-call content

  Scenario: Reject prioritizing a done work unit
    Given DONE-001 has status done and is in states.done
    When I run `fspec prioritize-work-unit DONE-001 --position top`
    Then the process exits with code 1
    And stderr contains "Cannot prioritize work units in done column"

  Scenario: Relative placement with before and after
    Given spec/work-units.json implementing order is AUTH-002, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --before AUTH-002`
    Then the process exits with code 0
    And the implementing order becomes AUTH-001, AUTH-002

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given the codelet/fspec crate is built
    When I inspect codelet/fspec/src/prioritize_work_unit.rs
    Then the source declares it calls codelet_fspec_core::commands::prioritize_work_unit::run
    Then the source does NOT perform any file IO directly on spec/work-units.json
