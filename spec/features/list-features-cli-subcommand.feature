@done
@querying
@cli
@rust
@RPC-245
Feature: List features CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_features::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes ONE flag — `--tag <TAG>` — mirroring the single TypeScript Commander.js option at src/commands/list-features.ts:156. No --format, no --workspace, no --cwd.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The fspec Rust binary exposes `list-features` as a clap v4 derive subcommand with the single flag `--tag <TAG>` mirroring the TypeScript Commander.js registration at src/commands/list-features.ts:152-157
  #   2. The action arm marshals CliArgs into JSON and delegates to fspec_core::commands::list_features::run — no glob, parse, filter, sort or render logic in the bridge module
  #   3. CWD-rooted: project root resolves via std::env::current_dir() (parity with TS process.cwd() default)
  #   4. Exit-code contract: 0 on success, 2 when error message contains 'Directory not found', 1 on any other FspecCoreError; stderr-prefixed with 'Error:'
  #   5. The combined default mode (no subcommand) is preserved alongside daemon, client, status, list-work-units, list-prefixes, and list-features
  #
  # EXAMPLES:
  #   1. `./codelet/target/release/fspec list-features --help` exits 0 and stdout describes the subcommand and the --tag flag (and does NOT advertise --status/--prefix/--epic/--format/--workspace)
  #   2. Running `fspec list-features` from a directory with no spec/ subdir exits 2 and writes 'Error:' plus 'Directory not found: spec/features/' to stderr
  #   3. Running `fspec list-features` from a directory with empty spec/features/ exits 0 and stdout contains 'No feature files found in spec/features/'
  #   4. Running `fspec list-features` from a directory with two feature files exits 0; stdout contains 'Found 2 feature files' and one rendered line per feature
  #   5. Running `fspec list-features --tag @critical` against a populated spec/features/ where one feature is tagged @critical → exit 0; stdout contains 'Found 1 feature files matching @critical' and lists only the matching feature
  #   6. fspec --help still lists daemon, client, status, list-work-units, list-prefixes, list-features as subcommands and documents the combined-mode default
  #   7. The CLI bridge module contains NO inline parsing/glob/filter/sort/render logic (asserted by source-content scan)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-features` directly from a shell — with optional `--tag` filtering — to browse my feature files
    So that I can audit the project's living documentation from a terminal or script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-features as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-features --help` from a shell
    Then the command exits 0
    Then stdout contains help describing the list-features subcommand
    Then stdout contains the substring '--tag'
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--epic'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against directory with no spec/ exits 2 with Directory-not-found error
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-features` from that directory
    Then the command exits with code 2
    Then stderr contains the substring 'Directory not found: spec/features/'

  Scenario: CLI against empty spec/features/ prints sentinel and exits 0
    Given a working directory containing an empty spec/features/ subdirectory
    When I run `./codelet/target/release/fspec list-features` from that directory
    Then the command exits 0
    Then stdout contains the substring 'No feature files found in spec/features/'

  Scenario: CLI text output renders feature listing for the populated case
    Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 2 scenarios
    Given spec/features/billing.feature exists with name 'Billing', tags '@billing' and 1 scenario
    When I run `./codelet/target/release/fspec list-features`
    Then the command exits 0
    Then stdout contains the exact line '  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]'
    Then stdout contains the exact line '  spec/features/billing.feature - Billing (1 scenarios) [@billing]'
    Then stdout contains the exact line 'Found 2 feature files'

  Scenario: CLI --tag filter narrows results and updates summary line
    Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    Given spec/features/billing.feature exists with tag '@billing' and 1 scenario
    When I run `./codelet/target/release/fspec list-features --tag @critical`
    Then the command exits 0
    Then stdout contains the substring 'spec/features/auth.feature'
    Then stdout does NOT contain the substring 'spec/features/billing.feature'
    Then stdout contains the exact line 'Found 1 feature files matching @critical'

  Scenario: Default combined TUI mode and other subcommands are preserved
    Given the fspec Rust binary has list-features registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-features as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI bridge module embeds NO duplicated business logic
    Given the file codelet/fspec/src/list_features.rs exists as the CLI bridge module
    When I read the bridge module source
    Then the source does NOT contain the substring 'No feature files found'
    Then the source does NOT contain the substring 'Found {}'
    Then the source does NOT contain the substring 'scenarioCount'
    Then the source does NOT contain the substring 'glob_feature_files'

  Scenario: CLI DirectoryNotFound error renders bare message plus indented Suggestion line
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-features` from that directory
    Then stderr contains the exact line 'Directory not found: spec/features/' (WITHOUT an 'Error:' prefix)
    Then stderr contains the exact line "  Suggestion: Run 'fspec create-feature' to create your first feature"
    Then the command exits with code 2


  Scenario: CLI prints a Warning line to stderr when a feature file cannot be parsed
    Given spec/features/valid.feature contains a parseable feature with 2 scenarios
    When I run `./codelet/target/release/fspec list-features` from that directory
    Then the command exits 0
    Given spec/features/broken.feature contains plain text with no Feature header
    Then stderr contains the exact line 'Warning: Could not parse spec/features/broken.feature'
    Then stdout contains the substring 'spec/features/valid.feature - Valid'
    Then stdout contains the exact line 'Found 1 feature files'


  Scenario: list-features --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-features --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-features.txt
    And stdout starts with a blank line followed by 'LIST-FEATURES'
    And stdout contains the section headers 'OPTIONS' and 'EXAMPLES' and 'NOTES'

