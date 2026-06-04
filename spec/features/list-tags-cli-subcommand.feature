@done
@RPC-251
@rust
@querying
@cli
Feature: List tags CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_tags::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes exactly ONE flag: `--category <CATEGORY>` (long-only, no short form), mirroring the TypeScript Commander.js registration at src/commands/list-tags.ts:100-105 which declares `.command('list-tags').description('List all registered tags from TAGS.md').option('--category <category>', ...)`. No `--format`, no `--workspace`, no `--cwd`.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-tags` directly from a shell with the same `--category`-only surface offered by the TypeScript Commander.js CLI
    So that I can browse registered tag categories from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-tags as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-tags --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-tags subcommand
    Then stdout contains the substring '--category'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'
    Then stdout does NOT contain the substring '--cwd'

  Scenario: CLI against empty directory auto-creates tags.json and prints all 9 default categories
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-tags` from that directory
    Then the command exits 0
    Then stdout contains the substring 'Phase Tags (0 tags)'
    Then stdout contains the substring 'Component Tags (0 tags)'
    Then stdout contains the substring 'Automation Tags (0 tags)'
    Then stdout contains the substring '  No tags registered'
    Then spec/tags.json was created in the directory

  Scenario: CLI text output renders alphabetically sorted tags per category
    Given spec/tags.json contains a Phase Tags category with tags '@zed' (description 'Z desc') and '@aaa' (description 'A desc') in that insertion order
    When I run `./codelet/target/release/fspec list-tags`
    Then the command exits 0
    Then stdout contains the substring 'Phase Tags (2 tags)'
    Then stdout contains the exact line '  @aaa - A desc'
    Then stdout contains the exact line '  @zed - Z desc'
    Then the line '  @aaa - A desc' appears BEFORE the line '  @zed - Z desc' in stdout

  Scenario: CLI --category filter restricts output to the matching category
    Given spec/tags.json contains Phase Tags (with '@critical') and Component Tags (with '@cli') categories
    When I run `./codelet/target/release/fspec list-tags --category 'Phase Tags'`
    Then the command exits 0
    Then stdout contains the substring 'Phase Tags'
    Then stdout contains the substring '@critical'
    Then stdout does NOT contain the substring 'Component Tags'
    Then stdout does NOT contain the substring '@cli'

  Scenario: CLI --category filter exits 1 and writes 'Category not found' to stderr for unknown category
    Given spec/tags.json contains Phase Tags and Component Tags categories
    When I run `./codelet/target/release/fspec list-tags --category 'No Such Category'`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Category not found: No Such Category. Available categories:'

  Scenario: CLI exits 1 and writes to stderr when tags.json is malformed
    Given spec/tags.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec list-tags`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse tags.json'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-tags registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-tags as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/tags.json contains a Phase Tags category with '@critical' (description 'Critical features')
    When I dispatch list-tags through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher's DispatchResult.data parses to a structure whose Phase Tags entry contains '@critical' with description 'Critical features'
    Then the CLI bridge module codelet/fspec/src/list_tags.rs contains NO inline category-filter, tag-sorting, or rendering logic
    Then the bridge module's only computation is JSON arg marshalling and CWD resolution
