@done
@RPC-265
Feature: register-tag CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/register_tag.rs (NEW CLI bridge); codelet/fspec/tests/cli_register_tag.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/register-tag.txt (captured help fixture from `node dist/index.js register-tag --help`)
  Bridge marshals positional <tag> <category> <description> args to JSON and delegates to commands::register_tag::run. No logic in bridge — JSON marshalling only.
  Exit codes: 0 on success, 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec register-tag <tag> <category> <description>` to behave byte-identically to the TypeScript implementation
    So that I can register tags from a shell without depending on Node.js

  Scenario: CLI successfully registers a new tag and prints the multi-line success block
    Given a tempdir with no spec/tags.json
    When I run 'fspec register-tag @ws "Technical Tags" "WebSocket features"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully registered @ws in Technical Tags'
    And stdout contains the substring 'Updated: spec/tags.json'
    And stdout contains the substring 'Regenerated: spec/TAGS.md'
    And spec/tags.json exists in the tempdir
    And spec/TAGS.md exists in the tempdir

  Scenario: CLI rejects invalid tag format with stderr Error prefix and exit 1
    Given a tempdir with no spec/tags.json
    When I run 'fspec register-tag InvalidTag "Technical Tags" "desc"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Invalid tag format'

  Scenario: CLI reports lowercase conversion note when tag contained uppercase characters
    Given a tempdir with no spec/tags.json
    When I run 'fspec register-tag @API-Integration "Technical Tags" "API"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully registered @api-integration (converted from @API-Integration) in Technical Tags'
    And stdout contains the substring 'Note: Tag converted to lowercase: @API-Integration → @api-integration'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec register-tag --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/register-tag.txt
