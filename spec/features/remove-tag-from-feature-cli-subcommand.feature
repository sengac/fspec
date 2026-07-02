@done
@RPC-281
Feature: remove-tag-from-feature CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/remove_tag_from_feature.rs (NEW CLI bridge); codelet/fspec/tests/cli_remove_tag_from_feature.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/remove-tag-from-feature.txt (captured help fixture from `node dist/index.js remove-tag-from-feature --help`).
  Bridge marshals positional <file> + variadic <tags...> into JSON and delegates to commands::remove_tag_from_feature::run. No logic in bridge — JSON marshalling only.
  Exit codes: 0 on success, 1 on FspecCoreError or {success:false} with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec remove-tag-from-feature <file> <tags...>` to behave byte-identically to the TypeScript implementation
    So that I can remove feature-level tags from a shell without depending on Node.js

  Scenario: CLI successfully removes a tag and prints the success line
    Given a tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec remove-tag-from-feature spec/features/login.feature @wip' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Removed @wip from spec/features/login.feature'
    And the file spec/features/login.feature in the tempdir does NOT contain a line whose trimmed value is '@wip'

  Scenario: CLI rejects removal of a tag not on the feature with exit 1
    Given a tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec remove-tag-from-feature spec/features/login.feature @notthere' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Tag @notthere not found on this feature'

  Scenario: CLI rejects missing file with exit 1
    Given a tempdir with NO spec/features/missing.feature file
    When I run 'fspec remove-tag-from-feature spec/features/missing.feature @wip' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'File not found: spec/features/missing.feature'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec remove-tag-from-feature --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/remove-tag-from-feature.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature through fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' and tags=['@wip']
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Removed @wip from spec/features/login.feature'
    And the CLI bridge module codelet/fspec/src/remove_tag_from_feature.rs contains NO inline gherkin parsing or tag-filter logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
