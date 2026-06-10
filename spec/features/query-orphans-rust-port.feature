@done
@cli
@querying
@RPC-262 @wip
Feature: Port query-orphans command to Rust

  """
  Use ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults if missing) — TS source-of-truth at src/commands/query-orphans.ts:37 calls ensureWorkUnitsFile.
  Read dependency arrays (blocks, blockedBy, dependsOn, relatesTo) from WorkUnit.extra via extra.get(k).and_then(Value::as_array) — do NOT modify shared types/work_unit.rs (file-ownership rule: shared types module is supervisor-only).
  Epic check matches TS wu.epic && wu.epic.trim().length > 0: WorkUnit.epic is Option<String>; treat None or trimmed-empty as no-epic.
  OrphanedWorkUnit struct uses #[derive(Serialize)] with declaration order {id, title, status, suggestedActions} and #[serde(rename_all = "camelCase")]; suggestedActions is a static [&str; 3] literal cloned into a Vec<String> per orphan.
  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::query_orphans::run function; CLI bridge does only JSON arg marshalling and stdout rendering.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via ensure_work_units_file (auto-creates canonical empty store on ENOENT, escalates malformed JSON via ParseJson)
  #   2. A work unit is orphaned only when it has NEITHER a non-blank epic NOR any non-empty relationship array
  #   3. Epic 'has' check uses string trim
  #   4. Relationship 'has' check uses array length
  #   5. Each orphan entry includes id, title, status, and a fixed suggestedActions array
  #   6. Result JSON field order is declaration order
  #   7. Orphans appear in IndexMap insertion order
  #   8. Text output (default) emits '✓ No orphaned work units found.' when empty
  #   9. CLI exit codes: 0 on success, 1 on error
  #  10. --exclude-done flag skips orphans whose status is 'done'
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of query-orphans wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one orphan-detection implementation with byte-parity to the TS source

  Scenario: Returns empty orphans array when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch query-orphans with output='json'
    Then the dispatcher returns success=true
    And the returned JSON has orphans=[]
    And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: A work unit with a non-blank epic and no relationships is not orphaned
    Given spec/work-units.json contains A with epic='auth' and no relationship arrays
    When I dispatch query-orphans with output='json'
    Then the returned JSON has orphans=[]

  Scenario: A work unit with no epic but a non-empty blocks array is not orphaned
    Given spec/work-units.json contains A with no epic and blocks=['X']
    When I dispatch query-orphans with output='json'
    Then the returned JSON has orphans=[]

  Scenario: A work unit with no epic and no relationships is orphaned
    Given spec/work-units.json contains A with no epic and no relationship arrays
    When I dispatch query-orphans with output='json'
    Then the returned JSON has orphans containing exactly one entry whose id='A'
    And that entry has status equal to A's status and suggestedActions=['Assign epic','Add relationship','Delete']

  Scenario: A work unit with epic='   ' (whitespace) is treated as no epic and is orphaned
    Given spec/work-units.json contains A with epic='   ' and no relationship arrays
    When I dispatch query-orphans with output='json'
    Then the returned JSON has orphans containing exactly one entry whose id='A'

  Scenario: Empty arrays for all four relationships are treated as no relationships
    Given spec/work-units.json contains A with no epic and blocks=[] blockedBy=[] dependsOn=[] relatesTo=[]
    When I dispatch query-orphans with output='json'
    Then the returned JSON has orphans containing exactly one entry whose id='A'

  Scenario: --exclude-done skips done orphans while default includes them
    Given spec/work-units.json contains DONE-1 with status='done' and no epic and no relationships, and OPEN-1 with status='backlog' and no epic and no relationships
    When I dispatch query-orphans with output='json' and excludeDone=false
    Then the returned JSON has orphans containing both DONE-1 and OPEN-1
    When I dispatch query-orphans with output='json' and excludeDone=true
    Then the returned JSON has orphans containing only OPEN-1

  Scenario: Multiple orphans appear in IndexMap insertion order
    Given spec/work-units.json contains FIRST-1 then SECOND-1 then THIRD-1 all with no epic and no relationships
    When I dispatch query-orphans with output='json'
    Then orphans[0].id='FIRST-1' and orphans[1].id='SECOND-1' and orphans[2].id='THIRD-1'

  Scenario: JSON field declaration order is preserved
    Given spec/work-units.json contains a single orphaned work unit ORPH-1
    When I dispatch query-orphans with output='json'
    Then the orphan object's field declaration order is id, title, status, suggestedActions

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch query-orphans against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
