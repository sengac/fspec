@querying
@cli
@done
@RPC-302
Feature: Show epic CLI subcommand

  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_epic::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  Unlike list-epics (which is flag-less), show-epic exposes one required positional argument <epicId> AND a `-f, --format <format>` flag defaulting to 'text' — mirroring the TypeScript Commander.js registration at src/commands/show-epic.ts:136-142.

  Exit-code contract: 0 on success; 1 when fspec_core::commands::show_epic::run returns FspecCoreError (epic missing OR malformed epics.json). Error messages are written to stderr prefixed with 'Error:'.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec show-epic <epicId>` directly from a shell with the same positional argument and --format flag offered by the TypeScript Commander.js CLI
    So that I can audit a single epic from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-epic as a subcommand and prints epicId-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-epic --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the show-epic subcommand
    Then stdout advertises the required positional <epicId> argument
    Then stdout advertises the '--format' flag
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--epic'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against empty workspace exits 1 with Epic not found error
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-epic auth` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Epic auth not found'
    Then spec/epics.json was NOT created in the directory
    Then spec/work-units.json was NOT created in the directory

  Scenario: CLI text output renders epic header and progress for the populated case
    Given spec/epics.json contains auth (title 'Authentication', description 'Login features')
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    When I run `./codelet/target/release/fspec show-epic auth`
    Then the command exits 0
    Then stdout contains the line 'Epic: auth'
    Then stdout contains the line 'Title: Authentication'
    Then stdout contains the line 'Description: Login features'
    Then stdout contains the line 'Progress:'
    Then stdout contains the exact line '  Total work units: 2'
    Then stdout contains the exact line '  Completed: 1'
    Then stdout contains the exact line '  Completion: 50%'

  Scenario: CLI exits 1 and writes to stderr when epics.json is malformed
    Given spec/epics.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec show-epic auth`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse epics.json'

  Scenario: CLI exits 1 when epicId is not registered
    Given spec/epics.json contains auth (title 'Authentication')
    When I run `./codelet/target/release/fspec show-epic nonexistent`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Epic nonexistent not found'

  Scenario: CLI exits 0 with text output when work-units.json is malformed
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I run `./codelet/target/release/fspec show-epic auth`
    Then the command exits 0
    Then stdout contains the line 'Epic: auth'
    Then stdout contains the exact line '  Total work units: 0'
    Then stdout contains the exact line '  Completion: 0%'

  Scenario: CLI --format json emits JSON to stdout
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    When I run `./codelet/target/release/fspec show-epic auth --format json`
    Then the command exits 0
    Then stdout parses as JSON whose root object has an 'epic' key
    Then the parsed JSON has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    Then the parsed JSON epic.id equals 'auth'

  Scenario: CLI -f json short flag matches the long form
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json does NOT exist
    When I run `./codelet/target/release/fspec show-epic auth -f json`
    Then the command exits 0
    Then stdout parses as JSON with an 'epic' key

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-epic registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-epics
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, list-epics, and show-epic as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/epics.json contains auth (title 'Authentication') and spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    When I dispatch show-epic through fspec_core::dispatch::dispatch_command with epicId='auth' and format='json'
    Then the dispatcher's DispatchResult.data parses to a structure with totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    Then the CLI text output (./fspec show-epic auth) reflects the same '  Completion: 50%' line
    Then the CLI bridge module codelet/fspec/src/show_epic.rs contains NO inline aggregation, filter, or rendering logic — its only computation is JSON arg marshalling

  Scenario: show-epic --help is byte-for-byte identical to the TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-epic --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-epic.txt
    And stdout starts with a blank line followed by 'SHOW-EPIC'
