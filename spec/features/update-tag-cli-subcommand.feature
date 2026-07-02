@done
@RPC-316
Feature: update-tag-cli-subcommand
  """
  Files: codelet/fspec/src/update_tag.rs (NEW CLI bridge); codelet/fspec/tests/cli_update_tag.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/update-tag.txt (captured help fixture from `node dist/index.js update-tag --help`).
  Bridge marshals positional <tag> + --category + --description into JSON and delegates to commands::update_tag::run. No logic in bridge — JSON marshalling only.
  Exit codes: 0 on success, 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec update-tag <tag>` with optional --category and --description to behave byte-identically to the TypeScript implementation
    So that I can refine tag descriptions or move tags between categories from a shell without depending on Node.js

  Scenario: CLI updates description in place and prints multi-line success block
    Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    When I run 'fspec update-tag @critical --description "Critical paths only"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully updated @critical'
    And stdout contains the substring 'Updated: spec/tags.json'
    And stdout contains the substring 'Regenerated: spec/TAGS.md'

  Scenario: CLI rejects missing updates with stderr Error prefix and exit 1
    Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    When I run 'fspec update-tag @critical' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'No updates specified'

  Scenario: CLI moves tag between categories with --category flag
    Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags and an empty Priority Tags category
    When I run 'fspec update-tag @critical --category "Priority Tags"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully updated @critical'
    And the Priority Tags category on disk contains a tag named '@critical'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec update-tag --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/update-tag.txt
