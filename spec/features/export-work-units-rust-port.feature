@done
@RPC-229
Feature: Port export-work-units command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/export_work_units.rs: signature pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>; direct std::fs::read_to_string of spec/work-units.json (no auto-create), Object.values via data.work_units.values(), for format=json serde_json::to_string_pretty(units) then std::fs::write(output, ...); else wrap 'Unsupported format: <fmt>'; returns {success:true}
  CLI bridge codelet/fspec/src/export_work_units.rs marshals format(positional)/output(positional)/status(--status) to core; success log mirrors broken TS 'Exported undefined work units to undefined'. Help config codelet/fspec-core/src/help/configs/export_work_units.rs mirrors export-work-units-help.ts. SUPERVISOR wires canonical.rs/dispatch.rs/commands.mod.rs/help configs.mod.rs/main.rs
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The core run function reads spec/work-units.json directly without auto-creating it (mirrors TS readFile, not ensureWorkUnitsFile)
  #   2. For format 'json' the command writes Object.values(workUnits) as 2-space-indent pretty JSON to the raw output path (used verbatim, not joined with cwd)
  #   3. The exported JSON is an array of full work-unit objects in insertion order, preserving each unit's on-disk field order
  #   4. For any format other than 'json' (e.g. 'csv') the command fails with the wrapped message 'Failed to export work units: Unsupported format: <format>' (CSV is NOT implemented despite the description)
  #   5. The --status and --epic options are accepted by the surface but IGNORED by the function: no filtering occurs and all work units are exported
  #   6. On success the core function returns the JSON envelope {"success": true}
  #   7. The --help output is byte-for-byte identical to node dist/index.js export-work-units --help (rendered via the Rust CommandHelpConfig)
  #   8. Framing A: the TS CLI success log references result.count and result.outputFile which are undefined, so the shell prints 'Exported undefined work units to undefined'; the Rust CLI bridge mirrors this while the dispatcher implements the useful function contract returning {success:true}
  #
  # EXAMPLES:
  #   1. Dispatcher: exporting format=json to out.json writes a 2-space pretty JSON array of all work units and returns {success:true}
  #   2. Dispatcher: exporting format=csv returns an error containing 'Failed to export work units: Unsupported format: csv'
  #   3. Dispatcher: with --status filter set, the exported file still contains ALL work units (status ignored)
  #   4. Dispatcher: the exported array preserves work-unit insertion order (C-1, A-1, B-1)
  #   5. CLI: 'fspec export-work-units json out.json' exits 0 and writes out.json; success line shows 'Exported undefined work units to undefined' per Framing A
  #   6. CLI: 'fspec export-work-units csv out.csv' exits 1 with stderr containing 'Failed to export work units: Unsupported format: csv'
  #   7. CLI: 'fspec export-work-units --help' prints the formatted help matching the captured fixture
  #   8. Two-front-doors: the CLI bridge and the dispatcher both call commands::export_work_units::run with identical JSON args and produce identical results
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to export all work units to a JSON file via the Rust-ported export-work-units command
    So that the standalone fspec binary can dump the work-unit store to an external file with TS-parity behaviour through both the dispatcher and the CLI

  Scenario: Dispatcher exports all work units to a JSON file
    Given a workspace with three work units in spec/work-units.json
    When the dispatcher runs export-work-units with format "json" and output "out.json"
    Then the result is the JSON envelope with success true
    And the file "out.json" contains a 2-space pretty JSON array of all three work units

  Scenario: Dispatcher rejects an unsupported format
    Given a workspace with a valid spec/work-units.json
    When the dispatcher runs export-work-units with format "csv" and output "out.csv"
    Then the error message contains "Failed to export work units: Unsupported format: csv"

  Scenario: Dispatcher ignores the status filter and exports every unit
    Given a workspace with work units in mixed statuses
    When the dispatcher runs export-work-units with format "json", output "out.json" and status "done"
    Then the file "out.json" contains every work unit regardless of status

  Scenario: Dispatcher preserves work-unit insertion order in the export
    Given a workspace whose work units were inserted in the order "C-1", "A-1", "B-1"
    When the dispatcher runs export-work-units with format "json" and output "out.json"
    Then the exported array lists work units in the order "C-1", "A-1", "B-1"

  Scenario: CLI and dispatcher converge on the same fspec-core function
    Given a workspace with a valid spec/work-units.json
    When the dispatcher and the CLI both invoke export-work-units with identical JSON args
    Then both invocations produce identical results from commands::export_work_units::run
