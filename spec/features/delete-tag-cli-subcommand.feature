@done
@RPC-222
Feature: delete-tag CLI subcommand (Rust shell front-door)

  """
  Files: codelet/fspec/src/delete_tag.rs (NEW CLI bridge); codelet/fspec/tests/cli_delete_tag.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/delete-tag.txt (captured fixture from `node dist/index.js delete-tag --help`)
  Bridge marshals positional <tag> + --force flag + --dry-run flag into JSON and delegates to commands::delete_tag::run. No logic in bridge — JSON marshalling only.
  Exit codes: 0 on success, 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec delete-tag <tag>` with optional --force and --dry-run to behave byte-identically to the TypeScript implementation
    So that I can prune obsolete tags from a shell without depending on Node.js


  Scenario: CLI deletes a tag and prints the multi-line success block when no feature files reference it
    Given a tempdir with spec/tags.json containing tag '@deprecated' under Status Tags
    And no feature files in the tempdir reference '@deprecated'
    When I run 'fspec delete-tag @deprecated' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully deleted tag @deprecated from registry'
    And stdout contains the substring 'Updated: spec/tags.json'
    And stdout contains the substring 'Regenerated: spec/TAGS.md'
    And spec/tags.json on disk in the tempdir no longer contains a tag named '@deprecated'


  Scenario: CLI dry-run prints the would-delete preview and skips the trailing 'Updated:' / 'Regenerated:' lines
    Given a tempdir with spec/tags.json containing tag '@critical' under Status Tags
    When I run 'fspec delete-tag @critical --dry-run' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Would delete tag @critical from category "Status Tags"'
    And stdout does not contain the substring 'Updated: spec/tags.json'
    And stdout does not contain the substring 'Regenerated: spec/TAGS.md'


  Scenario: CLI blocks deletion with stderr Error prefix and exit 1 when the tag is referenced and --force is not set
    Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    And spec/features/auth.feature in the tempdir contains the substring '@critical'
    When I run 'fspec delete-tag @critical' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Tag @critical is used in'
    And stderr contains the substring 'Use --force to delete anyway'


  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec delete-tag --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/delete-tag.txt
