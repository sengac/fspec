@done
@RPC-217
@rust
@cli
@mutation
Feature: Port delete-epic command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/delete_epic.rs uses read_epics_or_empty to load spec/epics.json (auto-creating an empty store first to match the TS LockedFileManager.transaction side effect), checks for the requested key, and on hit removes it plus dereferences any matching prefixes (Prefix.epicId) and work units (WorkUnit.epic) — all writes go through io::locked_file::write_json_atomic.
  Errors are wrapped with the 'Failed to delete epic: ' prefix to mirror TS outer-catch semantics at src/commands/delete-epic.ts:84-89. Missing or malformed side-effect files (prefixes.json, work-units.json) are silently swallowed (TS bare-catch parity at lines 57-68 and 70-81).
  Two-front-doors: clap CLI and LLM dispatcher both call commands::delete_epic::run(args_json, project_root). The CLI bridge marshals only — no validation or rendering logic is duplicated.
  """

  Background: User Story
    As a fspec maintainer
    I want to port the delete-epic command to the Rust fspec-core crate
    So that the standalone fspec binary can delete epics natively (clearing epic references in prefixes and work-units) without delegating to TypeScript

  Scenario: Dispatcher deletes an existing epic from spec/epics.json
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And spec/epics.json no longer contains an 'auth' epic

  Scenario: Dispatcher clears epicId references on matching prefixes when deleting an epic
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/prefixes.json contains prefix 'AUTH' with epicId='auth'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And the AUTH prefix in spec/prefixes.json no longer has an epicId field

  Scenario: Dispatcher clears epic references on matching work units when deleting an epic
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/work-units.json contains work unit AUTH-001 with epic='auth'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And the AUTH-001 work unit in spec/work-units.json no longer has an epic field

  Scenario: Dispatcher rejects deletion of a missing epic with the canonical wrapped error
    Given spec/epics.json contains epic 'dash' with title='Dashboard'
    When I dispatch delete-epic with epicId='nonexistent'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to delete epic'
    And the error message contains the substring 'Epic nonexistent not found'
    And spec/epics.json still contains the 'dash' epic

  Scenario: Dispatcher tolerates missing spec/prefixes.json (TS bare-catch parity)
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/prefixes.json does NOT exist
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And spec/epics.json no longer contains an 'auth' epic

  Scenario: Dispatcher tolerates missing spec/work-units.json (TS bare-catch parity)
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/work-units.json does NOT exist
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true

  Scenario: Dispatcher tolerates malformed spec/prefixes.json silently
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/prefixes.json exists with malformed bytes '{ not json'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And spec/epics.json no longer contains an 'auth' epic

  Scenario: Dispatcher tolerates malformed spec/work-units.json silently
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    And spec/work-units.json exists with malformed bytes '{ not json'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And spec/epics.json no longer contains an 'auth' epic

  Scenario: Dispatcher preserves non-matching epics, prefixes, and work units
    Given spec/epics.json contains epics 'auth' and 'dash'
    And spec/prefixes.json contains prefix 'AUTH' with epicId='auth' and prefix 'OTHER' with epicId='dash'
    And spec/work-units.json contains work unit AUTH-001 with epic='auth' and DASH-001 with epic='dash'
    When I dispatch delete-epic with epicId='auth'
    Then the dispatcher returns success=true
    And spec/epics.json still contains the 'dash' epic
    And the OTHER prefix still has epicId='dash'
    And the DASH-001 work unit still has epic='dash'

  Scenario: Dispatcher response text renders the canonical success line
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    When I dispatch delete-epic with epicId='auth'
    Then the DispatchResult.data contains the line '✓ Epic auth deleted successfully'

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch delete-epic with no epicId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command delete-epic'
