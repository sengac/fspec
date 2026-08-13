@done
@mutation
@cli
@rust
@RPC-317
Feature: Port update-work-unit command to Rust
  """
  Core impl at rust/fspec-core/src/commands/update_work_unit.rs; signature pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>. Loads work-units via ensure_work_units_file and epics via ensure_epics_file. Two atomic writes (epics.json then work-units.json) via io::locked_file::write_json_atomic.
  WorkUnit typed fields used: title, epic, updated_at. description, parent, children arrays live in the WorkUnit.extra map and are mutated by string key (same pattern as update_prefix.rs) to avoid touching the shared work_unit.rs type. Circular-reference check is a recursive helper over the work_units IndexMap mirroring TS wouldCreateCircularReference.
  Core returns raw error reasons (TS throws unwrapped). CLI bridge at rust/fspec/src/update_work_unit.rs marshals --title/--description/--epic/--parent + positional workUnitId into JSON (omitting None) and prints '✗ Work unit <id> updated successfully' on success / error to stderr on failure (parity with TS chalk path). Help config at rust/fspec-core/src/help/configs/update_work_unit.rs mirrors update-work-unit-help.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both the LLM dispatcher and the clap CLI subcommand call the single commands::update_work_unit::run(args_json, project_root) function (two-front-doors)
  #   2. If the target work unit does not exist, return error "Work unit '<id>' does not exist" and leave disk state untouched
  #   3. If the type field is supplied (dispatcher only), reject it as immutable with the multi-line error 'Work unit type is immutable and cannot be changed after creation' including current and attempted type
  #   4. If --parent is supplied and the parent work unit does not exist, return error "Parent work unit '<parent>' does not exist"
  #   5. If setting --parent would create a circular ancestry (including self-parenting), return error 'Circular parent relationship detected'
  #   6. If --epic is supplied and the epic does not exist in spec/epics.json, return error "Epic '<epic>' does not exist"
  #   7. When --title or --description is provided, update only the supplied fields on the work unit (omitted fields are preserved verbatim)
  #   8. When --epic changes, remove the work unit id from the old epic's workUnits array and add it to the new epic's workUnits array (no duplicates), writing spec/epics.json atomically
  #   9. When --parent changes, remove the work unit id from the old parent's children array and add it to the new parent's children array (no duplicates)
  #   10. Every successful update sets updatedAt to the current ISO-8601 timestamp and writes spec/work-units.json atomically, returning { success: true }
  #   11. fspec update-work-unit --help is byte-for-byte identical to node dist/index.js update-work-unit --help
  #
  # EXAMPLES:
  #   1. Dispatch update-work-unit AUTH-001 with title='OAuth 2.0' updates the title and bumps updatedAt
  #   2. Dispatch update-work-unit MISSING-999 returns success=false with "Work unit 'MISSING-999' does not exist"
  #   3. Dispatch update-work-unit AUTH-001 with type='bug' returns success=false with the immutable-type error
  #   4. Dispatch update-work-unit AUTH-002 with parent=AUTH-002 (self) returns success=false with 'Circular parent relationship detected'
  #   5. Dispatch update-work-unit AUTH-001 with epic=SECURITY removes AUTH-001 from the old epic's workUnits and appends it to SECURITY's workUnits
  #   6. CLI: ./fspec update-work-unit AUTH-001 --title 'New' exits 0 and prints '✓ Work unit AUTH-001 updated successfully'
  #   7. CLI: ./fspec update-work-unit MISSING-999 --title X exits 1 and prints the failure message to stderr
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to port the update-work-unit command to the Rust fspec-core crate
    So that the standalone fspec binary can update work unit metadata natively without delegating to TypeScript

  Scenario: Dispatcher updates a work unit title and bumps updatedAt
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    When I dispatch update-work-unit with workUnitId='AUTH-001' and title='OAuth 2.0'
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-001' has title 'OAuth 2.0'
    And the updatedAt of 'AUTH-001' is set to a non-empty ISO-8601 string

  Scenario: Dispatcher rejects a missing work unit
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch update-work-unit with workUnitId='MISSING-999' and title='X'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-999' does not exist"

  Scenario: Dispatcher rejects changing the immutable type field
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    When I dispatch update-work-unit with workUnitId='AUTH-001' and type='bug'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Work unit type is immutable and cannot be changed after creation'

  Scenario: Dispatcher rejects a self-referential parent as circular
    Given spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-002'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Circular parent relationship detected'

  Scenario: Dispatcher rejects a non-existent parent
    Given spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-999'
    Then the dispatcher returns success=false
    And the error message contains the substring "Parent work unit 'AUTH-999' does not exist"

  Scenario: Dispatcher rejects a non-existent epic
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    When I dispatch update-work-unit with workUnitId='AUTH-001' and epic='NONEXISTENT'
    Then the dispatcher returns success=false
    And the error message contains the substring "Epic 'NONEXISTENT' does not exist"

  Scenario: Dispatcher moves a work unit between epics updating both workUnits arrays
    Given spec/epics.json contains epic 'auth' whose workUnits array includes 'AUTH-001'
    And spec/epics.json contains epic 'security' with an empty workUnits array
    And spec/work-units.json contains work unit 'AUTH-001' with title 'Login' and epic 'auth'
    When I dispatch update-work-unit with workUnitId='AUTH-001' and epic='security'
    Then the dispatcher returns success=true
    And the epic 'auth' workUnits array no longer contains 'AUTH-001'
    And the epic 'security' workUnits array contains 'AUTH-001'
    And spec/work-units.json work unit 'AUTH-001' has epic 'security'

  Scenario: Dispatcher sets a parent and updates the parent children array
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Parent'
    And spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-001'
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-002' has parent 'AUTH-001'
    And the children array of 'AUTH-001' contains 'AUTH-002'

  Scenario: CLI delegates to the same fspec-core function as the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    When I dispatch update-work-unit via the dispatcher with workUnitId='AUTH-001' and title='Same'
    And I run `./rust/target/release/fspec update-work-unit AUTH-001 --title Same` in an identical workspace
    Then both invocations produce the same success result and the same on-disk title 'Same'
