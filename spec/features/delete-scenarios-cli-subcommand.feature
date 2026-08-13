@done
@bulk-operations
@cli
@RPC-220
Feature: delete-scenarios CLI subcommand on the standalone fspec Rust binary
  """
  CLI subcommand wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::delete_scenarios::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  delete-scenarios exposes a repeatable --tag flag (AND logic) and a --dry-run flag. No positional arguments. The core returns the envelope {success, deletedCount, fileCount, message?, scenarios?, error?} and the bridge owns all rendering: dry-run prints 'Dry run mode - no files modified' + a 'Would delete N scenario(s) from M file(s):' header + per-file scenario lists; real delete prints '✓ <message>'; an inner success=false prints 'Error: <error>' to stderr and exits 1; missing --tag prints 'Error: At least one --tag is required' to stderr and exits 1.

  delete-scenarios has NO matching custom -help.ts (the TS file delete-scenarios-by-tag-help.ts maps to command name delete-scenarios-by-tag), so its --help is bare Commander.js, hard-coded as DELETE_SCENARIOS_HELP in main.rs and emitted via a print! intercept arm (mirrors delete-features / list-foundation-sections).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec delete-scenarios --tag @spike --dry-run` from a shell with the same flags offered by the TypeScript Commander.js CLI
    So that I can clean up scenarios from a script without going through the LLM tool-call dispatcher

  Scenario: CLI dry-run previews deletions without removing scenarios
    Given a tempdir with one feature containing two @spike scenarios
    When I run 'fspec delete-scenarios --tag @spike --dry-run' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring 'Dry run mode - no files modified'
    And stdout contains the substring 'Would delete 2 scenario(s) from 1 file(s):'
    And the feature file still contains both @spike scenarios

  Scenario: CLI deletes matching scenarios and prints the success message
    Given a tempdir with one feature containing two @spike scenarios and one untagged scenario
    When I run 'fspec delete-scenarios --tag @spike' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Deleted 2 scenario(s) from 1 file(s)'
    And the feature file no longer contains the @spike scenarios

  Scenario: CLI with no --tag exits 1 with stderr Error prefix
    Given a tempdir with a feature tagged @spike
    When I run 'fspec delete-scenarios' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with one feature containing two @spike scenarios
    When I run a dry-run delete-scenarios once via the dispatcher and once via the CLI on identical inputs
    Then both front doors report the same deletedCount and fileCount

  Scenario: CLI help output matches the captured bare-Commander fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec delete-scenarios --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/delete-scenarios.txt
