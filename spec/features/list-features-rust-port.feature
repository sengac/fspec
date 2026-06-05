@done
@querying
@cli
@rust
@RPC-245
Feature: Port list-features command to Rust

  """
  Uses an inline hand-rolled scanner in `parse_feature_header` rather than the upstream `gherkin` crate — deliberate divergence to keep the dep tree tight while maintaining the same public surface as the TypeScript implementation.
  New shared helper `io::feature_glob::glob_feature_files(cwd) -> Result<Vec<String>, FspecCoreError>` returns sorted forward-slash relative paths for every `spec/features/**/*.feature` match. Uses std walk + manual filtering (no extra glob crate dep) to keep the dep tree tight.
  New `commands/list_features.rs` with: `ListFeaturesArgs { tag: Option<String>, format: Option<String> }`, `FeatureInfo { file: String, name: String, scenario_count: usize, tags: Vec<String> }` (serde rename_all=camelCase), `run(args_json, project_root) -> Result<String, FspecCoreError>`. Empty-spec/features error MUST escalate; parse errors MUST be silently swallowed (eprintln warning OK).
  New `FspecCoreError::DirectoryNotFound { path: String }` variant — escalated when spec/features/ does not exist. Its Display impl MUST contain the exact substring 'Directory not found: spec/features/' (parity with TS error message). The CLI bridge inspects this substring to choose exit code 2 vs 1.
  CLI bridge `codelet/fspec/src/list_features.rs`: `pub struct CliArgs { pub tag: Option<String> }`, `pub async fn run(args: CliArgs) -> Result<u8>`. Marshals args into JSON via serde_json::Map, calls fspec_core::commands::list_features::run, prints rendered text to stdout. On error: prints 'Error: <msg>' to stderr; exit code 2 if the error message contains 'Directory not found', else 1. NO inline glob/parse/filter/sort/render code.
  Shared-file additions: (1) io/mod.rs — `pub mod feature_glob;`, (2) error.rs — `DirectoryNotFound { path }` variant, (3) canonical.rs PORTED_COMMANDS — add `"list-features"`, (4) dispatch.rs run_ported — add `"list-features" => commands::list_features::run(...)` arm AND remove its line from run_stub, (5) main.rs — `mod list_features;`, `Mode::ListFeatures { tag: Option<String> }` variant + arm, (6) cargo_shape.rs locked-file list — extend 8→9 to add `list_features.rs`. No new Cargo.toml dep is required because parsing is performed by the inline scanner.
  File-glob ordering note: TypeScript uses tinyglobby which returns paths with forward slashes regardless of platform; the Rust port MUST normalise to forward slashes (replace '\\' with '/') so the `file` field matches TS output byte-for-byte on Windows and the alphabetical sort is platform-stable.
  Estimate justification: 5 points (complex). Inline gherkin scanner, new shared helper module, new error variant, two new feature files, dispatcher arm, CLI bridge, lock-list update. Similar shape to list-prefixes (RPC-248, 5pts) but with the parse-error-swallowing nuance pushing complexity up rather than down.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-features` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by list-prefixes (RPC-248) and list-work-units (RPC-253)
  #   2. If spec/features/ does NOT exist, the command MUST escalate a structured error containing the substring 'Directory not found: spec/features/' (parity with the TS throw at src/commands/list-features.ts:36-37); this is NOT swallowed
  #   3. If spec/features/ exists but is EMPTY (no .feature files), the command MUST return success with an empty features array (parity with TS short-circuit at src/commands/list-features.ts:53-55)
  #   4. For each .feature file under spec/features/ (recursive glob), parse with a Gherkin parser; on parse error the file MUST be silently SKIPPED (parity with TS `output.warn(...)` bare-catch at src/commands/list-features.ts:92-95) — parse failures are NOT escalated
  #   5. Each emitted feature entry MUST contain `file` (string, relative to cwd, forward-slash), `name` (string from feature.name), `scenarioCount` (count of top-level scenarios — NOT nested rule scenarios, NOT background), and `tags` (array of strings with leading '@' character preserved) — matching the TS FeatureInfo interface exactly
  #   6. When `tag` arg is supplied, the command MUST filter to only features whose tags array contains that EXACT string (leading '@' included) — TS uses `tags.includes(options.tag)` at src/commands/list-features.ts:76; tag normalisation (auto-adding '@') is NOT performed
  #   7. Output MUST be sorted alphabetically by `file` field (parity with `features.sort((a,b) => a.file.localeCompare(b.file))` at src/commands/list-features.ts:99); deterministic ordering regardless of glob traversal order
  #   8. The text format (default) prints 'No feature files found in spec/features/' for empty/filtered-empty result; populated lists print one line per feature as '  <file> - <name> (<N> scenarios)[ [<tag1> <tag2>]]' followed by a blank line and the summary 'Found <N> feature files' (or 'Found <N> feature files matching <tag>' when --tag is set) — parity with src/commands/list-features.ts:116-134
  #   9. The JSON format wraps the result in `{ features: [...] }` with 2-space indentation (parity with the dispatcher's structured-output path used by list-prefixes); --format is NOT exposed at the TS CLI surface but the Rust shared `run()` accepts it for the dispatcher path
  #   10. The standalone fspec binary at codelet/fspec/src/main.rs MUST expose `list-features` as a clap v4 derive subcommand with ONE flag: `--tag <TAG>` (matching the single TS Commander.js `.option('--tag <tag>', ...)` registration at src/commands/list-features.ts:156); no --format, no --cwd, no --workspace
  #   11. The clap subcommand action MUST delegate to the same fspec_core::commands::list_features::run() function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate glob, parsing, filter, sorting, or rendering logic in the CLI bridge
  #   12. The CLI wrapper MUST resolve the project root from CWD (parity with TS process.cwd() default at src/commands/list-features.ts:29); exit 0 on success, exit 2 when the error message contains 'Directory not found' (parity with TS src/commands/list-features.ts:138-145), exit 1 on any other FspecCoreError; structured errors written to stderr prefixed with `Error:`
  #   13. Shared infrastructure MUST be reused: a new helper `io::feature_glob::glob_feature_files(cwd) -> Result<Vec<String>, FspecCoreError>` provides the spec/features/**/*.feature listing; gherkin parsing is performed by an inline scanner module-private to commands/list_features.rs (rather than the gherkin crate) to keep the dep tree tight. Parse 'failures' in this context means lines that cannot be classified as Feature/Background/Scenario/Tag/Comment/Blank.
  #
  # EXAMPLES:
  #   1. Dispatch list-features against a tempdir with NO spec/ directory → dispatcher returns success=false with error message containing 'Directory not found: spec/features/'
  #   2. Tempdir has empty spec/features/ directory with no .feature files → dispatcher returns success=true, JSON data parses to {features: []}
  #   3. Tempdir has spec/features/auth.feature (3 scenarios, tags @critical @auth) and spec/features/billing.feature (1 scenario, tag @billing) → dispatcher returns both features sorted by file (auth before billing), with correct scenarioCount and tags arrays preserving the leading '@'
  #   4. spec/features/ has auth.feature (tagged @critical) and billing.feature (tagged @billing); dispatch with tag='@critical' → result contains ONLY auth.feature
  #   5. spec/features/ contains valid-feature.feature (2 scenarios) and broken.feature (invalid gherkin syntax) → dispatcher succeeds, result contains only valid-feature; broken.feature is silently skipped (no throw, no error escalation)
  #   6. spec/features/zebra.feature, alpha.feature, mango.feature (each 1 scenario) — dispatcher returns features in alphabetical-by-file order: alpha, mango, zebra
  #   7. Dispatch with format='text' against empty spec/features/ → DispatchResult.data is exactly 'No feature files found in spec/features/'
  #   8. Dispatch with format='text' against spec/features/ containing one auth.feature (name 'User Authentication', 2 scenarios, tags @critical @auth) → output contains line '  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]' followed by blank line and 'Found 1 feature files'
  #   9. Dispatch with format='text' and tag='@critical' filter, one matching feature → summary line is 'Found 1 feature files matching @critical' (not the unfiltered variant)
  #   10. Dispatch format='json' with one feature → DispatchResult.data parses as JSON with `features` array containing one entry whose fields are file, name, scenarioCount, tags — using 2-space indentation
  #   11. Running `./codelet/target/release/fspec list-features --help` prints clap-generated help showing the --tag flag and NOT --status/--prefix/--epic/--format/--workspace
  #   12. Running `./codelet/target/release/fspec list-features` from an empty directory (no spec/) prints `Error: ... Directory not found: spec/features/` to stderr and exits with code 2 (NOT 1 — parity with TS exit-code 2 branch at lines 138-145)
  #   13. Running `./codelet/target/release/fspec list-features --tag @critical` against a populated spec/features/ where one feature is tagged @critical → exit 0; stdout contains 'Found 1 feature files matching @critical' and lists the matching feature only
  #   14. Both invocation paths produce the SAME structured data: (a) dispatch_command('list-features', {format:'json',tag:'@critical'}, project_root) and (b) `./codelet/target/release/fspec list-features --tag @critical` against the same on-disk state — only the rendering and delivery channel differ
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to list feature files with their names, scenario counts, and tags — optionally filtered by tag — from both the LLM-facing agent-loop dispatcher and the shell-facing CLI subcommand
    So that I can browse the project's living documentation without depending on the TypeScript Node implementation, sharing one source of truth between both invocation paths

  Scenario: Escalates a structured error when spec/features/ does not exist
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-features command against that project root with format='json'
    Then the dispatcher returns success=false with an error message containing the substring 'Directory not found: spec/features/'

  Scenario: Returns an empty features list when spec/features/ exists but contains no .feature files
    Given a project root containing an empty spec/features/ directory
    When I dispatch the list-features command with format='json'
    Then the dispatcher returns success=true with an empty features array

  Scenario: Aggregates feature names, scenario counts, and tags sorted by file path
    Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 3 scenarios
    And spec/features/billing.feature exists with name 'Billing', tags '@billing' and 1 scenario
    When I dispatch list-features with format='json'
    Then the features array contains exactly two entries in order spec/features/auth.feature then spec/features/billing.feature
    And the auth entry has scenarioCount=3 and tags exactly ['@critical', '@auth']
    And the billing entry has scenarioCount=1 and tags exactly ['@billing']

  Scenario: Filters features by exact tag match including the leading '@'
    Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    And spec/features/billing.feature exists with tag '@billing' and 1 scenario
    When I dispatch list-features with format='json' and tag='@critical'
    Then the features array contains exactly one entry whose file is spec/features/auth.feature

  Scenario: Silently skips files that fail to parse without escalating
    Given spec/features/valid-feature.feature contains a parseable feature with 2 scenarios
    And spec/features/broken.feature contains the malformed bytes 'not a feature file'
    When I dispatch list-features with format='json'
    Then the dispatcher returns success=true
    And the features array contains exactly one entry whose file is spec/features/valid-feature.feature

  Scenario: Sorts features alphabetically by file path regardless of glob order
    Given spec/features/zebra.feature, spec/features/alpha.feature and spec/features/mango.feature each contain one scenario
    When I dispatch list-features with format='json'
    Then the features array file values are in order spec/features/alpha.feature, spec/features/mango.feature, spec/features/zebra.feature

  Scenario: Text format prints sentinel for empty results
    Given a project root containing an empty spec/features/ directory
    When I dispatch list-features with format='text'
    Then the DispatchResult.data is exactly the string 'No feature files found in spec/features/'

  Scenario: Text format renders a populated listing with header line and unfiltered summary
    Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 2 scenarios
    When I dispatch list-features with format='text'
    Then the DispatchResult.data contains the exact line '  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]'
    And the DispatchResult.data contains the exact line 'Found 1 feature files'

  Scenario: Text format with a tag filter uses the matching summary phrasing
    Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    When I dispatch list-features with format='text' and tag='@critical'
    Then the DispatchResult.data contains the exact line 'Found 1 feature files matching @critical'

  Scenario: JSON format emits a 2-space indented payload with the canonical field set
    Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical' and 2 scenarios
    When I dispatch list-features with format='json'
    Then the DispatchResult.data parses as JSON whose root object has a 'features' array of length 1
    And the first features entry contains fields file='spec/features/auth.feature', name='User Authentication', scenarioCount=2, tags=['@critical']
    And the DispatchResult.data uses 2-space indentation

  Scenario: Shared infrastructure modules exist under fspec-core for reuse by other gherkin-aware commands
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/
    Then the module io::feature_glob::glob_feature_files exists and is publicly accessible from the crate root
    And the error::FspecCoreError enum declares a DirectoryNotFound variant whose Display contains the substring 'Directory not found'
    And list_features::run delegates to these shared modules rather than embedding its own filesystem-walk logic
