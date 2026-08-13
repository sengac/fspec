@done
@feature-management
@work-management
@prefix-epic
@prefixes
@rust
@cli
@RPC-213
Feature: Port create-prefix command to Rust
  """
  Core impl lives at rust/fspec-core/src/commands/create_prefix.rs and replaces the NotYetPorted stub. It reuses shared infrastructure: `io::ensure::ensure_prefixes_file` (auto-create), `io::locked_file::write_json_atomic` (atomic write), `types::work_unit::PrefixesData` (IndexMap<String, Prefix>), `types::prefix::Prefix`.
  CLI bridge at rust/fspec/src/create_prefix.rs marshals clap-derived `CliArgs { prefix: String, description: String }` to JSON `{ prefix, description }`, then re-enters `create_prefix::run(args_json, cwd)`. Bridge has NO validation or file IO — only `env::current_dir()` + JSON serialisation + stdout/stderr formatting.
  Help config at rust/fspec-core/src/help/configs/create_prefix.rs ports `src/commands/create-prefix-help.ts` 1:1 — same description, when_to_use, two required arguments, single example with output `✓ Created prefix AUTH\n  Description: Authentication features`, related commands list, notes.
  Dispatcher returns a Serialize struct `CreatePrefixResult { success: bool, prefix: String, description: String, created_at: String }` (field order via #[derive(Serialize)] preserves declaration order). The CLI bridge ignores the JSON body content (success is checked via Ok variant) and prints the canonical `✓ Prefix <X> created successfully` message itself.
  Time source: ISO-8601 UTC string via the same `iso8601_now()` epoch_to_ymdhms helper already in io/ensure.rs — kept private to ensure.rs today. To avoid touching shared ensure.rs, the create_prefix command computes its own `now_iso()` inline (same algorithm: seconds_since_epoch → YYYY-MM-DDThh:mm:ss.000Z). Algorithm duplication is acceptable for now; can refactor into a shared time helper later.
  Validation regex `^[A-Z]{2,6}$` uses the `regex` crate (already a transitive dep) OR a manual `chars().all(|c| c.is_ascii_uppercase()) && (2..=6).contains(&len)` check. We use the manual check to avoid pulling regex into fspec-core if not already present, matching TS semantic exactly: only ASCII A-Z, length 2..=6.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Prefix MUST match /^[A-Z]{2,6}$/ — lowercase, digits, length<2 or >6, or special characters are rejected before any file IO with the message 'Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH)' (parity with src/commands/create-prefix.ts:19,29-31)
  #   2. All thrown errors are wrapped by the TS catch arm and prefixed with 'Failed to create prefix:' (src/commands/create-prefix.ts:58-63) so validation surfaces as 'Failed to create prefix: Prefix must be ...' and duplicate surfaces as 'Failed to create prefix: Prefix AUTH already exists' — Rust MUST mirror these exact substrings
  #   3. Reads (and auto-creates if missing) spec/prefixes.json via the shared `ensure_prefixes_file` helper — this is DIFFERENT from list-prefixes which uses `read_prefixes_or_empty` (no auto-create). Mutation commands MUST auto-create to match TS `ensurePrefixesFile` at src/commands/create-prefix.ts:35
  #   4. If `data.prefixes[options.prefix]` already exists, command MUST fail with message containing 'Prefix <X> already exists' BEFORE any write occurs (src/commands/create-prefix.ts:39-41) — the existing file remains untouched
  #   5. Successful creation atomically writes the updated PrefixesData (with the new entry inserted via IndexMap::insert preserving registration order) using `io::locked_file::write_json_atomic` — TS uses `fileManager.transaction()` which is atomic write-then-rename
  #   6. New Prefix entry shape is `{ prefix: <name>, description: <desc>, createdAt: <ISO-8601 UTC now> }` mirroring `new Date().toISOString()` at src/commands/create-prefix.ts:47 — `epicId` is NOT set by create-prefix (only update-prefix can attach it)
  #   7. Insertion order of prefixes.json MUST be preserved — appending AUTH then UI yields `{ "prefixes": { "AUTH": {...}, "UI": {...} } }` so subsequent list-prefixes outputs them in that order (`IndexMap<String, Prefix>` in PrefixesData)
  #   8. If spec/prefixes.json exists but is malformed JSON, the command MUST propagate a structured 'Failed to parse prefixes.json' error (escalated through `ensure_prefixes_file` → `read_or_init_json`), NOT silently overwrite the file
  #   9. The CLI bridge at rust/fspec/src/create_prefix.rs marshals two required positional args (<prefix> and <description>) to JSON `{ "prefix": "...", "description": "..." }` and delegates to `fspec_core::commands::create_prefix::run` — NO validation or filesystem logic in the bridge (RPC-003 §7/§11 two-front-doors)
  #   10. On success the CLI bridge prints `✓ Prefix <X> created successfully` to stdout and exits 0; on any error it prints `Error: <message>` to stderr and exits 1 (parity with `output.error('✗ Failed to create prefix:', err.message)` + `process.exit(1)`)
  #   11. The dispatcher path returns the canonical JSON shape `{ success: true, prefix: <X>, description: <desc>, createdAt: <iso> }` so structured callers (the LLM agent loop) can read the created entry. Field order: success, prefix, description, createdAt (use #[derive(Serialize)] to preserve)
  #   12. clap subcommand exposes NO flags — only the two positional args `<prefix> <description>` (parity with TS Commander.js at src/commands/create-prefix.ts:66-86 which has no `.option(...)` calls) — and --workspace MUST NOT appear in `fspec create-prefix --help`
  #
  # EXAMPLES:
  #   1..6, 11 — dispatcher contract (this file)
  #   7..10 — CLI surface (see create-prefix-cli-subcommand.feature)
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to register a new work-unit prefix from either the LLM dispatcher or the shell CLI
    So that I can namespace work units by component without depending on the TypeScript implementation

  Scenario: Successful registration creates spec/prefixes.json with the new entry
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    Then the dispatcher returns success=true
    Then spec/prefixes.json exists and contains the prefix entry 'AUTH' with description 'Auth features'
    Then the returned JSON includes a non-empty createdAt timestamp matching the ISO-8601 UTC format
    Then the returned JSON fields appear in the order success, prefix, description, createdAt

  Scenario: Lowercase prefix is rejected before any file IO occurs
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='auth' and description='bad case'
    Then the dispatcher returns success=false with an error message that does NOT include the outer-catch wrap
    Then the error message contains the substring 'Prefix must be 2-6 uppercase letters'
    Then spec/prefixes.json does not exist after the call

  Scenario: Prefix shorter than two characters is rejected
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='A' and description='too short'
    Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    Then spec/prefixes.json does not exist after the call

  Scenario: Prefix longer than six characters is rejected
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='ABCDEFG' and description='too long'
    Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    Then spec/prefixes.json does not exist after the call

  Scenario: Prefix containing digits is rejected
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='AB1' and description='has digit'
    Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    Then spec/prefixes.json does not exist after the call

  Scenario: Duplicate prefix is rejected and the existing file is left untouched
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    When I dispatch create-prefix with args prefix='AUTH' and description='Different desc'
    Then the dispatcher returns success=false with an error message containing the substring 'Prefix AUTH already exists'
    Then spec/prefixes.json is byte-identical to its pre-call content

  Scenario: Appending a second prefix preserves insertion order
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    When I dispatch create-prefix with args prefix='UI' and description='User interface'
    Then the dispatcher returns success=true
    Then spec/prefixes.json contains both AUTH and UI as keys
    Then in the on-disk JSON the AUTH entry appears before the UI entry

  Scenario: Malformed prefixes.json escalates a structured parse error
    Given spec/prefixes.json exists but contains the malformed bytes '{ not valid json'
    When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json'
    Then spec/prefixes.json is byte-identical to its pre-call content

  Scenario: Successful dispatcher path returns the canonical JSON shape
    Given an empty project root with no spec/ subdirectory
    When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON whose root object has fields success=true, prefix='AUTH', description='Auth features', and a createdAt string
    Then the createdAt field value matches the regex '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}.[0-9]{3}Z$'

  Scenario: Shared infrastructure is reused without duplication
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/create_prefix.rs
    Then the source declares it uses `ensure_prefixes_file` and `write_json_atomic` from the shared io modules
    Then the source does NOT contain the substring 'FspecCoreError::NotYetPorted'
    Then the source does NOT inline any std::fs::write or serde_json::to_writer call for spec/prefixes.json
