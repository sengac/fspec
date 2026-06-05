@done
@rust
@cli
@RPC-247
Feature: Port list-hooks command to Rust

  """
  New impl file at codelet/fspec-core/src/commands/list_hooks.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_prefixes::run. Args struct deserializes `{format?: 'text'|'json'}` with `#[serde(default)]`.
  Hook config is parsed using a lightweight Rust shape: `struct HookConfigPartial { hooks: IndexMap<String, Vec<serde_json::Value>> }` so that insertion order is preserved AND we can pluck `.name` as `Option<String>` regardless of whether the entry has extra/missing fields. We deliberately do NOT model the full HookDefinition struct because list-hooks only needs the name field (parity with TS `hooks.map(h => h.name)`).
  Error swallowing: the impl uses a single try-block (Rust equivalent: a helper fn returning Result, then `.unwrap_or_else(|_| empty_result_with_message())`) that catches BOTH the std::fs::read_to_string IO error AND the serde_json::from_str parse error, mapping each to the canonical empty `{events:[], message:'No hooks are configured'}` shape. This is intentionally wider than list-prefixes' swallowing pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-hooks` MUST replace the NotYetPorted stub
  #   2. ENOENT on spec/fspec-hooks.json → success with events=[] + message='No hooks are configured'
  #   3. Invalid JSON in spec/fspec-hooks.json → ALSO swallowed (success with events=[] + message)
  #   4. Empty hooks object → events=[] but NO message field
  #   5. Insertion order preserved (IndexMap)
  #   6. Missing `name` field → emits null in JSON
  #   7. JSON format wraps in `{events:[...]}` (plus optional message) with 2-space indent
  #   8. Text format: `No hooks are configured` for empty; `Configured Hooks:\n` + events for populated
  #   9. CLI surface is flag-less (parity with TS Commander.js)
  #   10. CLI delegates to single source of truth in fspec_core
  #   11. CLI resolves project_root from CWD; exits 0 on success
  #   12. No new shared io helpers needed
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-hooks from the agent loop AND invoke `fspec list-hooks` from a shell
    So that I can audit the lifecycle hooks configured for the project, sharing one source-of-truth between the LLM dispatcher and the CLI without going through Node.js

  Scenario: Returns empty events with 'No hooks are configured' when spec/fspec-hooks.json does not exist
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-hooks command against that project root with format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has events array of length 0
    Then the parsed JSON has message field equal to 'No hooks are configured'
    Then spec/fspec-hooks.json does not exist after the call

  Scenario: Returns event/hook mapping when spec/fspec-hooks.json is populated
    Given spec/fspec-hooks.json contains event 'post-implementing' with hooks named 'lint' and 'test' in that order
    When I dispatch list-hooks with format='json'
    Then the dispatcher returns success=true
    Then the events array contains exactly one entry
    Then the first event has event='post-implementing' and hooks=['lint','test']
    Then the parsed JSON does NOT contain a top-level 'message' field

  Scenario: Treats empty hooks object as no events without a message field
    Given spec/fspec-hooks.json exists and parses to an object whose 'hooks' field is the empty object
    When I dispatch list-hooks with format='json'
    Then the dispatcher returns success=true
    Then the events array has length 0
    Then the parsed JSON does NOT contain a top-level 'message' field

  Scenario: Swallows invalid JSON as empty result with 'No hooks are configured' message
    Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    When I dispatch list-hooks with format='json'
    Then the dispatcher returns success=true
    Then the events array has length 0
    Then the parsed JSON has message field equal to 'No hooks are configured'

  Scenario: Preserves insertion order of events (not alphabetical)
    Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID
    When I dispatch list-hooks with format='json'
    Then the dispatcher returns success=true
    Then the events array contains three entries in order ZED, AAA, MID

  Scenario: Emits null for hooks missing the name field
    Given spec/fspec-hooks.json contains event 'pre-implementing' with two hook entries — the first with name='lint' and the second with NO name field
    When I dispatch list-hooks with format='json'
    Then the dispatcher returns success=true
    Then the first event's hooks array equals ['lint', null]

  Scenario: JSON format emits two-space indented payload for the empty/missing case
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-hooks with format='json'
    Then the DispatchResult.data starts with the exact string "{\n  \"events\": [],\n"
    Then the DispatchResult.data contains the exact substring "\"message\": \"No hooks are configured\""

  Scenario: Text format renders the populated case using the documented help-example layout
    Given spec/fspec-hooks.json contains event 'pre-implementing' with hooks ['lint'] and event 'post-implementing' with hooks ['test', 'notify'] in that order
    When I dispatch list-hooks with format='text'
    Then the DispatchResult.data contains the line 'Configured Hooks:'
    Then the substring 'pre-implementing:' appears before 'post-implementing:' in the output
    Then the DispatchResult.data contains the exact line 'pre-implementing:'
    Then the DispatchResult.data contains the exact line '  - lint'
    Then the DispatchResult.data contains the exact line 'post-implementing:'
    Then the DispatchResult.data contains the exact line '  - test'
    Then the DispatchResult.data contains the exact line '  - notify'

  Scenario: Text format prints 'No hooks are configured' for the empty/missing case
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-hooks with format='text'
    Then the DispatchResult.data is exactly the string 'No hooks are configured'

  Scenario: Default format (no format key supplied) is text
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-hooks with an empty args object {}
    Then the dispatcher returns success=true
    Then the DispatchResult.data is exactly the string 'No hooks are configured'

  Scenario: Renders unnamed placeholder when a hook lacks the name field
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single hook entry that has NO name field but a command field
    When I dispatch list-hooks with format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact line '  - (unnamed)'

