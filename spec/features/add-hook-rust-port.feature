@done
@RPC-184
@rust
@cli
Feature: Port add-hook command to Rust

  """
  Replace the stub at codelet/fspec-core/src/commands/add_hook.rs with `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`. Module exposes the same signature shape as list_hooks::run.
  Local on-disk shapes: `struct HookFile { hooks: IndexMap<String, Vec<HookEntry>>, #[serde(flatten)] extra: Map<String,Value> }` and `struct HookEntry { name: String, command: String, blocking: bool, #[serde(skip_serializing_if=Option::is_none)] timeout: Option<u64>, #[serde(flatten)] extra: Map<String,Value> }`. Local, NOT promoted to types/hooks.rs.
  Load strategy: `std::fs::read_to_string` + `serde_json::from_str` wrapped in a single helper that returns `HookFile::default()` on EITHER IO error OR parse error (TS bare-catch parity at add-hook.ts:26-32). Do NOT use ensure.rs helpers — they auto-write the default to disk, which would race the subsequent atomic write.
  Args struct: `#[serde(default, rename_all = "camelCase")] struct AddHookArgs { event: String, name: String, command: String, #[serde(default)] blocking: bool, timeout: Option<u64> }`. The CLI bridge marshals clap fields → JSON, omitting `None` for timeout.
  Persistence: ensure `spec/` exists via `std::fs::create_dir_all(project_root.join("spec"))` then call `write_json_atomic(&path, &config)` (2-space indent, no trailing newline). This mirrors the TS `mkdir(... recursive:true)` + `fileManager.transaction` round-trip.
  Result shape: `pub async fn run` returns `Ok(String::new())` on success (zero bytes — TS Commander action prints nothing). The dispatcher wraps this as `success=true, data=""`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `add-hook` MUST replace the NotYetPorted stub and be registered in PORTED_COMMANDS
  #   2. ENOENT on spec/fspec-hooks.json → silently create a fresh `{hooks:{<event>:[<new>]}}` and write it
  #   3. Invalid JSON in spec/fspec-hooks.json → swallow the parse error and OVERWRITE the file (TS bare catch parity)
  #   4. Successful read+parse → append the new hook entry to `hooks[event]` (initialising the array if the key is absent)
  #   5. Hook entry on-disk shape is `{name, command, blocking, timeout?}` — `timeout` is OMITTED when not supplied
  #   6. Unknown top-level fields (`global`, etc.) and per-entry fields MUST be preserved across the round-trip
  #   7. Event-key insertion order MUST be preserved
  #   8. Adding the same name twice to the same event is allowed
  #   9. Atomic on-disk write uses `write_json_atomic` (2-space indent, no trailing newline)
  #   10. Dispatcher returns success=true with empty `data` on success
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-hook from the agent loop AND invoke `fspec add-hook` from a shell
    So that I can register lifecycle hooks for a project without going through Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Creates spec/fspec-hooks.json when missing and writes a single entry
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-hook with event='pre-implementing', name='lint', command='spec/hooks/lint.sh', blocking=false
    Then the dispatcher returns success=true
    Then spec/fspec-hooks.json exists after the call
    Then the on-disk JSON parses to a top-level object whose 'hooks' key contains exactly the event 'pre-implementing'
    Then the 'pre-implementing' array has exactly one entry with name='lint', command='spec/hooks/lint.sh', blocking=false
    Then the entry on disk does NOT contain a 'timeout' field

  Scenario: Appends to an existing event array preserving previous entries
    Given spec/fspec-hooks.json contains event 'post-implementing' with a single entry named 'lint'
    When I dispatch add-hook with event='post-implementing', name='test', command='spec/hooks/test.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk 'post-implementing' array has exactly two entries
    Then the first entry has name='lint'
    Then the second entry has name='test' and command='spec/hooks/test.sh'

  Scenario: Adds a new event key when missing from existing config
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I dispatch add-hook with event='post-implementing', name='notify', command='spec/hooks/notify.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk 'hooks' object contains both 'pre-implementing' and 'post-implementing'
    Then the 'pre-implementing' event still has its 'lint' entry
    Then the 'post-implementing' event has exactly one entry named 'notify'

  Scenario: Omits timeout field on disk when not supplied
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false (no timeout)
    Then the dispatcher returns success=true
    Then the raw JSON bytes of spec/fspec-hooks.json do NOT contain the substring '"timeout"'

  Scenario: Writes blocking=true and timeout when supplied
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-hook with event='post-implementing', name='test', command='spec/hooks/test.sh', blocking=true, timeout=300
    Then the dispatcher returns success=true
    Then the on-disk entry has blocking=true and timeout=300

  Scenario: Swallows invalid JSON and overwrites with the new config
    Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk JSON parses successfully
    Then the on-disk 'hooks' object contains exactly the event 'pre-implementing' with one entry named 'lint'

  Scenario: Preserves unknown top-level global section
    Given spec/fspec-hooks.json contains a top-level 'global' object with timeout=30 and an empty 'hooks' object
    When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk JSON still contains a 'global' object with timeout=30
    Then the on-disk 'hooks' object contains exactly the event 'pre-implementing' with one entry named 'lint'

  Scenario: Allows duplicate hook names within the same event
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I dispatch add-hook with event='pre-implementing', name='lint', command='spec/hooks/other.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk 'pre-implementing' array has exactly two entries both named 'lint'

  Scenario: Preserves event-key insertion order across writes
    Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID each with one entry
    When I dispatch add-hook with event='MID', name='extra', command='extra.sh', blocking=false
    Then the dispatcher returns success=true
    Then the on-disk events appear in the order ZED, AAA, MID
