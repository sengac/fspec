@done
@rust
@prefixes
@prefix-epic
@work-management
@feature-management
@cli
@RPC-313
Feature: Port update-prefix command to Rust
  """
  Two-front-doors invariant (RPC-003 §7/§11): dispatcher and clap CLI both call commands::update_prefix::run(args_json, project_root). CLI bridge marshals positional <prefix> + --description into JSON object {prefix, description}; epicId is dispatcher-only.
  Prefix struct (rust/fspec-core/src/types/prefix.rs) does NOT have native epicId or updatedAt fields. RPC-313 mutates these via the existing #[serde(flatten)] extra: serde_json::Map<String, Value> catch-all. Existing on-disk epicId/updatedAt round-trip through this map; new values are inserted by string key. This keeps RPC-313 fully within worker-owned files and avoids a shared-type change.
  Epic verification uses io::ensure::read_epics_or_empty (RPC-243 helper) rather than a (non-existent) ensure_epics_file. Observable difference vs TS: when epicId is provided but spec/epics.json is missing, TS auto-creates an empty epics.json before failing with 'Epic <Y> not found'; Rust returns 'Epic <Y> not found' without touching disk. User-visible error message is identical. If strict side-effect parity is required, supervisor can add ensure_epics_file later (read-only RPC).
  Atomic write via io::locked_file::write_json_atomic (write-temp + rename). spec/prefixes.json is rebuilt from the in-memory PrefixesData and re-serialised in full — this matches TS fileManager.transaction semantics and ensures partial writes never land.
  Insertion order: PrefixesData.prefixes is IndexMap<String, Prefix>. We update the existing entry in-place using IndexMap::get_mut(&prefix); we do NOT call insert() (which would not move existing keys but is misleading). On-disk JSON preserves the original position of the updated record.
  Result shape: #[derive(Serialize)] struct UpdatePrefixResult { success: bool } — single field. Dispatcher returns serde_json::to_string_pretty(&result). CLI bridge ignores the payload and prints `✓ Prefix <X> updated successfully` for parity with TS `output.log(...)`.
  ISO-8601 timestamp generation duplicated from create_prefix.rs (Howard Hinnant epoch_to_ymdhms). Acceptable for RPC-313 because the helper is small and there's no public io::time::now() yet; the duplication is tracked as a future refactor opportunity (same as RPC-213 noted).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dispatcher accepts an args JSON object with `prefix` (required), and optional `description` and `epicId` — matching TS updatePrefix({prefix, description?, epicId?, cwd?})
  #   2. Missing `prefix` argument errors with `Failed to update prefix: Prefix  not found` (empty-string fallback through the same not-found branch as TS truthiness check)
  #   3. Reading prefixes uses `ensure_prefixes_file` (auto-create empty spec/prefixes.json on ENOENT) — matching TS `ensurePrefixesFile`
  #   4. If the named prefix does not exist in `data.prefixes`, the command errors with `Failed to update prefix: Prefix <X> not found` and the file stays byte-identical (no write)
  #   5. If `epicId` is provided, the command reads epics via `read_epics_or_empty` and verifies the epic exists; missing epic errors with `Failed to update prefix: Epic <Y> not found`
  #   6. If `description` is provided, it OVERWRITES the existing description; if omitted, the existing description is preserved verbatim
  #   7. If `epicId` is provided AND valid, it OVERWRITES the existing epicId on the Prefix; if omitted, the existing epicId is preserved verbatim
  #   8. The Prefix entry's `updatedAt` field is ALWAYS set to the current ISO-8601 timestamp on every successful run, even when no other fields changed
  #   9. Updates preserve insertion order in `prefixes.json` — the modified Prefix stays at its original position, NOT moved to the end
  #   10. Writes are atomic via `write_json_atomic` (write-temp + rename) — matching TS `fileManager.transaction`
  #   11. On success the dispatcher returns pretty-printed JSON `{"success": true}`; on failure it returns an `FspecCoreError::InvalidArgs` whose message starts with `Failed to update prefix:`
  #   12. CLI surface is `fspec update-prefix <prefix> [-d|--description <description>]` — `epicId` is NOT exposed on the CLI (dispatcher-only field)
  #   13. CLI prints `✓ Prefix <X> updated successfully` to stdout on success (exit 0) and `Error: Failed to update prefix: <msg>` to stderr on failure (exit 1)
  #
  # EXAMPLES:
  #   1. Dispatcher updates description: AUTH exists with description 'old'. JSON args `{"prefix":"AUTH","description":"new"}` → returns `{"success":true}`, file now has description 'new' and a fresh `updatedAt`
  #   2. Dispatcher updates epicId: AUTH exists, epics.json has epic 'auth-epic'. JSON args `{"prefix":"AUTH","epicId":"auth-epic"}` → success, AUTH now references that epic
  #   3. Dispatcher rejects unknown prefix: prefixes.json is empty. JSON args `{"prefix":"NONE"}` → error `Failed to update prefix: Prefix NONE not found`, file unchanged
  #   4. Dispatcher rejects unknown epicId: AUTH exists, epics.json missing or empty. JSON args `{"prefix":"AUTH","epicId":"ghost"}` → error `Failed to update prefix: Epic ghost not found`, prefixes.json unchanged
  #   5. Dispatcher updates BOTH description and epicId in one call: JSON args `{"prefix":"AUTH","description":"new","epicId":"auth-epic"}` → success, both fields replaced, updatedAt bumped
  #   6. Dispatcher no-op call (no description/epicId): JSON args `{"prefix":"AUTH"}` → success, only `updatedAt` is bumped, description and epicId preserved verbatim
  #   7. Dispatcher preserves insertion order: prefixes.json has AUTH then UI then API; update AUTH; on-disk order remains AUTH, UI, API (AUTH not moved to end)
  #   8. Dispatcher escalates malformed prefixes.json: file contains `{ not json`. Dispatcher returns error containing `Failed to parse prefixes.json` (consistent with create-prefix RPC-213 rule [8])
  #   9. CLI updates description: AUTH exists. `fspec update-prefix AUTH -d "new"` → stdout `✓ Prefix AUTH updated successfully`, exit 0
  #   10. CLI no-op (no --description): AUTH exists. `fspec update-prefix AUTH` → stdout `✓ Prefix AUTH updated successfully`, only updatedAt bumped
  #   11. CLI rejects unknown prefix: prefixes.json empty. `fspec update-prefix MISSING -d x` → stderr contains `Failed to update prefix: Prefix MISSING not found`, exit 1
  #   12. CLI does NOT expose --epic-id: `fspec update-prefix AUTH --epic-id foo` → clap error 'unexpected argument', exit 2 (parity with the TS Commander surface that omits this option)
  #   13. CLI surfaces missing positional: `fspec update-prefix` (no prefix arg) → clap usage error, exit 2
  #   14. CLI help surface: `fspec update-prefix --help` prints byte-for-byte the same usage block the TS `node dist/index.js update-prefix --help` emits (captured as `tests/fixtures/help/update-prefix.txt`)
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the TypeScript CLI to Rust
    I want to port `update-prefix` to the fspec-core crate behind a clap subcommand and the existing LLM dispatcher
    So that the standalone Rust binary can mutate `spec/prefixes.json` with full TS parity (success messages, error wrapping, atomic writes, insertion-order preservation) without reading from `node dist/index.js`

  Scenario: CLI updates the description of an existing prefix
    Given spec/prefixes.json contains AUTH with description 'old'
    When I run `fspec update-prefix AUTH -d "new"` in that project root
    Then the process exits with code 0
    Then stdout contains the line '✓ Prefix AUTH updated successfully'
    Then spec/prefixes.json now has AUTH.description equal to 'new'
    Then spec/prefixes.json now has AUTH.updatedAt set to a non-empty ISO-8601 UTC timestamp

  Scenario: CLI no-op call bumps only updatedAt
    Given spec/prefixes.json contains AUTH with description 'kept'
    When I run `fspec update-prefix AUTH` in that project root
    Then the process exits with code 0
    Then stdout contains the line '✓ Prefix AUTH updated successfully'
    Then spec/prefixes.json AUTH.description is preserved verbatim as 'kept'
    Then spec/prefixes.json AUTH.updatedAt is set to a non-empty ISO-8601 UTC timestamp

  Scenario: CLI rejects an unknown prefix with a wrapped error and exit 1
    Given spec/prefixes.json is empty (no prefixes registered)
    When I run `fspec update-prefix MISSING -d "ignored"` in that project root
    Then the process exits with code 1
    Then stderr contains the substring 'Failed to update prefix'
    Then stderr contains the substring 'Prefix MISSING not found'
    Then spec/prefixes.json is byte-identical to its pre-call content

  Scenario: CLI surface does NOT expose --epic-id
    Given spec/prefixes.json contains AUTH with description 'Auth features'
    When I run `fspec update-prefix AUTH --epic-id auth-epic` in that project root
    Then the process exits with code 2
    Then stderr contains the substring 'unexpected argument'
    Then spec/prefixes.json is byte-identical to its pre-call content

  Scenario: CLI requires the prefix positional argument
    Given any project root
    When I run `fspec update-prefix` with no arguments in that project root
    Then the process exits with code 2
    Then stderr contains the substring 'required'

  Scenario: CLI help surface matches the captured TS fixture
    Given the TS help fixture at rust/fspec/tests/fixtures/help/update-prefix.txt
    When I run `fspec update-prefix --help`
    Then the process exits with code 0
    Then stdout matches the captured TS fixture byte-for-byte (modulo trailing newline)

  Scenario: Clap exposes update-prefix as a top-level subcommand
    Given the rust/fspec crate is built
    When I run `fspec --help`
    Then the process exits with code 0
    Then stdout contains the substring 'update-prefix'

  Scenario: CLI delegates to the same fspec_core function the dispatcher uses
    Given the rust/fspec crate is built
    When I inspect rust/fspec/src/update_prefix.rs
    Then the source declares it calls `codelet_fspec_core::commands::update_prefix::run`
    Then the source does NOT perform any file IO directly on spec/prefixes.json

  Scenario: Default combined TUI mode is preserved when no subcommand is given
    Given the rust/fspec crate is built
    When I run `fspec` with no arguments
    Then the binary does NOT route to update_prefix::run
    Then the binary attempts to launch the combined TUI mode (the existing default arm)
