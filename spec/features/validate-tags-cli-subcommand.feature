@validation
@cli
@rust
@wip
@RPC-324
Feature: Validate-tags CLI subcommand

  """
  File layout: core impl codelet/fspec-core/src/commands/validate_tags.rs (rewrite stub); help config codelet/fspec-core/src/help/configs/validate_tags.rs; CLI bridge codelet/fspec/src/validate_tags.rs; core test codelet/fspec-core/tests/validate_tags.rs; CLI test codelet/fspec/tests/cli_validate_tags.rs; help fixture codelet/fspec/tests/fixtures/help/validate-tags.txt
  Reuse: ensure_tags_file (RPC-251), io::gherkin::parse_feature_lenient (RPC-299) for feature+scenario tag extraction. SHARED-FILE REQUEST to supervisor: (1) a read-only work-units loader returning Option<WorkUnitsData> on any error (null parity) — model in-command as Option or add io::ensure::read_work_units_or_none; (2) non-throwing feature glob (glob_feature_files errors on missing spec/features; need empty-vec parity) — either add glob_feature_files_or_empty or handle DirectoryNotFound in-command; (3) supervisor wires canonical.rs PORTED_COMMANDS, dispatch.rs run_ported, commands/mod.rs already has module, help/configs/mod.rs, main.rs Mode+intercept+forward. Work-unit tag regexes ported inline.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The tag registry is loaded via ensure_tags_file (auto-creates spec/tags.json if missing); validTags = flat set of every tag.name across all categories; requiredCategories.component = names under 'Component Tags', requiredCategories.featureGroup = names under 'Feature Group Tags'
  #   2. Work-units data is loaded read-only and treated as null on ANY error (ENOENT or parse) — it is NOT auto-created (parity with loadWorkUnitsData)
  #   3. File selection: when a [file] argument is given, validate only that file; otherwise glob spec/features/**/*.feature; an empty file set returns results=[], validCount=0, invalidCount=0 (missing spec/features must NOT error — tinyglobby returns [])
  #   4. Per file: if it fails to gherkin-parse or has no Feature, the file is treated as valid (skipped). Feature-level and scenario-level tags are extracted separately
  #   5. Unregistered FEATURE tags: a work-unit tag (matches @[A-Z]{2,6}-\d+) routes to work-unit existence checks; a work-unit-LIKE tag (lowercase letters) errors 'Invalid work unit tag format: <tag>'; @component or @feature-group error 'Placeholder tag: <tag>'; anything else errors 'Unregistered tag: <tag> in <file>'
  #   6. Scenario-level work-unit tags (matching @[A-Z]{2,6}-\d+) are ALWAYS an error: 'Work unit ID tag <tag> must be at feature level, not scenario level' (BUG-005). Other unregistered scenario tags error 'Invalid work unit tag format' or 'Unregistered tag: <tag> in <file>'
  #   7. Work-unit tag existence (reportWorkUnitTag): extractWorkUnitId null -> 'Invalid work unit tag format'; workUnitsData null -> 'Work unit <tag> found but spec/work-units.json does not exist'; id not present in workUnits -> 'Work unit <tag> not found in spec/work-units.json'; present -> no error
  #   8. Required-category check (feature tags): missing a component-category tag AND no '@component' placeholder -> 'Missing required component tag'; missing feature-group tag AND no '@feature-group' placeholder -> 'Missing required feature-group tag'
  #   9. Output rendering: --summary suppresses per-file lines and prints only summary; --verbose prints one '✓ All tags in <file> are registered' per passing file; default prints only '✗ <file> has tag violations:' blocks (with '  <message>' and '  Suggestion: <s>' lines). Summary printed when summary flag set OR more than one file. Single-file no-flag passing run produces NO output
  #   10. Exit codes: 0 when all files valid, 1 when one or more files invalid, 2 on unexpected error (caught exception, output.error('Error:', msg))
  #   11. Two front doors: the clap subcommand exposes [file] positional + --verbose + --summary booleans and delegates to fspec_core::commands::validate_tags::run; the LLM dispatcher passes the same args_json. The CLI bridge does only JSON marshalling
  #   12. validate-tags --help is byte-for-byte identical to TS formatCommandHelp output (custom validate-tags-help.ts exists -> dedicated help config module)
  #
  # EXAMPLES:
  #   1. All feature files have only registered tags and the required component + feature-group categories -> dispatcher returns validCount=N, invalidCount=0
  #   2. A feature uses tag @made-up not in tags.json -> result invalid with message 'Unregistered tag: @made-up in <file>'
  #   3. A scenario carries a work-unit tag @AUTH-001 -> result invalid with message 'Work unit ID tag @AUTH-001 must be at feature level, not scenario level'
  #   4. A feature has @AUTH-999 at feature level but spec/work-units.json has no AUTH-999 -> message 'Work unit @AUTH-999 not found in spec/work-units.json'
  #   5. A feature missing any component-category tag -> message 'Missing required component tag'
  #   6. Running validate-tags on a single valid file with no flags produces no stdout and exits 0; running across multiple files prints the '✓ N files passed' summary
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec validate-tags` and have it validate feature-file tags against spec/tags.json with the same registration and placement rules as the TypeScript implementation
    So that I can catch unregistered tags and misplaced work-unit tags from a shell or the LLM dispatcher without relying on Node.js

  Scenario: Clap exposes validate-tags as a subcommand with file argument and flags
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec validate-tags --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/validate-tags.txt
    And stdout starts with a blank line followed by 'VALIDATE-TAGS'

  Scenario: CLI exits 0 with no output for a single valid file and no flags
    Given spec/tags.json registers the tags used by a feature file including a component and feature-group tag
    Given a single feature file carries only registered tags
    When I run `./codelet/target/release/fspec validate-tags spec/features/valid.feature`
    Then the command exits 0
    Then stdout is empty

  Scenario: CLI exits 1 and prints a violation block for an unregistered tag
    Given a feature file carries the unregistered feature-level tag '@made-up'
    When I run `./codelet/target/release/fspec validate-tags spec/features/bad.feature`
    Then the command exits with code 1
    Then stdout contains the substring 'has tag violations:'
    Then stdout contains the substring 'Unregistered tag: @made-up'

  Scenario: CLI --verbose prints a passing line per valid file
    Given spec/tags.json registers the tags used by a feature file including a component and feature-group tag
    Given a single feature file carries only registered tags
    When I run `./codelet/target/release/fspec validate-tags spec/features/valid.feature --verbose`
    Then the command exits 0
    Then stdout contains the substring '✓ All tags in spec/features/valid.feature are registered'

  Scenario: CLI --summary prints only the summary count lines
    Given two feature files where one has an unregistered tag
    When I run `./codelet/target/release/fspec validate-tags --summary`
    Then the command exits with code 1
    Then stdout contains the substring 'files passed'
    Then stdout contains the substring 'files have tag violations'
    Then stdout does NOT contain the substring 'has tag violations:'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root with spec/tags.json and a feature file carrying an unregistered tag
    When I dispatch validate-tags through fspec_core::dispatch::dispatch_command and also run `./codelet/target/release/fspec validate-tags` against the same on-disk state
    Then both paths agree the file is invalid
    Then the CLI bridge module codelet/fspec/src/validate_tags.rs contains NO inline validation or rendering logic — its only computation is JSON arg marshalling
