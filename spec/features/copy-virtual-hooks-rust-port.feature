@done
@querying
@cli
@rust
@RPC-209
Feature: Port copy-virtual-hooks command to Rust
  """
  New impl file at rust/fspec-core/src/commands/copy_virtual_hooks.rs replaces the NotYetPorted stub. Module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` mirroring the list_virtual_hooks::run signature.

  The shell-facing CLI bridge is delivered alongside this port — see rust/fspec/src/copy_virtual_hooks.rs and spec/features/copy-virtual-hooks-cli-subcommand.feature. Both front doors (LLM tool-call dispatcher and clap shell subcommand) call the same `copy_virtual_hooks::run` function defined in this port.

  Args parsed via serde camelCase: `{from: String (default ''), to: String (default ''), hookName: Option<String>}`. `from`/`to` empty after default → CLI bridge emits the friendly "--from option is required" / "--to option is required" error before delegating. The core function ALSO defends both fields and raises the same canonical error strings when invoked over the dispatcher with empty/missing values.

  The command reads `spec/work-units.json` via the shared `crate::io::ensure::ensure_work_units_file` helper. Validation order (matching the TS source):
  1. Source work unit must exist → `Error("Source work unit '<from>' does not exist")`.
  2. Target work unit must exist → `Error("Target work unit '<to>' does not exist")`.
  3. Source must have at least one virtualHook → `Error("No virtual hooks configured for source work unit <from>")` (no single quotes around `<from>`).
  4. If `hookName` is supplied, it must match the `name` field of at least one source hook → `Error("Hook '<hookName>' not found in <from>")`.

  Selected hooks are deep-cloned (`Value::clone()`) and APPENDED to the target's `extra["virtualHooks"]` array (initialized to `[]` if missing). Existing target hooks are preserved at the front. Target `updated_at` is bumped via `crate::io::time::iso8601_now()`; the source unit's `updated_at` is NOT touched. State is persisted with a single `crate::io::locked_file::write_json_atomic` call.

  Output shape (JSON): `{"success": true, "copiedCount": <n>}` — serialized via `#[derive(Serialize)]` struct so insertion order is preserved. Text format returned by `run`: `"✓ Copied <n> virtual hook(s) from <from> to <to>"`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust dispatcher route for copy-virtual-hooks replaces the NotYetPorted stub
  #   2. Both from and to are required; absence surfaces TS-compatible errors at the CLI bridge before the core function runs
  #   3. Source missing returns success=false with exact "Source work unit '<from>' does not exist"
  #   4. Target missing returns success=false with exact "Target work unit '<to>' does not exist"
  #   5. Source has no virtualHooks returns success=false with exact "No virtual hooks configured for source work unit <from>"
  #   6. When hookName specified and not found returns success=false with exact "Hook '<hookName>' not found in <from>"
  #   7. Without hookName all source hooks are deep-cloned and appended to target.virtualHooks (existing target hooks preserved)
  #   8. With hookName only that hook (matched by name field) is copied
  #   9. Target updatedAt is bumped to ISO-8601 now; source updatedAt is NOT touched
  #   10. Persistence uses a single write_json_atomic over spec/work-units.json after mutations
  #   11. No script files are generated or copied — copy is config-only
  #   12. CLI delegates to single source of truth in fspec_core via the two-front-doors pattern
  #   13. CLI stdout success: "✓ Copied <n> virtual hook(s) from <from> to <to>"
  #   14. Help intercept produces byte-exact output matching node dist/index.js copy-virtual-hooks --help
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch copy-virtual-hooks from the agent loop AND invoke `fspec copy-virtual-hooks --from <src> --to <dst>` from a shell
    So that I can replicate virtual hook configuration across related work units from a single source of truth

  Scenario: Copies all source hooks into an empty target
    Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing), 'test' (post-implementing), 'eslint' (pre-validating)
    Given spec/work-units.json contains AUTH-002 with no virtualHooks field
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    Then the dispatcher returns success=true
    Then the parsed JSON has copiedCount=3
    Then spec/work-units.json AUTH-002 virtualHooks contains the names ['lint','test','eslint'] in that order
    Then spec/work-units.json AUTH-001 virtualHooks is unchanged (same length and names)
    Then spec/work-units.json AUTH-002 updatedAt is newer than its prior value
    Then spec/work-units.json AUTH-001 updatedAt is NOT bumped

  Scenario: Copies only the named hook when hookName is supplied
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint', 'test', 'eslint'
    Given spec/work-units.json contains AUTH-002 with no virtualHooks
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002' and hookName='eslint'
    Then the dispatcher returns success=true
    Then the parsed JSON has copiedCount=1
    Then spec/work-units.json AUTH-002 virtualHooks contains a single entry with name='eslint'

  Scenario: Copied hooks are APPENDED after existing target hooks (existing entries preserved)
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    Given spec/work-units.json contains AUTH-002 with virtualHook 'old-hook' already configured
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    Then the dispatcher returns success=true
    Then spec/work-units.json AUTH-002 virtualHooks contains names ['old-hook','lint','test'] in that order

  Scenario: Source work unit missing returns the canonical source error
    Given spec/work-units.json contains AUTH-002 only
    When I dispatch copy-virtual-hooks with from='MISSING-001' and to='AUTH-002'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Source work unit 'MISSING-001' does not exist"

  Scenario: Target work unit missing returns the canonical target error
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint'
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='MISSING-002'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Target work unit 'MISSING-002' does not exist"

  Scenario: Source with no virtualHooks returns the no-hooks-configured error
    Given spec/work-units.json contains AUTH-001 with no virtualHooks
    Given spec/work-units.json contains AUTH-002 with no virtualHooks
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "No virtual hooks configured for source work unit AUTH-001"

  Scenario: hookName not present in source returns the hook-not-found error
    Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    Given spec/work-units.json contains AUTH-002 with no virtualHooks
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002' and hookName='missing'
    Then the dispatcher returns success=false
    Then the error message contains the exact substring "Hook 'missing' not found in AUTH-001"

  Scenario: Missing from argument is rejected as InvalidArgs
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch copy-virtual-hooks with to='AUTH-002' and no from key
    Then the dispatcher returns success=false
    Then the error message indicates that --from option is required

  Scenario: Missing to argument is rejected as InvalidArgs
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch copy-virtual-hooks with from='AUTH-001' and no to key
    Then the dispatcher returns success=false
    Then the error message indicates that --to option is required

  Scenario: Result JSON shape preserves field order success then copiedCount
    Given spec/work-units.json contains AUTH-001 with virtualHook 'lint' and AUTH-002 with no hooks
    When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses to a JSON object whose first key is "success" and whose second key is "copiedCount"
