@done
@RPC-193
Feature: add-tag-to-feature CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/add_tag_to_feature.rs (NEW CLI bridge); codelet/fspec/tests/cli_add_tag_to_feature.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/add-tag-to-feature.txt (captured help fixture from `node dist/index.js add-tag-to-feature --help`).
  Bridge marshals positional <file> + variadic <tags...> + optional --validate-registry into JSON and delegates to commands::add_tag_to_feature::run. No logic in bridge — JSON marshalling only.
  Exit codes: 0 on success, 1 on FspecCoreError or {success:false} with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec add-tag-to-feature <file> <tags...>` to behave byte-identically to the TypeScript implementation
    So that I can add feature-level tags from a shell without depending on Node.js

  Scenario: CLI successfully adds a single tag and prints the success line
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-tag-to-feature spec/features/login.feature @critical' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added @critical to spec/features/login.feature'
    And the file spec/features/login.feature in the tempdir contains the line '@critical' above 'Feature: Login'

  Scenario: CLI surfaces invalid-format errors with stderr Error prefix and exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-tag-to-feature spec/features/login.feature InvalidTag' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Invalid tag format. Tags must start with @'

  Scenario: CLI --validate-registry rejects unregistered tag with exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n' and spec/tags.json carrying the canonical 9-category default
    When I run 'fspec add-tag-to-feature spec/features/login.feature @unregistered --validate-registry' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Tag @unregistered is not registered in spec/tags.json'

  Scenario: CLI prints consolidated system-reminder block after success line when unregistered non-work-unit tag is added
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n' and spec/tags.json carrying the canonical 9-category default
    When I run 'fspec add-tag-to-feature spec/features/login.feature @unknown' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added @unknown to spec/features/login.feature'
    And stdout contains the substring '<system-reminder>'
    And stdout contains the substring 'is not registered in spec/tags.json'
    And stdout contains the substring '</system-reminder>'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec add-tag-to-feature --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-tag-to-feature.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature through fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' and tags=['@cli']
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Added @cli to spec/features/login.feature'
    And the CLI bridge module codelet/fspec/src/add_tag_to_feature.rs contains NO inline gherkin parsing, tag-validation regex, or insertion logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
