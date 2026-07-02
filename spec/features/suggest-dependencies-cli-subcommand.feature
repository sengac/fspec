@done
@querying
@cli
@RPC-309
Feature: suggest-dependencies clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `suggest-dependencies` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → codelet/fspec/src/suggest_dependencies.rs → fspec_core::commands::suggest_dependencies::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::suggest_dependencies::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes `--output <text|json>` defaulting to `text`.
  Text format prints a numbered summary; the empty case prints 'No dependency suggestions found.' verbatim.
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::suggest_dependencies::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/suggest-dependencies.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via ensure_work_units_file (auto-creates canonical empty store on ENOENT, escalates malformed JSON via ParseJson with 'Failed to parse work-units.json')
  #   2. Build/Test pairs (Rule 3) are HIGH confidence and evaluated first: a 'Test X' work unit dependsOn a 'Build X' work unit when the build title contains the test target
  #   3. Infrastructure-before-features (Rule 4) is HIGH confidence: a feature work unit (title starts with add/create/implement/build) dependsOn a same-prefix infra work unit (title contains schema/migration/database schema/setup/infrastructure)
  #   4. Sequential IDs (Rule 2) are MEDIUM confidence FALLBACK: within a prefix, units sorted by numeric ID suffix, each dependsOn its predecessor, skipped when a specific pattern already matched the pair
  #   5. Existing relationships are excluded: a suggestion is never produced when from already lists to in its dependsOn or blockedBy arrays
  #   6. Circular suggestions (Rule 5) are filtered: when a reverse suggestion exists for the same pair, only the one whose from < to (lexicographic) is kept
  #   7. The JSDoc-documented 'same epic -> relatesTo' rule is NOT implemented in the TS source; the Rust port must mirror TS and emit only dependsOn suggestions
  #   8. output='json' returns pretty-printed JSON {suggestions:[{from,to,type,reason,confidence}]} in declaration order; default text prints a numbered summary or 'No dependency suggestions found.' when empty
  #   9. CLI exit codes: 0 on success, 1 on any FspecCoreError with stderr prefixed '✗ Failed to suggest dependencies:'
  #
  # EXAMPLES:
  #   1. AUTH-001 and AUTH-002 with no relationships -> suggests AUTH-002 dependsOn AUTH-001 (sequential, MEDIUM)
  #   2. 'Build authentication' (BUILD-001) and 'Test authentication' (TEST-001) -> suggests TEST-001 dependsOn BUILD-001 (build/test, HIGH)
  #   3. 'Database schema setup' (FEAT-001) and 'Add user features' (FEAT-002) same prefix -> suggests FEAT-002 dependsOn FEAT-001 (infrastructure, HIGH)
  #   4. Empty workspace (no spec/) -> auto-creates work-units.json and returns suggestions=[] with text 'No dependency suggestions found.'
  #   5. AUTH-002 already lists AUTH-001 in dependsOn -> no sequential suggestion produced for that pair
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of suggest-dependencies wired through both the LLM dispatcher and the clap subcommand
    So that the standalone Rust binary and the daemon share one dependency-suggestion implementation

  Scenario: Clap exposes suggest-dependencies as a subcommand and prints --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec suggest-dependencies --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'suggest-dependencies'

  Scenario: CLI without options prints the empty sentinel against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec suggest-dependencies` from that directory
    Then the command exits 0
    And stdout contains the substring 'No dependency suggestions found.'

  Scenario: CLI with --output=json prints empty suggestions array
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec suggest-dependencies --output json` from that directory
    Then the command exits 0
    And stdout parses as JSON whose root object has suggestions=[]

  Scenario: CLI text output lists a sequential suggestion
    Given a workspace whose spec/work-units.json contains AUTH-001 and AUTH-002 with no relationships
    When I run `./codelet/target/release/fspec suggest-dependencies` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Found 1 dependency suggestion(s):'
    And stdout contains the substring 'AUTH-002'
    And stdout contains the substring 'Confidence: MEDIUM'

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec suggest-dependencies --output json` from that directory
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace whose spec/work-units.json contains AUTH-001 and AUTH-002 with no relationships
    When I dispatch suggest-dependencies through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    And I run `./codelet/target/release/fspec suggest-dependencies --output json` against the same workspace
    Then both invocations produce JSON with a suggestion from='AUTH-002' to='AUTH-001'
    And the CLI bridge module codelet/fspec/src/suggest_dependencies.rs contains NO inline suggestion logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: suggest-dependencies --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec suggest-dependencies --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/suggest-dependencies.txt
    And stdout starts with a blank line followed by 'SUGGEST-DEPENDENCIES'
