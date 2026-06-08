@done
@querying
@cli
@rust
@RPC-242
Feature: Port list-checkpoints command to Rust

  """
  File layout (worker-owned): codelet/fspec-core/src/commands/list_checkpoints.rs (rewrite stub), codelet/fspec-core/src/help/configs/list_checkpoints.rs (new), codelet/fspec-core/tests/list_checkpoints.rs (new dispatcher test), codelet/fspec/src/list_checkpoints.rs (new CLI bridge), codelet/fspec/tests/cli_list_checkpoints.rs (new CLI test), codelet/fspec/tests/fixtures/help/list-checkpoints.txt (new help fixture)
  Shared-file changes required from supervisor: (1) codelet/fspec-core/Cargo.toml — add `codelet-git.workspace = true` to [dependencies]; (2) codelet/fspec-core/src/help/configs/mod.rs — add `pub mod list_checkpoints;`; (3) codelet/fspec-core/src/dispatch.rs — register list-checkpoints route delegating to commands::list_checkpoints::run; (4) codelet/fspec/src/main.rs — add Mode::ListCheckpoints { work_unit_id: String } clap variant plus action arm plus help-printing branch. canonical.rs already lists list-checkpoints so no change there.
  Reuse `codelet_git::ghost_commit::list_ghost_checkpoints(dir, workUnitId) -> Vec<String>` and `codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN` constant. NO new gix/git2 code — list-checkpoints is a thin assembly over the existing git crate.
  Index-file reading: keep `read_checkpoint_index_or_empty(cwd, work_unit_id) -> IndexMap<String,String>` as a PRIVATE helper inside commands/list_checkpoints.rs (NOT in io/ensure.rs) to minimise shared-file churn — only list-checkpoints reads .git/fspec-checkpoints-index/{id}.json today. If a second consumer (cleanup-checkpoints / TUI / etc.) ports, refactor into io/ensure.rs at that point.
  Error handling: if open_repo fails (not a git repo) return Ok(empty list) to match `count_checkpoints` semantics and avoid leaking gix errors to the LLM. Any genuine FspecCoreError comes from JSON arg parsing (missing workUnitId).
  Sort stability: when two checkpoints share an identical timestamp (e.g. both default to current time), Vec::sort_by maintains insertion order from list_ghost_checkpoints (which iterates gix refs — order undefined). Tests therefore assert on UNIQUE timestamps only; ties are not part of the contract.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-checkpoints` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by every other ported command
  #   2. The command takes a single REQUIRED positional argument workUnitId (camelCase JSON key); missing argument MUST return success=false with InvalidArgs (parity with TS Commander `.argument('<work-unit-id>')` which exits 1 on missing arg)
  #   3. Checkpoints are read by listing git refs under `refs/fspec-checkpoints/{workUnitId}/` via `codelet_git::ghost_commit::list_ghost_checkpoints(cwd, workUnitId)` — the SAME pure-gix function NAPI exposes — so the Rust port shares one source of truth with the TS NAPI binding
  #   4. If the project root is NOT a git repository the command MUST silently return success with an empty checkpoints list (parity with `codelet_git::ghost_commit::count_checkpoints` which already swallows the `open_repo` failure — keeps the test fixture surface manageable)
  #   5. Timestamps are read from the JSON index file at `.git/fspec-checkpoints-index/{workUnitId}.json` whose shape is `{ checkpoints: [{ name, sha, timestamp }] }` (parity with `src/utils/git-checkpoint.ts:114-128`)
  #   6. If a checkpoint exists in git refs but is MISSING from the index file (or the index file does not exist OR contains malformed JSON), the timestamp defaults to the current ISO-8601 timestamp (parity with TS `indexEntry?.timestamp || new Date().toISOString()`)
  #   7. Each checkpoint is classified as automatic if and only if its name contains the substring `-auto-` (the `AUTO_CHECKPOINT_PATTERN` constant exposed by `codelet_git::ghost_commit`); all other names are manual
  #   8. Output is sorted by timestamp DESCENDING (newest first) — parity with TS `checkpoints.sort((a,b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())`
  #   9. Text format prints exactly `No checkpoints found for {workUnitId}` (no leading newline) when the checkpoint list is empty; populated lists emit `\nCheckpoints for {workUnitId}:\n\n{icon}  {name} ({automatic|manual})\n   Created: {timestamp}\n\n` for each checkpoint where icon is `🤖` for automatic and `📌` for manual
  #   10. JSON format returns `{ workUnitId, checkpoints: [{ name, timestamp, displayIcon, isAutomatic }] }` with 2-space indentation (parity with TS `JSON.stringify(result, null, 2)`); `--format` is NOT exposed at the TS CLI surface but the Rust shared `run()` accepts it for the dispatcher's structured-output path (same convention as `list-prefixes`)
  #   11. The standalone fspec binary at codelet/fspec/src/main.rs MUST expose `list-checkpoints <work-unit-id>` as a clap v4 derive subcommand with exactly one required positional argument and NO flags — mirroring the TS Commander.js registration at src/commands/list-checkpoints.ts:83-88
  #   12. The clap subcommand action MUST delegate to the same `fspec_core::commands::list_checkpoints::run()` function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate checkpoint-listing, classification or rendering logic in the CLI bridge
  #   13. The CLI wrapper MUST resolve the project root from current working directory (parity with TS `process.cwd()` default), exit 0 on success, exit 1 on FspecCoreError, and write structured errors to stderr prefixed with `Error:` (same contract as RPC-253 rule [14] and RPC-248 rule [13])
  #
  # EXAMPLES:
  #   1. Dispatch list-checkpoints with workUnitId='AUTH-001' against an empty tempdir (NO .git directory) → returns success=true with empty checkpoints array, text data is exactly 'No checkpoints found for AUTH-001'
  #   2. Dispatch list-checkpoints with empty workUnitId='' → success=false with InvalidArgs error mentioning 'workUnitId' (TS Commander requires a non-empty positional)
  #   3. Dispatch list-checkpoints with no workUnitId field in args JSON → success=false with InvalidArgs error mentioning 'workUnitId'
  #   4. Dispatch list-checkpoints in a git repo with one manual checkpoint 'baseline' for AUTH-001 (index timestamp 2026-06-01T10:00:00.000Z) → returns success=true and text data contains 'Checkpoints for AUTH-001:', the line '📌  baseline (manual)', and 'Created: 2026-06-01T10:00:00.000Z'
  #   5. Dispatch list-checkpoints in a git repo with one automatic checkpoint 'AUTH-001-auto-testing' for AUTH-001 → text data contains the line '🤖  AUTH-001-auto-testing (automatic)' (icon and label reflect automatic classification via the -auto- substring)
  #   6. Dispatch list-checkpoints with format='json' against a git repo containing manual 'baseline' (timestamp 2026-06-01T10:00:00.000Z) and automatic 'AUTH-001-auto-testing' (timestamp 2026-06-02T12:00:00.000Z) for AUTH-001 → JSON checkpoints array has length 2 with AUTH-001-auto-testing first (newest timestamp) and isAutomatic=true, displayIcon='🤖' for that entry
  #   7. Dispatch list-checkpoints against a git repo containing 'baseline' for AUTH-001 but with NO .git/fspec-checkpoints-index/AUTH-001.json file → returns success=true and the baseline entry's timestamp is a non-empty ISO-8601 string (current time fallback)
  #   8. Dispatch list-checkpoints against a git repo containing 'baseline' for AUTH-001 but with malformed JSON in the index file ('{ not json') → command succeeds (does NOT escalate the parse error) and the baseline entry's timestamp is a non-empty ISO-8601 string (fallback path)
  #   9. Running `./codelet/target/release/fspec list-checkpoints --help` prints clap-generated help with NO --format / --workspace / --prefix flags, lists the positional <workUnitId> argument, and exits 0
  #   10. Running `./codelet/target/release/fspec list-checkpoints` with NO positional argument exits with clap's standard required-arg error (code 2) and stderr contains 'workUnitId' or 'work-unit-id'
  #   11. Running `./codelet/target/release/fspec list-checkpoints AUTH-001` in an empty directory prints 'No checkpoints found for AUTH-001' to stdout and exits 0 (does NOT create .git or auto-init a repo)
  #   12. Running `./codelet/target/release/fspec list-checkpoints AUTH-001` against a git repo with one manual checkpoint 'baseline' (index timestamp present) prints the header 'Checkpoints for AUTH-001:', the line '📌  baseline (manual)', and 'Created: ...' to stdout and exits 0
  #   13. Running `./codelet/target/release/fspec --help` lists `list-checkpoints` as an available subcommand alongside daemon, client, status, list-work-units, list-prefixes (default combined TUI mode preserved)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-checkpoints from the agent loop (and run it from the shell) to view all checkpoints for a work unit with the same visual indicators as the TypeScript implementation
    So that I can browse manual and automatic checkpoints from a Rust binary without Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Returns empty checkpoints list against an empty tempdir with no git repository
    Given an empty project root directory with no .git subdirectory
    When I dispatch the list-checkpoints command against that project root with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true with an empty checkpoints array
    Then the JSON data has a workUnitId field equal to 'AUTH-001'

  Scenario: Returns text sentinel 'No checkpoints found' for empty results
    Given an empty project root directory with no .git subdirectory
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data is exactly the string 'No checkpoints found for AUTH-001'

  Scenario: Missing workUnitId field in args JSON fails with InvalidArgs
    Given an empty project root directory
    When I dispatch the list-checkpoints command with the args JSON '{}'
    Then the dispatcher returns success=false
    Then the error message contains the substring 'workUnitId'

  Scenario: Empty workUnitId string fails with InvalidArgs
    Given an empty project root directory
    When I dispatch the list-checkpoints command with workUnitId=''
    Then the dispatcher returns success=false
    Then the error message contains the substring 'workUnitId'

  Scenario: Renders a single manual checkpoint with the manual icon and label
    Given a git repository at the project root with a manual checkpoint named 'baseline' for AUTH-001
    Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the substring 'Checkpoints for AUTH-001:'
    Then the DispatchResult.data contains the substring '📌  baseline (manual)'
    Then the DispatchResult.data contains the substring 'Created: 2026-06-01T10:00:00.000Z'

  Scenario: Renders a single automatic checkpoint with the automatic icon and label
    Given a git repository at the project root with an automatic checkpoint named 'AUTH-001-auto-testing' for AUTH-001
    Given the checkpoint index file records timestamp '2026-06-02T12:00:00.000Z' for 'AUTH-001-auto-testing'
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the substring '🤖  AUTH-001-auto-testing (automatic)'

  Scenario: JSON format sorts checkpoints by timestamp descending
    Given a git repository at the project root with checkpoints 'baseline' and 'AUTH-001-auto-testing' for AUTH-001
    Given the checkpoint index file records 'baseline' at '2026-06-01T10:00:00.000Z' and 'AUTH-001-auto-testing' at '2026-06-02T12:00:00.000Z'
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON checkpoints array has length 2
    Then the first entry has name='AUTH-001-auto-testing', isAutomatic=true, displayIcon='🤖'
    Then the second entry has name='baseline', isAutomatic=false, displayIcon='📌'

  Scenario: JSON format emits two-space indented payload with the canonical field set
    Given a git repository at the project root with one manual checkpoint 'baseline' for AUTH-001
    Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON whose root object has workUnitId='AUTH-001' and a 'checkpoints' array of length 1
    Then the first checkpoints entry contains fields name='baseline', timestamp='2026-06-01T10:00:00.000Z', displayIcon='📌', isAutomatic=false
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Missing index file falls back to a non-empty ISO-8601 timestamp
    Given a git repository at the project root with a manual checkpoint 'baseline' for AUTH-001
    Given the file .git/fspec-checkpoints-index/AUTH-001.json does NOT exist
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON checkpoints array has length 1
    Then the baseline entry's timestamp is a non-empty string of length >= 20

  Scenario: Malformed index file is silently swallowed and falls back to a non-empty timestamp
    Given a git repository at the project root with a manual checkpoint 'baseline' for AUTH-001
    Given the file .git/fspec-checkpoints-index/AUTH-001.json contains the malformed bytes '{ not json'
    When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the JSON checkpoints array has length 1
    Then the baseline entry's timestamp is a non-empty string of length >= 20

  Scenario: Shared infrastructure modules exist under codelet/fspec-core and codelet-git is wired as a dependency
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/Cargo.toml
    Then the dependencies section declares codelet-git via the workspace
    When I inspect codelet/fspec-core/src/commands/list_checkpoints.rs
    Then it references codelet_git::ghost_commit::list_ghost_checkpoints
    Then it references codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN
    Then it does NOT contain the substring 'FspecCoreError::NotYetPorted'
