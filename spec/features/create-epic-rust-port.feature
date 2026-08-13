@done
@RPC-211
@rust
@cli
@mutation
Feature: Port create-epic command to Rust
  """
  Core impl at rust/fspec-core/src/commands/create_epic.rs reads spec/epics.json (tolerating missing/malformed via TS bare-catch parity), validates the epic id with regex `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`, rejects duplicates, then writes the merged store via io::locked_file::write_json_atomic. Errors are wrapped with the 'Failed to create epic: ' prefix to mirror TS outer-catch semantics.
  Help config at rust/fspec-core/src/help/configs/create_epic.rs mirrors src/commands/create-epic-help.ts byte-for-byte.
  CLI bridge at rust/fspec/src/create_epic.rs marshals the two positional args (epicId, title) plus optional -d/--description into JSON and delegates to commands::create_epic::run. No logic duplication.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::create_epic::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer
    I want to port the create-epic command to the Rust fspec-core crate
    So that the standalone fspec binary can create epics natively without delegating to TypeScript

  Scenario: Dispatcher creates a minimal epic and writes spec/epics.json
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='auth' and title='Authentication'
    Then the dispatcher returns success=true
    And spec/epics.json exists and contains an epic 'auth' with id='auth', title='Authentication', and a non-empty createdAt string
    And the epic record does NOT contain a 'description' key

  Scenario: Dispatcher creates an epic with a description
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='auth', title='Authentication', and description='Login features'
    Then the dispatcher returns success=true
    And spec/epics.json contains epic 'auth' with description='Login features'
    And in the on-disk JSON the 'createdAt' key appears before the 'description' key

  Scenario: Dispatcher preserves pre-existing epics when adding a new one
    Given spec/epics.json contains epic 'dash' with title='Dashboard'
    When I dispatch create-epic with epicId='auth' and title='Authentication'
    Then the dispatcher returns success=true
    And spec/epics.json contains both 'dash' and 'auth' epics
    And the existing 'dash' epic still has title='Dashboard'

  Scenario: Dispatcher rejects an invalid epicId with the canonical regex error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='AUTH' and title='Authentication'
    Then the dispatcher returns success=false
    And the error message does NOT contain the substring 'Failed to create epic'
    And the error message contains the substring 'lowercase-with-hyphens format'
    And spec/epics.json does NOT exist

  Scenario: Dispatcher rejects creating an epic that already exists
    Given spec/epics.json contains epic 'auth' with title='Old Title'
    When I dispatch create-epic with epicId='auth' and title='New Title'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to create epic'
    And the error message contains the substring 'Epic auth already exists'
    And the existing 'auth' epic in spec/epics.json still has title='Old Title'

  Scenario: Dispatcher auto-creates the spec directory when missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='auth' and title='Authentication'
    Then the dispatcher returns success=true
    And the directory spec/ exists
    And the file spec/epics.json exists

  Scenario: Dispatcher tolerates malformed spec/epics.json by treating it as empty (TS bare-catch parity)
    Given spec/epics.json exists with malformed bytes '{ not json'
    When I dispatch create-epic with epicId='auth' and title='Authentication'
    Then the dispatcher returns success=true
    And spec/epics.json is overwritten and contains exactly one epic 'auth'

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with no epicId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command create-epic'

  Scenario: Dispatcher response text renders the canonical success block without description
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='auth' and title='Authentication'
    Then the DispatchResult.data contains the line '✓ Created epic auth'
    And the DispatchResult.data contains the line '  Title: Authentication'
    And the DispatchResult.data does NOT contain the substring 'Description:'

  Scenario: Dispatcher response text includes the Description line when provided
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch create-epic with epicId='auth', title='Authentication', and description='Login features'
    Then the DispatchResult.data contains the line '✓ Created epic auth'
    And the DispatchResult.data contains the line '  Title: Authentication'
    And the DispatchResult.data contains the line '  Description: Login features'
