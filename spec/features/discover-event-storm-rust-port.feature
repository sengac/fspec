@done
@querying
@cli
@RPC-225
Feature: Port discover-event-storm command to Rust

  """
  File layout: core impl codelet/fspec-core/src/commands/discover_event_storm.rs (rewrite stub); CLI bridge codelet/fspec/src/discover_event_storm.rs; help config codelet/fspec-core/src/help/configs/discover_event_storm.rs; help fixture codelet/fspec/tests/fixtures/help/discover-event-storm.txt; core test codelet/fspec-core/tests/discover_event_storm.rs; CLI test codelet/fspec/tests/cli_discover_event_storm.rs. Module already registered as a stub in commands/mod.rs (do not edit).
  Shared types reused: crate::types::work_unit::WorkUnitsData + WorkUnitStatus::as_str() (parity with add_rule.rs specifying gate); crate::error::FspecCoreError. Missing-file Option B (inline path.exists(), no ensure_work_units_file) mirrors add_domain_event.rs. Read-only command — no write_json_atomic call.
  SHARED-CONTENT REQUEST (supervisor): getEventStormSection() is a ~220-line static guidance template (src/utils/slashCommandSections/eventStorm.ts) not yet ported to Rust. Need a shared fspec-core source (e.g. crate::slash_command_sections::event_storm_section() -> &'static str). Also verify a shared wrap_in_system_reminder helper — currently each command inlines '<system-reminder>' literals (show_work_unit.rs has a private wrap_in_system_reminder). Will ASK before inlining.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both the LLM dispatcher and the clap subcommand call the single fspec_core::commands::discover_event_storm::run function (two-front-doors); the CLI bridge does only JSON marshalling and stdout/stderr rendering
  #   2. Reads spec/work-units.json with NO auto-create (Option B): if the file does not exist the command fails with 'spec/work-units.json not found. Run fspec init first.' and writes nothing
  #   3. If workUnits[workUnitId] is absent the command fails with 'Work unit <id> not found'
  #   4. If the work unit status is not 'specifying' the command fails with 'Work unit <id> must be in specifying status (currently: <status>)' plus the guidance line 'Run: fspec update-work-unit-status <id> specifying'
  #   5. On success the command is read-only (no mutation of work-units.json) and emits the green line '✓ Event Storm discovery session started for <id>' followed by the Event Storm guidance wrapped in <system-reminder> tags
  #   6. The system-reminder body begins with 'EVENT STORM DISCOVERY - <id>', embeds the full Event Storm guidance section, and ends with 'Work unit: <id>' plus the next-step hint 'When done, run: fspec generate-example-mapping-from-event-storm <id>'
  #   7. The clap subcommand exposes exactly one required positional <work-unit-id> argument and no flags; --help is byte-for-byte identical to the captured TS help fixture
  #   8. The CLI exit code is 0 on success and 1 on any error, with error messages written to stderr prefixed 'Error:'
  #
  # EXAMPLES:
  #   1. Agent dispatches discover-event-storm for a work unit in specifying status and receives the green confirmation plus the Event Storm guidance wrapped in a system-reminder
  #   2. Agent runs discover-event-storm against an empty workspace with no spec/ directory and sees the error 'spec/work-units.json not found. Run fspec init first.' with exit code 1
  #   3. Agent runs discover-event-storm on a work unit currently in backlog and sees 'Work unit X must be in specifying status (currently: backlog)' plus the update-work-unit-status hint, exit 1
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to run a Rust port of discover-event-storm wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one Event Storm discovery guidance implementation

  Scenario: Dispatcher emits guidance for a work unit in specifying status
    Given spec/work-units.json contains AUTH-001 in specifying status
    When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success
    Then the output contains the green line '✓ Event Storm discovery session started for AUTH-001'
    Then the output contains a <system-reminder> block beginning with 'EVENT STORM DISCOVERY - AUTH-001'
    Then the output ends the reminder body with the hint 'When done, run: fspec generate-example-mapping-from-event-storm AUTH-001'
    Then spec/work-units.json is byte-identical after the call (read-only command)

  Scenario: Dispatcher returns missing-file error in an empty workspace without auto-creating
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'spec/work-units.json not found. Run fspec init first.'
    Then spec/work-units.json does NOT exist after the call

  Scenario: Dispatcher returns Work unit not found when the id is not a key in workUnits
    Given spec/work-units.json contains BUG-001 but not AUTH-001
    When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'

  Scenario: Dispatcher rejects a work unit not in specifying status
    Given spec/work-units.json contains AUTH-001 in backlog status
    When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false with an error message containing 'Work unit AUTH-001 must be in specifying status (currently: backlog)'
    Then the error message also contains 'Run: fspec update-work-unit-status AUTH-001 specifying'
