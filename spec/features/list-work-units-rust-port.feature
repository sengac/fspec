@wip
@querying
@cli
@rust
@RPC-253
Feature: Port list-work-units command to Rust
  """
  Shared modules to be created in rust/fspec-core: src/io/project_root.rs (find_or_create_spec_directory), src/io/locked_file.rs (atomic JSON read/write with file locking via fs2), src/io/ensure.rs (ensure_work_units_file, ensure_prefixes_file), src/types/work_unit.rs (WorkUnit/WorkUnitsData/WorkUnitType + Meta + state map)
  DispatchResult.data carries the JSON-encoded result; for text format the data field carries the plain text (with no ANSI), matching the existing dispatcher contract
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-work-units` MUST replace the NotYetPorted stub and return a real DispatchResult
  #   2. If spec/work-units.json is missing it MUST be auto-created with empty workUnits, all 7 Kanban states, and version stamp '0.7.1'
  #   3. If spec/prefixes.json is missing it MUST also be auto-created (empty prefixes object) for parity with the TS command
  #   4. Filter behavior MUST match TS: status=exact match, prefix appends '-' before startsWith, epic=exact match, type defaults missing field to 'story'
  #   5. Multiple filters combine with AND semantics
  #   6. Output preserves insertion order from spec/work-units.json (use IndexMap or serde_json::Map)
  #   7. Each output item contains id, title, status, and epic (epic only when truthy/non-empty)
  #   8. JSON format wraps results in { workUnits: [...] } with 2-space indent
  #   9. Text format prints 'Work Units (N)' header then each WU as 'ID [status] / title / Epic: name' separated by blank lines; empty result prints 'No work units found'
  #   10. Shared infrastructure (work-units I/O, project root detection, locked file access, WorkUnit types) MUST live in shared modules under rust/fspec-core/src/{io,types,output} and NOT inside the command stub
  #
  # EXAMPLES:
  #   1. Dispatch `list-work-units` against a tempdir that has no spec/ → command succeeds, returns JSON with empty workUnits array, spec/work-units.json is created with all 7 states
  #   2. Tempdir has work-units.json with AUTH-001 (backlog, epic=ux), AUTH-002 (implementing), DASH-001 (backlog) → `list-work-units` returns all three in JSON ordered as they appear in the file
  #   3. With same three WUs, `list-work-units --status=backlog --format=json` returns only AUTH-001 and DASH-001
  #   4. `--prefix=AUTH` against the same store returns AUTH-001 and AUTH-002 but not DASH-001
  #   5. `--epic=ux` returns only the WUs whose epic field equals 'ux' (AUTH-001 only)
  #   6. WU with no `type` field is treated as 'story'; `--type=story` includes it, `--type=task` excludes it
  #   7. Empty workUnits with `--format=text` prints 'No work units found'; populated with `--format=text` prints 'Work Units (N)' header plus one block per WU
  #   8. Combined `--status=backlog --prefix=AUTH` returns only AUTH-001
  #   9. Malformed work-units.json (invalid JSON) → command returns success=false with error containing 'Failed to parse work-units.json'
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec list-work-units` (with optional --status, --prefix, --epic, --type, --format filters) and get the same output as the TypeScript implementation
    So that I can browse and filter work units without relying on Node.js, achieving cross-frontend parity for the Rust dispatcher

  Scenario: Auto-creates spec/work-units.json and spec/prefixes.json on first run
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-work-units command against that project root
    Then the dispatcher returns success=true with an empty workUnits array
    Then spec/work-units.json exists with version '0.7.1' and all 7 Kanban states present and empty
    Then spec/prefixes.json exists with an empty prefixes object

  Scenario: Lists all work units in insertion order when no filters are applied
    Given spec/work-units.json contains AUTH-001 (backlog, epic 'ux'), AUTH-002 (implementing), DASH-001 (backlog) in that order
    When I dispatch list-work-units with no filters and format=json
    Then the workUnits array contains exactly AUTH-001, AUTH-002, DASH-001 in that order
    Then each entry contains id, title, and status fields, and AUTH-001 also contains an epic field equal to 'ux'

  Scenario: Filters by status when --status flag is provided
    Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    When I dispatch list-work-units with status='backlog' and format=json
    Then the workUnits array contains exactly AUTH-001 and DASH-001 and does not contain AUTH-002

  Scenario: Filters by prefix appending hyphen before startsWith match
    Given spec/work-units.json contains AUTH-001, AUTH-002, DASH-001 and AUTHX-001
    When I dispatch list-work-units with prefix='AUTH' and format=json
    Then the workUnits array contains exactly AUTH-001 and AUTH-002 and excludes both DASH-001 and AUTHX-001

  Scenario: Filters by epic with exact equality
    Given spec/work-units.json contains AUTH-001 (epic 'ux'), AUTH-002 (no epic), DASH-001 (epic 'platform')
    When I dispatch list-work-units with epic='ux' and format=json
    Then the workUnits array contains only AUTH-001

  Scenario: Filters by type defaulting missing type to story
    Given spec/work-units.json contains AUTH-001 with no type field and TASK-001 with type='task'
    When I dispatch list-work-units with type='story' and format=json
    Then the workUnits array contains AUTH-001 and does not contain TASK-001
    When I dispatch list-work-units again with type='task' and format=json
    Then the workUnits array contains only TASK-001

  Scenario: Combines multiple filters with AND semantics
    Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    When I dispatch list-work-units with status='backlog' and prefix='AUTH' and format=json
    Then the workUnits array contains only AUTH-001

  Scenario: Text format prints No work units found for empty result
    Given spec/work-units.json contains no work units
    When I dispatch list-work-units with format='text'
    Then the DispatchResult.data contains the string 'No work units found'

  Scenario: Text format prints work units header and entries when populated
    Given spec/work-units.json contains AUTH-001 (backlog, title 'Login feature', epic 'ux')
    When I dispatch list-work-units with format='text'
    Then the DispatchResult.data contains 'Work Units (1)' and 'AUTH-001 [backlog]' and 'Login feature' and 'Epic: ux'

  Scenario: Returns structured error when work-units.json is malformed
    Given spec/work-units.json exists but contains invalid JSON syntax
    When I dispatch list-work-units against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'

  Scenario: Shared infrastructure modules exist under rust/fspec-core for reuse by other commands
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/
    Then the modules io::project_root, io::locked_file, io::ensure, and types::work_unit exist and are publicly accessible from the crate root
    Then list_work_units::run delegates to these shared modules rather than embedding its own filesystem logic
