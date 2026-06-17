@done
@querying
@cli
@RPC-309
Feature: Port suggest-dependencies command to Rust

  """
  Core impl in codelet/fspec-core/src/commands/suggest_dependencies.rs: Args {output: Option<String>}; reads via ensure_work_units_file(project_root); iterates data.work_units.values() (IndexMap insertion order). Suggestion struct #[derive(Serialize)] #[serde(rename_all=camelCase)] decl order from,to,type(r#type renamed 'type'),reason,confidence.
  WorkUnit fields used: id, title (typed); dependsOn/blockedBy read from extra via extra.get(field).and_then(Value::as_array). Do NOT touch shared types/work_unit.rs. Circular tiebreak uses Rust &str < comparison = JS string compare for ASCII ids.
  Two-front-doors: CLI bridge codelet/fspec/src/suggest_dependencies.rs marshals --output into args_json and renders stdout; dispatcher passes args_json verbatim. Help config codelet/fspec-core/src/help/configs/suggest_dependencies.rs. No new shared helpers required.
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

  Scenario: Returns empty suggestions array when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch suggest-dependencies with output='json'
    Then the dispatcher returns success=true
    And the returned JSON has suggestions=[]
    And spec/work-units.json exists after the call

  Scenario: Sequential IDs in the same prefix produce a medium-confidence dependsOn suggestion
    Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has a suggestion with from='AUTH-002' to='AUTH-001' type='dependsOn' confidence='medium'
    And that suggestion reason contains 'sequential IDs in AUTH prefix'

  Scenario: A Test work unit depends on a matching Build work unit with high confidence
    Given spec/work-units.json contains BUILD-001 titled 'Build authentication' and TEST-001 titled 'Test authentication'
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has a suggestion with from='TEST-001' to='BUILD-001' type='dependsOn' confidence='high'
    And that suggestion reason contains 'test work depends on build work'

  Scenario: A feature work unit depends on a same-prefix infrastructure work unit with high confidence
    Given spec/work-units.json contains FEAT-001 titled 'Database schema setup' and FEAT-002 titled 'Add user features'
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has a suggestion with from='FEAT-002' to='FEAT-001' type='dependsOn' confidence='high'
    And that suggestion reason contains 'infrastructure work (schema/migration) should complete before feature work'

  Scenario: Specific patterns override the generic sequential suggestion for the same pair
    Given spec/work-units.json contains BUILD-001 titled 'Build authentication' and BUILD-002 titled 'Test authentication'
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has exactly one suggestion with from='BUILD-002' to='BUILD-001'
    And that suggestion confidence='high'

  Scenario: Existing dependsOn relationship excludes the sequential suggestion
    Given spec/work-units.json contains AUTH-001 and AUTH-002 where AUTH-002 already lists AUTH-001 in dependsOn
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has suggestions=[]

  Scenario: The unimplemented same-epic relatesTo rule produces no suggestions
    Given spec/work-units.json contains XX-001 and YY-001 in epic 'auth' with different prefixes and no relationships
    When I dispatch suggest-dependencies with output='json'
    Then the returned JSON has suggestions=[]

  Scenario: JSON suggestion field declaration order is from, to, type, reason, confidence
    Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    When I dispatch suggest-dependencies with output='json'
    Then the first suggestion object's field declaration order is from, to, type, reason, confidence

  Scenario: Default text output renders a numbered summary
    Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    When I dispatch suggest-dependencies with default text output
    Then the rendered text contains 'Found 1 dependency suggestion(s):'
    And the rendered text contains 'AUTH-002'
    And the rendered text contains 'Confidence: MEDIUM'

  Scenario: Default text output renders the empty sentinel when there are no suggestions
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch suggest-dependencies with default text output
    Then the rendered text contains 'No dependency suggestions found.'

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch suggest-dependencies against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
