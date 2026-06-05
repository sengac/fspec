@done
@rust
@querying
@cli
@RPC-244
Feature: List Feature Tags Cli Subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_feature_tags::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes exactly ONE flag: --show-categories (long-only, no short form), mirroring the TypeScript Commander.js registration at src/commands/list-feature-tags.ts:159-167 which declares .command('list-feature-tags').argument('<file>', ...).option('--show-categories', ...). No --format, no --workspace, no --cwd. The <file> positional is REQUIRED — clap returns exit code 2 if omitted.

  The bridge module at codelet/fspec/src/list_feature_tags.rs performs only JSON arg marshalling and CWD resolution; no Gherkin parsing, no tag iteration, no category lookup, no rendering. The scenario_cli_bridge_module_embeds_no_duplicated_business_logic test scans the bridge file for forbidden TAG-DOMAIN substrings (e.g. 'No tags found', 'Tags on this feature', 'Invalid Gherkin syntax') to enforce the two-front-doors invariant.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-feature-tags <file>` directly from a shell with the same positional-and---show-categories surface offered by the TypeScript Commander.js CLI
    So that I can audit the feature-level tags on a single .feature file (with optional category cross-reference) from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: CLI clap subcommand exposes list-feature-tags with --show-categories
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-feature-tags --help` from a shell
    Then the command exits 0
    And stdout contains clap-generated help describing the list-feature-tags subcommand
    And stdout contains the substring '--show-categories'
    And stdout contains a positional argument descriptor for `<FILE>` or `<file>`
    And stdout does NOT contain the substring '--format'
    And stdout does NOT contain the substring '--workspace'
    And stdout does NOT contain the substring '--cwd'

  Scenario: CLI happy path prints flat alphabetical-declaration tag list and exits 0
    Given spec/features/user-auth.feature exists with feature-level tags '@critical @auth' on a single line before 'Feature: User Authentication'
    When I run `./codelet/target/release/fspec list-feature-tags spec/features/user-auth.feature` from the project root
    Then the command exits 0
    And stdout contains the substring 'Tags on this feature:'
    And stdout contains the exact line '  @critical'
    And stdout contains the exact line '  @auth'
    And the line '  @critical' appears BEFORE the line '  @auth' in stdout

  Scenario: CLI bridge module embeds no duplicated business logic
    Given the CLI bridge module codelet/fspec/src/list_feature_tags.rs is the only shell-facing entry point for list-feature-tags
    When the test harness reads the bridge source file as a string
    Then the bridge source does NOT contain the substring 'No tags found on this feature'
    And the bridge source does NOT contain the substring 'File does not contain a valid Feature'
    And the bridge source does NOT contain the substring 'Invalid Gherkin syntax'

  Scenario: CLI --show-categories flag emits categorized tag/category pairs
    Given spec/features/user-auth.feature exists with feature-level tag '@critical' before 'Feature: User Authentication' AND spec/tags.json registers '@critical' under the Phase Tags category
    When I run `./codelet/target/release/fspec list-feature-tags spec/features/user-auth.feature --show-categories`
    Then the command exits 0
    And stdout reflects the category cross-reference produced by fspec_core::commands::list_feature_tags::run with showCategories=true
