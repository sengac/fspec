@done
@querying
@cli
@RPC-224
Feature: Port dependencies command to Rust

  """
  Result envelope: TS showDependencies returns a String (the rendered text). The dispatcher path returns {success:true, data:<string>} on success, and {success:false, error:<msg>} when the work unit is missing. Recoverable 'does not exist' error: in TS it throws; in Rust mirror query-style by returning FspecCoreError so the dispatcher maps to success=false (substring 'does not exist').
  Read work-units.json directly with std::fs (NO ensure/auto-create) to match TS loadWorkUnits readFile+JSON.parse. Reuse crate::types::work_unit::WorkUnitsData + WorkUnit (IndexMap insertion order). Relationship arrays read from typed/extra: prefer workUnit.extra legacy top-level fields then relationships object — read both as serde_json arrays of strings, do NOT add typed fields to shared types/work_unit.rs (supervisor-only).
  Help intercept: --help renders format_command_help(&configs::dependencies::CONFIG); fixture captured from `node dist/index.js dependencies --help`. clap variant Mode::Dependencies { work_unit_id: String, graph: bool } (--graph long only, no short). Bridge marshals {workUnitId, graph} into args_json omitting graph when false so #[serde(default)] fires.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json by reading the file directly (matching TS loadWorkUnits which does readFile + JSON.parse with no auto-create); a missing file surfaces as an error and malformed JSON escalates
  #   2. Errors when the requested work unit id does not exist in workUnits, with a message containing 'does not exist' (TS throws Error(`Work unit '${id}' does not exist`)); the CLI bridge surfaces an AI-friendly system-reminder and exits 1
  #   3. Default (non-graph) output is the literal header 'Dependencies for <id>:' followed by one indented line per non-empty relationship type in fixed order: '  Blocks: a, b', '  Blocked by: ...', '  Depends on: ...', '  Related to: ...'; empty relationship types are omitted entirely
  #   4. Relationship values are read from workUnit.relationships.{blocks,blockedBy,dependsOn,relatesTo}, falling back to legacy top-level workUnit.{blocks,blockedBy,dependsOn,relatesTo} fields when relationships are absent (TS: workUnit.blocks || workUnit.relationships?.blocks || [])
  #   5. With --graph, output is a depth-first tree starting at the work unit id, recursing only through the 'blocks' relationship; each blocked child is rendered on its own indented line as '  blocks → <id>' with two-space deeper indentation per level, and a visited set prevents infinite loops (each id printed at most once)
  #   6. Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::dependencies::run function; the clap subcommand exposes one positional <work-unit-id> argument and a boolean --graph flag (default false); CLI bridge does only JSON arg marshalling and stdout rendering
  #
  # EXAMPLES:
  #   1. Dispatch dependencies for AUTH-001 (blocks=['AUTH-002','AUTH-003'], blockedBy=['INFRA-001'], dependsOn=['SCHEMA-001'], relatesTo=['DOC-001']) returns the header plus all four lines in fixed order
  #   2. Dispatch dependencies for a work unit with no relationships returns exactly 'Dependencies for MCP-001:\n' with no relationship lines
  #   3. Dispatch dependencies for AUTH-001 --graph where AUTH-001 blocks AUTH-002 (which blocks AUTH-004) and AUTH-003 renders the indented tree AUTH-001 / blocks → AUTH-002 / blocks → AUTH-004 / blocks → AUTH-003
  #   4. Dispatch dependencies for INVALID-999 (absent from workUnits) returns success=false with an error message containing 'does not exist'
  #   5. CLI: `fspec dependencies INVALID-999` exits 1 and writes 'does not exist' to stderr
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec dependencies <work-unit-id>` (optionally with --graph) through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one read-only dependency-display implementation with byte-parity to the TS source

  Scenario: Dispatch dependencies for a unit with all four relationship types renders the header and all four lines in fixed order
    Given spec/work-units.json contains AUTH-001 whose relationships are blocks=['AUTH-002','AUTH-003'], blockedBy=['INFRA-001'], dependsOn=['SCHEMA-001'], relatesTo=['DOC-001']
    When I dispatch dependencies with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the data field equals "Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001\n"

  Scenario: Dispatch dependencies for a unit with no relationships renders only the header line
    Given spec/work-units.json contains MCP-001 with no relationship fields
    When I dispatch dependencies with workUnitId='MCP-001'
    Then the dispatcher returns success=true
    Then the data field equals "Dependencies for MCP-001:\n"

  Scenario: Dispatch dependencies with --graph renders a depth-first blocks tree with increasing indentation
    Given spec/work-units.json contains AUTH-001 with blocks=['AUTH-002','AUTH-003'], AUTH-002 with blocks=['AUTH-004'], and AUTH-003 and AUTH-004 with no relationships
    When I dispatch dependencies with workUnitId='AUTH-001' and graph=true
    Then the dispatcher returns success=true
    Then the data field equals "AUTH-001\n  blocks → AUTH-002\n    blocks → AUTH-004\n  blocks → AUTH-003"

  Scenario: Relationship values fall back to legacy top-level fields when relationships object is absent
    Given spec/work-units.json contains LEGACY-001 with top-level dependsOn=['LEGACY-002'] and no relationships object
    When I dispatch dependencies with workUnitId='LEGACY-001'
    Then the dispatcher returns success=true
    Then the data field equals "Dependencies for LEGACY-001:\n  Depends on: LEGACY-002\n"

  Scenario: Dispatch dependencies for a missing work unit returns a structured does-not-exist error
    Given spec/work-units.json contains AUTH-001 only
    When I dispatch dependencies with workUnitId='INVALID-999'
    Then the dispatcher returns success=false with an error message containing the substring 'does not exist'
