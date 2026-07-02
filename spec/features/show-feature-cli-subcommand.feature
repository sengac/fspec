@feature-management
@cli
@done
@RPC-304
@wip
Feature: show-feature CLI subcommand on the standalone fspec Rust binary
  """
  The CLI bridge module codelet/fspec/src/show_feature.rs marshals argv into JSON
  args and delegates to codelet_fspec_core::commands::show_feature::run — the
  same function the LLM-facing dispatcher invokes.
  Exit-code contract: 0 on success (rendered content to stdout), 1 on
  any failure (Error: <message> to stderr).
  The clap subcommand exposes the positional <feature> argument, plus hidden
  --format and --output flags (TS show-feature-help.ts does not advertise them
  so the byte-equal --help fixture must omit them too).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want fspec show-feature to render a feature file's contents and work-unit summary
    So that I can inspect specs in CI/scripts without launching Node

  Scenario: Clap exposes show-feature as a subcommand and prints flag-aware help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-feature --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'show-feature'
    And stdout contains the substring 'Feature file path'

  Scenario: show-feature against a workspace with no spec/features prints feature-not-found and exits 1
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-feature missing` from that directory
    Then the command exits 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found: missing'

  Scenario: CLI text output renders feature contents and Work Units None for a tag-free feature
    Given a temp workspace contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    When I run `./codelet/target/release/fspec show-feature login` from that workspace
    Then the command exits 0
    And stdout contains the file body of spec/features/login.feature
    And stdout contains the exact line 'Work Units: None'

  Scenario: CLI text output renders Work Unit progress block when the feature carries a work-unit tag
    Given a temp workspace contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with scenario 'A' on line 4
    And spec/work-units.json contains AUTH-001 with title 'Login' and status 'implementing'
    When I run `./codelet/target/release/fspec show-feature auth` from that workspace
    Then the command exits 0
    And stdout contains the exact line '  AUTH-001 (feature-level) - Login'
    And stdout contains the exact line '    auth.feature:4 - A'

  Scenario: show-feature --help is byte-for-byte identical to TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-feature --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-feature.txt
    And stdout starts with a blank line followed by 'SHOW-FEATURE'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-feature registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-features
    When I run `./codelet/target/release/fspec --help`
    Then the command exits 0
    And the help output lists daemon, client, status, list-work-units, list-prefixes, and show-feature as available subcommands
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with scenario 'A' on line 4 and spec/work-units.json contains AUTH-001 with title 'Login' and status 'done'
    When I dispatch show-feature through fspec_core::dispatch::dispatch_command with feature='auth' and format='text' against that workspace
    And I run `./codelet/target/release/fspec show-feature auth` against the same workspace
    Then both invocations produce the exact line '  AUTH-001 (feature-level) - Login'
    And the CLI bridge module codelet/fspec/src/show_feature.rs contains NO inline gherkin parsing, work-unit aggregation, or text rendering — its only computation is JSON arg marshalling
