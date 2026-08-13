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

  Scenario: Clap exposes update-work-unit with positional arg and metadata flags in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec update-work-unit --help`
    Then the command exits 0
    And stdout describes the update-work-unit subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout advertises the `--title` flag (or its `-t` short form)
    And stdout advertises the `--description` flag (or its `-d` short form)
    And stdout advertises the `--epic` flag (or its `-e` short form)
    And stdout advertises the `--parent` flag (or its `-p` short form)
    And the --help output is byte-for-byte identical to the captured TS reference fixture

  Scenario: CLI updates a work unit title and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    When I run `./rust/target/release/fspec update-work-unit AUTH-001 --title New`
    Then the command exits 0
    And stdout contains the line '✓ Work unit AUTH-001 updated successfully'
    And spec/work-units.json work unit 'AUTH-001' has title 'New'

  Scenario: CLI reports failure for a missing work unit on stderr
    Given an empty working directory with no spec/ subdirectory
    When I run `./rust/target/release/fspec update-work-unit MISSING-999 --title X`
    Then the command exits 1
    And stderr contains the substring "Work unit 'MISSING-999' does not exist"
