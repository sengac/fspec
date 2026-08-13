@done
@feature-management
@cli
@RPC-212
Feature: create-feature CLI subcommand (Rust shell front-door)
  """
  Files: rust/fspec/src/create_feature.rs (NEW CLI bridge); rust/fspec/tests/cli_create_feature.rs (NEW CLI tests); rust/fspec/tests/fixtures/help/create-feature.txt (captured help fixture from `node dist/index.js create-feature --help`).
  Bridge marshals positional <name> into JSON {name} and delegates to commands::create_feature::run. No logic in bridge — JSON marshalling + CWD resolution only.
  Exit codes: 0 on success (✓ Created line + coverage line + optional reminders to stdout), 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec create-feature <name>` to behave byte-identically to the TypeScript implementation
    So that I can scaffold feature files from a shell without depending on Node.js

  Scenario: CLI creates a feature file and prints the success lines
    Given a tempdir with an empty spec directory
    When I run 'fspec create-feature "Payment Processing"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Created features/payment-processing.feature'
    And stdout contains the substring 'Edit the file to add your scenarios'
    And the file spec/features/payment-processing.feature exists in the tempdir

  Scenario: CLI prints the prefill system-reminder on stdout
    Given a tempdir with an empty spec directory
    When I run 'fspec create-feature "Payment Processing"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '<system-reminder>'
    And stdout contains the substring 'PREFILL DETECTED'
    And stdout contains the substring '</system-reminder>'

  Scenario: CLI fails with exit 1 and Error prefix when the file already exists
    Given a tempdir whose spec/features/payment-processing.feature already exists
    When I run 'fspec create-feature "Payment Processing"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'File already exists: spec/features/payment-processing.feature'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec create-feature --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/create-feature.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature through fspec_core::dispatch::dispatch_command with name='User Authentication'
    Then the dispatcher's DispatchResult.data parses to a structure whose filePath ends with 'spec/features/user-authentication.feature'
    And the CLI bridge module rust/fspec/src/create_feature.rs contains NO inline template, kebab-case, coverage, or prefill logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
