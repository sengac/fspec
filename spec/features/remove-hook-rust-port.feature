@done
@RPC-275
@rust
@cli
Feature: Port remove-hook command to Rust
  """
  Replace the stub at rust/fspec-core/src/commands/remove_hook.rs with `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  Local on-disk shapes (same as add_hook port): `struct HookFile { hooks: IndexMap<String, Vec<HookEntry>>, #[serde(flatten)] extra }` and `struct HookEntry { name, command, blocking, timeout?, #[serde(flatten)] extra }`. Local to the module — no shared types/hooks.rs.
  Load strategy DIVERGES from add_hook: `std::fs::read_to_string` errors propagate as `FspecCoreError::Io` and `serde_json::from_str` errors propagate as `FspecCoreError::ParseJson`. NO bare-catch wrap. Mirrors TS readFile + JSON.parse without try/catch.
  Args struct: `#[serde(default, rename_all = "camelCase")] struct RemoveHookArgs { event: String, name: String }`.
  Mutation: filter ALL entries whose name matches. Empty array is RETAINED — do NOT remove the event key.
  Persistence: `write_json_atomic(&path, &config)` (2-space indent, no trailing newline).
  Result shape: `pub async fn run` returns `Ok(String::new())` on success.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dispatcher route replaces the NotYetPorted stub
  #   2. ENOENT → propagate error (TS readFile rejection, NO bare catch)
  #   3. Invalid JSON → propagate ParseJson error (NO silent overwrite)
  #   4. Missing event key → silent no-op success
  #   5. Removing a non-existent name → silent no-op success
  #   6. Filtering removes ALL entries whose name matches
  #   7. Empty array after filter is retained (key NOT deleted)
  #   8. Unknown fields preserved across round-trip
  #   9. Event-key insertion order preserved
  #   10. Atomic write via `write_json_atomic`
  #   11. Empty `data` on success
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch remove-hook from the agent loop AND invoke `fspec remove-hook` from a shell
    So that I can deregister lifecycle hooks from a project without going through Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Removes a single named entry leaving siblings intact
    Given spec/fspec-hooks.json contains event 'post-implementing' with entries named 'lint' and 'test' in that order
    When I dispatch remove-hook with event='post-implementing', name='lint'
    Then the dispatcher returns success=true
    Then the on-disk 'post-implementing' array has exactly one entry named 'test'

  Scenario: Empty array after removal is retained (event key NOT deleted)
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I dispatch remove-hook with event='pre-implementing', name='lint'
    Then the dispatcher returns success=true
    Then the on-disk 'hooks' object still contains the key 'pre-implementing'
    Then the on-disk 'pre-implementing' array is exactly []

  Scenario: All duplicate entries with the same name are removed
    Given spec/fspec-hooks.json contains event 'pre-implementing' with three entries — two named 'lint' and one named 'other'
    When I dispatch remove-hook with event='pre-implementing', name='lint'
    Then the dispatcher returns success=true
    Then the on-disk 'pre-implementing' array has exactly one entry named 'other'

  Scenario: Missing event key is a silent no-op success
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I dispatch remove-hook with event='post-implementing', name='test'
    Then the dispatcher returns success=true
    Then the on-disk 'hooks' object contains only the key 'pre-implementing'
    Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')

  Scenario: Removing a name that does not exist is a silent no-op success
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I dispatch remove-hook with event='pre-implementing', name='nonexistent'
    Then the dispatcher returns success=true
    Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')

  Scenario: ENOENT on spec/fspec-hooks.json propagates an error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-hook with event='pre-implementing', name='lint'
    Then the dispatcher returns success=false
    Then the dispatcher error message indicates an IO failure

  Scenario: Invalid JSON propagates a ParseJson error (NO silent overwrite)
    Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    When I dispatch remove-hook with event='pre-implementing', name='lint'
    Then the dispatcher returns success=false
    Then the dispatcher error message indicates a parse failure for fspec-hooks.json
    Then the raw bytes of spec/fspec-hooks.json equal '{ not json' (file unchanged)

  Scenario: Preserves unknown top-level fields and adjacent entries
    Given spec/fspec-hooks.json contains a top-level 'global' object with timeout=30 and event 'pre-implementing' with entries named 'lint' and 'keep' where 'keep' has command='spec/hooks/keep.sh' and blocking=true and timeout=120
    When I dispatch remove-hook with event='pre-implementing', name='lint'
    Then the dispatcher returns success=true
    Then the on-disk JSON still contains a 'global' object with timeout=30
    Then the on-disk 'pre-implementing' array has exactly one entry named 'keep' with command='spec/hooks/keep.sh', blocking=true, and timeout=120

  Scenario: Preserves event-key insertion order across writes
    Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID each with one entry
    When I dispatch remove-hook with event='AAA', name='a'
    Then the dispatcher returns success=true
    Then the on-disk events appear in the order ZED, AAA, MID
