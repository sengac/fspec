@validation
@validator
@rust
@wip
@RPC-324
Feature: Port validate-tags command to Rust
  """
  File layout: core impl rust/fspec-core/src/commands/validate_tags.rs (rewrite stub); help config rust/fspec-core/src/help/configs/validate_tags.rs; CLI bridge rust/fspec/src/validate_tags.rs; core test rust/fspec-core/tests/validate_tags.rs; CLI test rust/fspec/tests/cli_validate_tags.rs; help fixture rust/fspec/tests/fixtures/help/validate-tags.txt
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

  Scenario: Dispatcher reports all feature files valid when every tag is registered
    Given spec/tags.json registers a component tag and a feature-group tag plus the tags used by two feature files
    Given two feature files each carry only registered tags including a component and feature-group tag
    When I dispatch the validate-tags command against that project root
    Then the dispatcher returns success=true
    Then the result reports validCount=2 and invalidCount=0

  Scenario: Dispatcher flags an unregistered feature tag
    Given spec/tags.json does not register the tag '@made-up'
    Given a feature file carries the feature-level tag '@made-up'
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Unregistered tag: @made-up in spec/features/example.feature'
    Then the result reports invalidCount=1

  Scenario: Dispatcher rejects a scenario-level work-unit tag
    Given a feature file carries the work-unit tag '@AUTH-001' on a scenario
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Work unit ID tag @AUTH-001 must be at feature level, not scenario level'

  Scenario: Dispatcher reports a feature-level work-unit tag that is not in work-units.json
    Given spec/work-units.json exists and does NOT contain a work unit AUTH-999
    Given a feature file carries the feature-level work-unit tag '@AUTH-999'
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Work unit @AUTH-999 not found in spec/work-units.json'

  Scenario: Dispatcher reports a feature-level work-unit tag when work-units.json is missing
    Given spec/work-units.json does NOT exist in the project root
    Given a feature file carries the feature-level work-unit tag '@AUTH-001'
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Work unit @AUTH-001 found but spec/work-units.json does not exist'

  Scenario: Dispatcher reports a missing required component tag
    Given a feature file carries a feature-group tag but no component-category tag and no '@component' placeholder
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Missing required component tag'

  Scenario: Dispatcher reports a missing required feature-group tag
    Given a feature file carries a component tag but no feature-group-category tag and no '@feature-group' placeholder
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Missing required feature-group tag'

  Scenario: Dispatcher flags a lowercase work-unit-like tag as malformed
    Given a feature file carries the unregistered feature-level tag '@auth-001'
    When I dispatch the validate-tags command against that project root
    Then the result marks that file invalid
    Then the file errors include the message 'Invalid work unit tag format: @auth-001'

  Scenario: Dispatcher flags a placeholder @component tag
    Given a feature file carries the unregistered feature-level tag '@component'
    When I dispatch the validate-tags command against that project root
    Then the file errors include the message 'Placeholder tag: @component'

  Scenario: Dispatcher returns zero counts when no feature files exist
    Given an empty project root with no spec/features directory
    When I dispatch the validate-tags command against that project root
    Then the dispatcher returns success=true
    Then the result reports validCount=0 and invalidCount=0

  Scenario: Dispatcher validates only the single file named by the file argument
    Given two feature files exist but only one carries an unregistered tag
    When I dispatch the validate-tags command with file set to the valid feature file
    Then the result reports validCount=1 and invalidCount=0

  Scenario: Dispatcher skips a file that does not parse as Gherkin
    Given a feature file that does not contain a valid Feature header
    When I dispatch the validate-tags command against that project root
    Then the result marks that file valid
    Then the result reports invalidCount=0
