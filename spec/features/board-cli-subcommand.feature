@done
@rust
@querying
@cli
@RPC-199
Feature: board CLI subcommand
  """
  Core impl: rewrite rust/fspec-core/src/commands/board.rs to `pub async fn run(args_json, project_root)`; reuse io::ensure::{check_foundation_exists, ensure_work_units_file} and types::work_unit::{WorkUnitsData, WorkUnitStates}. WorkUnit.estimate is read from the `extra` map (no typed estimate field exists).
  columns/board are JSON objects keyed by status; emit in WorkUnitStates declaration order (backlog, specifying, testing, implementing, validating, done, blocked) to match TS Object.entries(states) on canonical files. SUPERVISOR must wire: canonical PORTED_COMMANDS, dispatch run_ported, main.rs Mode::Board{format,limit} + intercept + mod, help configs/mod.rs. OPEN QUESTION for supervisor: confirm headless text-mode rendering string OR make CLI default json (TS text mode is an Ink TUI with no stable fixture).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Requires foundation.json: calls check_foundation_exists(project_root, 'fspec board') first; missing foundation returns the verbatim FoundationMissing error
  #   2. Auto-creates spec/work-units.json via ensure_work_units_file when missing (does NOT swallow parse errors)
  #   3. Builds columns (per status: list of {id, title?, estimate?}) and board (per status: list of ids) iterating states in on-disk key order; title and estimate keys are omitted when absent
  #   4. Sums story points: estimate of a done work unit adds to completedPoints, any other status adds to inProgressPoints; units without an estimate contribute nothing
  #   5. summary string is exactly 'N points in progress, M points completed'; the JSON format is JSON.stringify({columns, board, summary}, null, 2) (2-space indent)
  #   6. The clap subcommand exposes --format <format> (default text) and --limit <limit> (default 25); the interactive Ink TUI text mode is replaced by a deterministic headless text rendering (TUI stays a combined-mode concern, out of scope per list-* precedent)
  #   7. Both front doors call the single fspec_core::commands::board::run function
  #
  # EXAMPLES:
  #   1. Given work-units.json with AUTH-001 (done, estimate 5) and AUTH-002 (implementing, estimate 3), when board runs with format=json, summary reads '3 points in progress, 5 points completed' and columns.done[0] is {id:AUTH-001, title, estimate:5}
  #   2. Given a work unit with no estimate field, when board runs with format=json, its column entry omits the estimate key and it contributes 0 to both point totals
  #   3. Given a project root with no spec/foundation.json, when board runs, it fails with the foundation-missing error and the CLI exits 1
  #   4. Given foundation.json exists but no work-units.json, when board runs with format=json, work-units.json is auto-created and all seven columns are present and empty with summary '0 points in progress, 0 points completed'
  #   5. Given the binary is run with 'board --help', output is byte-for-byte identical to the captured bare Commander.js help fixture (board has no custom -help.ts in TS), exit 0
  #
  # ========================================
  Background: User Story
    As a developer porting fspec to Rust
    I want to run board through the LLM dispatcher and the standalone Rust CLI
    So that the Kanban board JSON shape (columns/board/summary with story-point totals) has byte-parity with the TypeScript implementation

  Scenario: Clap exposes board with --format and --limit and prints byte-parity help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec board --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/board.txt
    Then stdout starts with the Commander usage line 'Usage: fspec board [options]'

  Scenario: CLI emits JSON board with story-point summary
    Given a project root whose spec/foundation.json exists and spec/work-units.json contains AUTH-001 (done, estimate 5) and AUTH-002 (implementing, estimate 3)
    When I run `./rust/target/release/fspec board --format json` from that directory
    Then the command exits 0
    Then stdout parses as JSON whose summary field reads '3 points in progress, 5 points completed'

  Scenario: CLI exits 1 with stderr when foundation.json is missing
    Given a project root with no spec/foundation.json
    When I run `./rust/target/release/fspec board --format json` from that directory
    Then the command exits 1
    Then stderr describes the missing foundation

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/foundation.json exists and spec/work-units.json contains AUTH-001 (done, estimate 5)
    When I dispatch board through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher's DispatchResult.data summary matches the CLI's JSON summary against the same on-disk state
    Then the CLI bridge module rust/fspec/src/board.rs contains NO inline column-building, point-summing, or rendering logic — its only computation is JSON arg marshalling
