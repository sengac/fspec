@done
@cli
@querying
@rust
@RPC-301
Feature: Port show-deleted command to Rust

  """
  File layout: codelet/fspec-core/src/commands/show_deleted.rs (impl) + codelet/fspec-core/src/help/configs/show_deleted.rs (help config) + codelet/fspec-core/tests/show_deleted.rs (dispatcher tests) + codelet/fspec/src/show_deleted.rs (CLI bridge) + codelet/fspec/tests/cli_show_deleted.rs (CLI shell tests) + codelet/fspec/tests/fixtures/help/show-deleted.txt (TS help fixture)
  Reuses existing io::ensure::ensure_work_units_file (load-or-init) — NOT the read-or-empty twin. The TS show-deleted explicitly uses ensureWorkUnitsFile (src/commands/show-deleted.ts:32), so missing spec/work-units.json gets created with the canonical empty initial structure before the work-unit-existence check runs
  WorkUnit.rules/examples/questions/architectureNotes are NOT modelled on the shared types::work_unit::WorkUnit struct — they round-trip through wu.extra: serde_json::Map. show_deleted.rs deserializes them inline using a private struct DeletedItemRaw { id: u64, text: String, #[serde(default)] deleted: bool, #[serde(default, skip_serializing_if='Option::is_none') ] deleted_at: Option<String> }. This keeps the shared WorkUnit type minimal and parallel-port-safe
  Two-front-doors per RPC-003 §7/§11: shell argv → clap → codelet/fspec/src/show_deleted.rs → codelet_fspec_core::commands::show_deleted::run; LLM tool call JSON → fspec_core::dispatch::dispatch_command → codelet_fspec_core::commands::show_deleted::run. Both call sites pass JSON-encoded args and project_root: &Path. CLI bridge marshals workUnitId into the JSON shape and does NOT exposeing --format (TS doesn't)
  JSON structured shape (dispatcher path with format=json): { success: bool, workUnitId: string, deletedItems: [{id, text, deletedAt?}], totalDeleted: number }. Use #[derive(Serialize)] with declaration-order fields so 2-space-indented JSON.stringify-equivalent is preserved (BTreeMap would alphabetize)
  Shared-file wiring needed from supervisor AFTER worker impl lands: (1) codelet/fspec-core/src/help/configs/mod.rs add `pub mod show_deleted;`; (2) codelet/fspec-core/src/dispatch.rs move show-deleted from run_stub to run_ported; (3) codelet/fspec-core/src/canonical.rs add show-deleted to is_ported whitelist if such exists; (4) codelet/fspec/src/main.rs add Mode::ShowDeleted clap variant + dispatch arm; (5) codelet/fspec-core/src/commands/mod.rs already declares pub mod show_deleted (stub)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for show-deleted MUST replace the NotYetPorted stub and return a real DispatchResult through the same poll_sync_future path used by list-prefixes (RPC-248)
  #   2. The command MUST require a workUnitId argument (positional on the CLI, required JSON field on the dispatcher) — parity with TS Commander.js argument('<workUnitId>', 'Work unit ID') at src/commands/show-deleted.ts:76
  #   3. Missing spec/work-units.json MUST be auto-created via the shared ensure_work_units_file helper (parity with TS ensureWorkUnitsFile at src/commands/show-deleted.ts:32) — show-deleted has load-or-init semantics, NOT the read-or-empty semantics used by list-prefixes
  #   4. If the requested workUnitId does not exist in workUnits, the dispatcher MUST return success=false with an error message containing the substring 'Work unit '<id>' does not exist' (parity with the TS throw at src/commands/show-deleted.ts:36)
  #   5. Deleted items MUST be collected from rules, examples, questions, and architectureNotes arrays in that exact concatenation order — within each array preserving on-disk array order (parity with TS L42-63)
  #   6. Only items whose deleted flag is true MUST appear in the result; items without a deleted field or with deleted=false MUST be excluded (parity with .filter(x => x.deleted) at TS L43, L47, L51, L55)
  #   7. Each deletedItem entry MUST contain exactly id (number), text (string), and optional deletedAt (string when present, omitted otherwise) — parity with TS .map at L44, L48, L52, L56; createdAt, selected, answered, and answer fields MUST be dropped
  #   8. The empty case (totalDeleted=0) MUST render the literal string 'No deleted items found' as the only output (parity with TS L82-84)
  #   9. The populated case MUST render a leading blank line, a header 'Deleted items in <workUnitId> (<N> total):', one line per item formatted '  [<id>] <text>' suffixed with ' (deleted: <iso>)' when deletedAt is set, and a trailing blank line (parity with TS L86-100)
  #   10. The standalone fspec Rust binary MUST expose show-deleted as a clap v4 derive subcommand with a single positional <workUnitId> argument and NO flags — matching the flag-less TS Commander.js registration at src/commands/show-deleted.ts:73-76 (no --status, no --workspace, no --format)
  #   11. The clap subcommand action MUST delegate to the same fspec_core::commands::show_deleted::run function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate filter or rendering logic in the CLI bridge
  #   12. The dispatcher MUST accept an optional format arg ('text' default | 'json') so the LLM tool-call path can request a structured JSON payload (mirrors the list-prefixes pattern); the CLI surface does NOT expose --format because TS Commander.js does not
  #   13. The JSON format MUST emit a 2-space-indented object with fields success (bool), workUnitId (string), deletedItems (array of {id,text,deletedAt?}), and totalDeleted (number) — preserving declaration order via #[derive(Serialize)] with explicit field order
  #   14. The CLI wrapper MUST resolve the project root from current working directory, exit 0 on success, exit 1 on FspecCoreError, and write structured errors to stderr prefixed with 'Error:' (same chalk-equivalent contract as RPC-253 rule [14])
  #   15. show-deleted --help MUST be byte-for-byte identical to the TS formatCommandHelp output captured in codelet/fspec/tests/fixtures/help/show-deleted.txt (parity with RPC-248 rule [11])
  #
  # EXAMPLES:
  #   1. Dispatch show-deleted with workUnitId='AUTH-001' against a tempdir whose spec/work-units.json contains AUTH-001 with one deleted rule and one live example → returns success=true with deletedItems=[{id, text, deletedAt}] of length 1 and totalDeleted=1
  #   2. Dispatch show-deleted against a tempdir with NO spec/ subdirectory → the helper auto-creates spec/work-units.json with empty workUnits, then the command fails with success=false and error containing "Work unit 'AUTH-001' does not exist"
  #   3. Dispatch show-deleted for a work unit whose rules/examples/questions/architectureNotes arrays are absent → returns success=true with deletedItems=[] and totalDeleted=0; text format renders exactly 'No deleted items found'
  #   4. Dispatch show-deleted for a work unit with 2 deleted rules, 1 deleted example, 1 deleted question, 1 deleted architecture note interleaved with live items → result preserves order rules→examples→questions→architectureNotes (totalDeleted=5)
  #   5. Dispatch show-deleted for a work unit with a deleted rule that has NO deletedAt field → result entry contains id and text but the JSON omits deletedAt; text rendering omits the ' (deleted: ...)' suffix
  #   6. Dispatch show-deleted with workUnitId='AUTH-001' against text format → DispatchResult.data contains '\nDeleted items in AUTH-001 (3 total):\n  [0] First rule (deleted: 2025-01-31T12:00:00.000Z)\n  [1] Second example (deleted: 2025-02-01T08:00:00.000Z)\n  [3] Question text (deleted: 2025-02-02T09:00:00.000Z)\n'
  #   7. Running './codelet/target/release/fspec show-deleted AUTH-001' against a directory whose spec/work-units.json contains a deleted rule prints the header and item lines to stdout and exits 0
  #   8. Running './codelet/target/release/fspec show-deleted UNKNOWN-999' against an empty workspace prints 'Error: Work unit \'UNKNOWN-999\' does not exist' to stderr and exits with code 1
  #   9. Running './codelet/target/release/fspec show-deleted --help' prints clap-generated help with the <workUnitId> positional argument and NO --status / --workspace / --format flags listed
  #   10. Running './codelet/target/release/fspec show-deleted --help' produces stdout byte-for-byte identical to codelet/fspec/tests/fixtures/help/show-deleted.txt
  #   11. Both invocation paths produce the SAME structured data: (a) dispatch_command('show-deleted', {workUnitId:'AUTH-001', format:'json'}, project_root) and (b) './codelet/target/release/fspec show-deleted AUTH-001' against the same on-disk state — the only differences are how args are parsed (JSON vs clap positional) and how the result is delivered (DispatchResult.data vs stdout text)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-deleted from the agent loop and invoke it from a shell to enumerate every soft-deleted rule, example, question, and architecture note for a work unit
    So that I can audit pending deletions and pick stable IDs to restore — sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Returns deleted items in canonical concatenation order with only id text and deletedAt fields
    Given spec/work-units.json contains AUTH-001 with one deleted rule 'first rule', one live rule, one live example, one deleted example 'first ex', one deleted question 'first q', and one deleted architecture note 'first note'
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true with totalDeleted=4
    Then the deletedItems text fields read 'first rule', 'first ex', 'first q', 'first note' in that exact order
    Then each deletedItems entry contains only id, text, and deletedAt fields and drops createdAt, selected, answered, and answer

  Scenario: Auto-creates work-units.json and fails when the requested work unit does not exist
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch show-deleted with workUnitId='AUTH-001'
    Then the dispatcher returns success=false with an error message containing the substring "Work unit 'AUTH-001' does not exist"
    Then spec/work-units.json exists after the call with an empty workUnits object

  Scenario: Returns empty deletedItems for a work unit that has never had soft-deletes
    Given spec/work-units.json contains a work unit AUTH-001 with NO rules, examples, questions, or architectureNotes arrays
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true with totalDeleted=0
    Then the deletedItems array is empty

  Scenario: Excludes items whose deleted flag is false or missing
    Given spec/work-units.json contains AUTH-001 with one rule whose deleted=false and one rule with no deleted field
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true with totalDeleted=0
    Then the deletedItems array is empty

  Scenario: Omits deletedAt from the JSON payload when the field is absent on the source item
    Given spec/work-units.json contains AUTH-001 with one deleted rule that has NO deletedAt field
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true with totalDeleted=1
    Then the first deletedItems entry contains id and text but the deletedAt field is omitted from the JSON

  Scenario: Text format renders the empty case as 'No deleted items found'
    Given spec/work-units.json contains AUTH-001 with no deleted items
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    Then the DispatchResult.data is exactly the string 'No deleted items found'

  Scenario: Text format renders the populated case with header item lines and timestamps
    Given spec/work-units.json contains AUTH-001 with one deleted rule (id=0, text='First rule', deletedAt='2025-01-31T12:00:00.000Z') and one deleted example (id=1, text='Second example', deletedAt='2025-02-01T08:00:00.000Z')
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    Then the DispatchResult.data contains the line 'Deleted items in AUTH-001 (2 total):'
    Then the DispatchResult.data contains the exact line '  [0] First rule (deleted: 2025-01-31T12:00:00.000Z)'
    Then the DispatchResult.data contains the exact line '  [1] Second example (deleted: 2025-02-01T08:00:00.000Z)'

  Scenario: Text format omits the deleted timestamp suffix when deletedAt is missing
    Given spec/work-units.json contains AUTH-001 with one deleted rule (id=7, text='No timestamp item') and NO deletedAt field on that rule
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    Then the DispatchResult.data contains the exact line '  [7] No timestamp item'
    Then the DispatchResult.data does NOT contain the substring 'deleted:'

  Scenario: Defaults to text format when the format argument is omitted
    Given spec/work-units.json contains AUTH-001 with no deleted items
    When I dispatch show-deleted with workUnitId='AUTH-001' and no format field supplied
    Then the DispatchResult.data is exactly the string 'No deleted items found'

  Scenario: Returns a structured error when workUnitId is missing from the args
    Given an empty project root directory
    When I dispatch show-deleted with an empty args object
    Then the dispatcher returns success=false with an error message describing the missing workUnitId argument

  Scenario: JSON format emits 2-space indented payload with the canonical field set
    Given spec/work-units.json contains AUTH-001 with one deleted rule (id=2, text='X', deletedAt='2025-06-01T00:00:00.000Z')
    When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    Then the DispatchResult.data parses as JSON whose root has success=true, workUnitId='AUTH-001', totalDeleted=1, and a deletedItems array of length 1
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Shared infrastructure delegation
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/commands/show_deleted.rs
    Then the file calls io::ensure::ensure_work_units_file rather than embedding its own work-units.json read logic
    Then the file does NOT contain the substring 'FspecCoreError::NotYetPorted'

