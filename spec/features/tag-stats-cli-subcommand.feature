@done
@rust
@querying
@cli
@RPC-310
Feature: Port tag-stats command to Rust

  """
  Two-front-doors pattern: dispatcher (LLM tool-call JSON) and clap CLI subcommand BOTH call the single fspec_core::commands::tag_stats::run function. CLI bridge only marshals empty args JSON; never duplicates counting/projection/rendering logic.
  Reuse codelet/fspec-core/src/io/feature_glob::glob_feature_files for spec/features/**/*.feature enumeration. When that helper returns FspecCoreError::DirectoryNotFound, tag-stats MUST catch it locally and treat it as an empty list (matching tinyglobby's empty-array-on-missing-dir behaviour). No shared-file change required.
  tags.json loading: DO NOT use ensure_tags_file (which auto-creates AND escalates parse errors). Read inline with std::fs::read_to_string + serde_json::from_str::<TagsData>. ANY failure (ENOENT, malformed JSON) → tagsFileFound=false and tagsData=None (mirrors TS bare catch at tag-stats.ts:42-49).
  Feature-tag extraction: reuse the inline gherkin scanner pattern from list_feature_tags::parse_feature_tags (private helper, copied into tag_stats.rs to keep the change parallel-safe). Returns Option<Vec<String>> — None means 'no Feature: header found' → file added to invalidFiles.
  Output struct: typed `TagStatsResult` with `#[derive(Serialize)]` and explicit `#[serde(rename_all = "camelCase")]` to preserve declaration order: success → totalFiles → uniqueTags → totalOccurrences → categories → unusedTags → tagsFileFound → invalidFiles.
  Text renderer mirrors TS `tagStatsCommand` line structure exactly (50-char `─` separators, `Total feature files:` counters, `⚠ Warning:` blocks for missing tags.json and invalid files, `Tag Counts by Category` section with `<tag.padEnd(30)> <count>` rows, `Unused Registered Tags` section). Use `format!("{:width$}", tag, width=30)` for the pad-end pattern, NOT chalk colours.
  Exit code on CLI error: 1 (consistent with list-prefixes/list-tags Rust bridges). TS uses 2 on uncaught throw; the Rust port unifies on 1 since all FspecCoreError surfaces are structured.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `tag-stats` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by the other ported commands
  #   2. If `spec/tags.json` is missing OR malformed, the command MUST set tagsFileFound=false and continue rendering using only the in-memory tag counts (parity with TS bare catch at src/commands/tag-stats.ts:42-49) — neither ENOENT nor JSON.parse errors propagate
  #   3. If `spec/features/` is missing the command MUST return totalFiles=0, uniqueTags=0, totalOccurrences=0 with empty categories / unusedTags / invalidFiles arrays (parity with tinyglobby returning [] for a missing directory at src/commands/tag-stats.ts:52-68)
  #   4. Only feature-level tags (the tag block immediately preceding the `Feature:` keyword) are counted; scenario-level, rule-level, and example-level tags MUST be ignored (TS uses `gherkinDocument.feature.tags` exclusively at src/commands/tag-stats.ts:96)
  #   5. Each tag occurrence on a feature counts as ONE (a feature with `@critical @auth` increments both `@critical` and `@auth` by 1, never by 2 even if the tag appears multiple times on the same feature)
  #   6. Files that fail to parse as Gherkin MUST be appended to invalidFiles[] (relative path) and skipped — the command MUST NOT throw on individual file parse failures (parity with the inner try/catch at src/commands/tag-stats.ts:84-89 and the outer try/catch at lines 100-102)
  #   7. When tags.json is loaded, categories MUST be projected in tags.json declaration order. Within each category, tags are sorted by count DESCENDING (TS `b.count - a.count` at line 130). Empty categories (zero counts) are omitted (TS line 132)
  #   8. Tags found in feature files but NOT declared in any tags.json category MUST be collected into a final synthetic category named 'Unregistered' (sorted by count descending). If tags.json is missing/unreadable, ALL used tags go into this same single 'Unregistered' bucket (TS lines 137-159)
  #   9. unusedTags[] MUST contain every registered tag (from tags.json) whose count is zero, sorted ALPHABETICALLY by tag name (`unusedTags.sort()` at TS line 173). When tags.json is missing/unreadable, unusedTags is always an empty array (lines 162-174)
  #   10. The structured result MUST contain exactly the fields {success, totalFiles, uniqueTags, totalOccurrences, categories, unusedTags, tagsFileFound, invalidFiles} in declaration order, matching the TS TagStatsResult interface at src/commands/tag-stats.ts:26-35
  #   11. The text format prints overall counters (`Total feature files:`, `Unique tags used:`, `Total tag occurrences:`) followed by optional warning blocks for missing tags.json and invalid files, then a `Tag Counts by Category` section, then an `Unused Registered Tags` section (parity with src/commands/tag-stats.ts:192-237)
  #   12. The JSON format wraps the result with 2-space indentation (parity with TS `JSON.stringify(result, null, 2)`); --format is NOT exposed at the TS CLI surface but the Rust shared `run()` accepts it for the dispatcher structured-output path
  #   13. The standalone fspec binary MUST expose `tag-stats` as a clap v4 derive subcommand with NO flags — matching the flag-less TS Commander.js registration at src/commands/tag-stats.ts:258-262
  #   14. The clap subcommand action MUST delegate to the same fspec_core::commands::tag_stats::run() function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate counting, projection, or rendering logic in the CLI bridge
  #   15. The CLI wrapper MUST resolve the project root from the current working directory (parity with TS `process.cwd()` default at src/commands/tag-stats.ts:40), exit 0 on success, exit 1 on FspecCoreError, and write structured errors to stderr prefixed with `Error:`
  #   16. `fspec tag-stats --help` MUST be byte-for-byte identical to the TypeScript `formatCommandHelp` reference fixture (captured from `node dist/index.js tag-stats --help`)
  #
  # EXAMPLES:
  #   1. Dispatcher called against an empty tempdir (no spec/) → returns success=true with totalFiles=0, uniqueTags=0, totalOccurrences=0, categories=[], unusedTags=[], tagsFileFound=false, invalidFiles=[]
  #   2. Tempdir has spec/features/a.feature with `@critical @auth` and spec/features/b.feature with `@critical @ui`, no tags.json → uniqueTags=3, totalOccurrences=4, tagsFileFound=false, single 'Unregistered' category sorted by count desc: [@critical(2), @auth(1), @ui(1)]
  #   3. Tempdir has tags.json declaring Phase Tags [@critical, @high] and Component Tags [@cli, @parser], plus features: a.feature(@critical @cli), b.feature(@critical @high), c.feature(@parser) → categories = [Phase Tags: @critical(2)@high(1), Component Tags: @cli(1)@parser(1)], unusedTags=[] (none unused)
  #   4. Tempdir has tags.json with Phase Tags=[@critical, @high, @low] but features only use @critical → unusedTags=['@high','@low'] sorted alphabetically; categories=[Phase Tags: @critical(1)]
  #   5. Tempdir has tags.json declaring @critical and features using @critical AND @undeclared → categories=[Phase Tags: @critical(1), Unregistered: @undeclared(1)], unusedTags=[] (because @critical is used)
  #   6. Tempdir has spec/features/bad.feature with the bytes 'This is not gherkin' → invalidFiles=['spec/features/bad.feature'], totalFiles=1, uniqueTags=0 (file silently skipped from counts)
  #   7. Tempdir has a.feature with `@critical` on Feature: and `@smoke` on a scenario → @critical count=1, @smoke count=0 (scenario-level tags are NOT counted)
  #   8. Tempdir has tags.json containing the bytes '{ not json' → tagsFileFound=false, command does NOT escalate (silent degradation), tags fall through to a single 'Unregistered' bucket
  #   9. Dispatcher with format='json' against tempdir with one tagged feature → DispatchResult.data parses as JSON with success=true and 2-space indented top-level keys in declaration order
  #   10. Dispatcher with format='text' against tempdir without tags.json prints header 'Tag Usage Statistics', counters, then `⚠ Warning: spec/tags.json not found`
  #   11. Dispatcher text format with one bad gherkin file prints '⚠ Warning: 1 file(s) with invalid syntax skipped:' followed by '  - spec/features/bad.feature'
  #   12. Dispatcher text format with tags.json containing unused tags prints 'Unused Registered Tags' section then 'N registered tag(s) not used in any feature file:' then '  @tag' lines alphabetically
  #   13. Running `./codelet/target/release/fspec tag-stats` in an empty directory exits 0 with stdout containing 'Total feature files: 0' and `⚠ Warning: spec/tags.json not found`
  #   14. Running `./codelet/target/release/fspec tag-stats --help` exits 0 and stdout is byte-for-byte identical to codelet/fspec/tests/fixtures/help/tag-stats.txt
  #   15. Running `./codelet/target/release/fspec tag-stats --help` exits 0 with stdout NOT containing '--category', '--format', '--workspace' or '--status' flags
  #   16. Both invocation paths produce equivalent data: (a) dispatch_command('tag-stats', '{"format":"json"}', project_root) and (b) `./codelet/target/release/fspec tag-stats` against the same on-disk state — CLI bridge file contains NO counting/projection/rendering logic
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch tag-stats from the agent loop and run `fspec tag-stats` from a shell, getting the same tag usage statistics as the TypeScript implementation
    So that I can audit tag usage and surface registered-but-unused tags without relying on Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Clap exposes tag-stats as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec tag-stats --help` from a shell
    Then the command exits 0
    Then stdout contains a description of the tag-stats subcommand
    Then stdout does NOT contain the substring '--category'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'
    Then stdout does NOT contain the substring '--status'

  Scenario: CLI against empty directory prints zero-totals output and does not auto-create files
    Given an empty directory with no spec subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec tag-stats` from that directory
    Then the command exits 0
    Then stdout contains the substring 'Total feature files: 0'
    Then stdout contains the substring '⚠ Warning: spec/tags.json not found'
    Then spec/tags.json was NOT created in the directory
    Then spec/features/ was NOT created in the directory

  Scenario: CLI text output renders category counts for the populated case
    Given spec/tags.json declares Phase Tags=[@critical, @high]
    Given spec/features/a.feature has feature-level tags '@critical'
    Given spec/features/b.feature has feature-level tags '@critical @high'
    When I run `./codelet/target/release/fspec tag-stats` from the workspace
    Then the command exits 0
    Then stdout contains the substring 'Tag Usage Statistics'
    Then stdout contains the substring 'Total feature files: 2'
    Then stdout contains the substring 'Unique tags used: 2'
    Then stdout contains the substring 'Total tag occurrences: 3'
    Then stdout contains the substring 'Phase Tags (2 tags)'

  Scenario: CLI text output prints invalid-files warning when a feature file is malformed
    Given spec/features/bad.feature contains the bytes 'not gherkin'
    When I run `./codelet/target/release/fspec tag-stats` from the workspace
    Then the command exits 0
    Then stdout contains the substring '⚠ Warning: 1 file(s) with invalid syntax skipped:'
    Then stdout contains the exact line '  - spec/features/bad.feature'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has tag-stats registered as a clap subcommand
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists tag-stats as an available subcommand
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/tags.json declares Phase Tags=[@critical]
    Given spec/features/a.feature has feature-level tags '@critical'
    When I dispatch tag-stats through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher result has totalFiles=1, uniqueTags=1, totalOccurrences=1
    Then running `./codelet/target/release/fspec tag-stats` against the same on-disk state exits 0 and stdout reports the same counters
    Then the CLI bridge module codelet/fspec/src/tag_stats.rs contains NO inline tag-counting, category-projection, or rendering logic

  Scenario: tag-stats --help is byte-for-byte identical to the TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec tag-stats --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/tag-stats.txt
    And stdout starts with a blank line followed by 'TAG-STATS'
