@done
@documentation
@cli
@RPC-299
Feature: show-acceptance-criteria CLI subcommand on the standalone fspec Rust binary
  """
  CLI subcommand wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_acceptance_criteria::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  show-acceptance-criteria exposes three optional flags: --tag <tag> (repeatable), --format <format> (default 'text'), --output <file>. No positional arguments (despite the TS help text claiming '<file>' — actually --output is the file flag).

  Exit-code contract: 0 on success; 1 when fspec_core::commands::show_acceptance_criteria::run returns FspecCoreError (spec/features missing) or the dispatcher envelope returns success=false. Error messages go to stderr prefixed with 'Error:'.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec show-acceptance-criteria --tag=@critical --format=markdown` from a shell with the same flags offered by the TypeScript Commander.js CLI
    So that I can extract acceptance-criteria documentation from a script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-acceptance-criteria as a subcommand and prints flag help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec show-acceptance-criteria --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'show-acceptance-criteria'

  Scenario: CLI against workspace with no spec/features exits 1
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./rust/target/release/fspec show-acceptance-criteria` from that directory
    Then the command exits 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'spec/features'

  Scenario: CLI default text output prints feature name and scenario steps
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario 'Login with valid credentials' and three steps
    When I run `./rust/target/release/fspec show-acceptance-criteria --tag @auth` from that workspace
    Then the command exits 0
    And stdout contains the substring 'login'
    And stdout contains the substring 'Login with valid credentials'

  Scenario: CLI --format=markdown prints H1/H2/bullet step output
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario and three steps
    When I run `./rust/target/release/fspec show-acceptance-criteria --tag @auth --format markdown` from that workspace
    Then the command exits 0
    And stdout contains the substring '# '
    And stdout contains the substring '## '

  Scenario: CLI --format=json prints 2-space JSON array
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    When I run `./rust/target/release/fspec show-acceptance-criteria --tag @auth --format json` from that workspace
    Then the command exits 0
    And stdout contains a JSON array as a substring

  Scenario: CLI --output writes file and prints message without dumping content
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    When I run `./rust/target/release/fspec show-acceptance-criteria --tag @auth --format markdown --output out.md` from that workspace
    Then the command exits 0
    And the file out.md in the workspace contains the rendered markdown
    And stdout contains the substring 'Acceptance criteria written to out.md'

  Scenario: CLI --tag matching zero features prints 'No features found matching tags'
    Given a temp workspace contains spec/features/login.feature tagged '@auth'
    When I run `./rust/target/release/fspec show-acceptance-criteria --tag @missing` from that workspace
    Then the command exits 0
    And stdout contains the substring 'No features found matching tags: @missing'

  Scenario: show-acceptance-criteria --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec show-acceptance-criteria --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-acceptance-criteria.txt
    And stdout starts with a blank line followed by 'SHOW-ACCEPTANCE-CRITERIA'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-acceptance-criteria registered as a clap subcommand alongside daemon, client, status, and other ported subcommands
    When I run `./rust/target/release/fspec --help`
    Then the help output lists show-acceptance-criteria as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    When I dispatch show-acceptance-criteria through fspec_core::dispatch::dispatch_command with tags=['@auth'] and format='json' against that workspace
    And I run `./rust/target/release/fspec show-acceptance-criteria --tag @auth --format json` against the same workspace
    Then both invocations produce equivalent JSON for the features array
    And the CLI bridge module rust/fspec/src/show_acceptance_criteria.rs contains NO inline gherkin parsing, filter, or rendering logic — its only computation is JSON arg marshalling
