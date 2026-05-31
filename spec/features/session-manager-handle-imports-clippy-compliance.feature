@done
@lift
@session
@regression
@source-shape
@bug-fix
@rust
@code-quality
@RPC-076
Feature: skeleton_invariants clippy fails on unused_imports NotificationSeverity in codelet-core session_manager_handle

  """
  Workspace-wide clippy lint `clippy::unused_imports` (implied by `-D warnings`) stays at deny — no `#[allow(unused_imports)]` escape hatch is added to session_manager_handle.rs
  Fix Option A (drop the import) is correct because the WIP diff shows NotificationSeverity was imported but the original UserNotification broadcast it was for has been intentionally removed (see git diff: 'The previous UserNotification { message: ... } broadcast for /clear was a Rust-side invention with no counterpart in the TypeScript reference… and has been removed')
  Out of scope: any other clippy violations elsewhere in codelet-core or codelet-sessions; any tui-test or e2e behavioural tests (this is a compile-time lint fix with zero behavioural change — no runtime surface to exercise)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The `NotificationSeverity` symbol MUST NOT appear in the `use codelet_rpc_types::{...}` import list in codelet/core/src/session_manager_handle.rs
  #   2. `cargo clippy -p codelet-core --all-targets -- -D warnings` MUST report zero unused_imports errors against session_manager_handle.rs after the fix
  #   3. `cargo test -p codelet-sessions --test skeleton_invariants -- scenario_workspace_lints_are_inherited_and_clippy_passes` MUST pass after the fix
  #   4. A source-shape regression test MUST assert that `NotificationSeverity` is not re-introduced as an unused import in codelet/core/src/session_manager_handle.rs
  #   5. The remaining imports in the `use codelet_rpc_types::{...}` block in session_manager_handle.rs MUST continue to compile (i.e. PauseState and ProviderInfo on line 33 are unaffected — only NotificationSeverity is removed)
  #
  # EXAMPLES:
  #   1. Before fix: line 33 reads `NotificationSeverity, PauseState, ProviderInfo,` and `cargo clippy -p codelet-core` emits `error: unused import: \`NotificationSeverity\``. After fix: line 33 reads `PauseState, ProviderInfo,` (or equivalent comma-merged with line 32) and clippy exits 0.
  #   2. Running `cargo test -p codelet-sessions --test skeleton_invariants -- scenario_workspace_lints_are_inherited_and_clippy_passes` exits 0 with no failures
  #   3. A new source-shape regression test `rpc076_session_manager_handle_imports_shape.rs` in codelet/sessions/tests/ scans the use-block in session_manager_handle.rs and panics if `NotificationSeverity` is re-introduced without a use-site
  #   4. `cargo clippy -p codelet-core --all-targets -- -D warnings` exits 0 with no unused_imports diagnostics against session_manager_handle.rs
  #
  # ========================================

  Background: User Story
    As a developer maintaining the Rust port
    I want to have the codelet-core session_manager_handle.rs comply with the workspace-wide `clippy::unused_imports = deny` policy by removing the unused NotificationSeverity import
    So that `scenario_workspace_lints_are_inherited_and_clippy_passes` goes green and stops blocking every subsequent RPC card on the codelet-integration branch

  Scenario: session_manager_handle.rs use-block does not import NotificationSeverity
    Given the file `codelet/core/src/session_manager_handle.rs` exists in the workspace
    When I scan the `use codelet_rpc_types::{...}` import block at the top of that file
    Then the symbol `NotificationSeverity` does not appear in the import list
    Then the symbols `PauseState` and `ProviderInfo` continue to appear in the import list


  Scenario: cargo clippy on codelet-core emits no unused_imports diagnostic against session_manager_handle.rs
    Given the workspace lint set denies `unused_imports` (implied by `-D warnings`)
    When I run `cargo clippy -p codelet-core --all-targets -- -D warnings`
    Then no `unused_imports` diagnostic is emitted against `core/src/session_manager_handle.rs`


  Scenario: codelet-core skeleton-invariants workspace-lint precondition: codelet-core itself passes -D warnings
    Given the codelet/Cargo.toml workspace lints declaration is inherited by codelet-core
    Given session_manager_handle.rs no longer imports `NotificationSeverity`
    When I run `cargo clippy -p codelet-core --all-targets -- -D warnings`
    Then the command exits 0 with no errors


  Scenario: Source-shape regression test pins the absence of unused NotificationSeverity import
    Given the file `codelet/core/src/session_manager_handle.rs` exists in the workspace
    When I scan the file for occurrences of the identifier `NotificationSeverity`
    Then either the identifier does not appear at all, or every occurrence inside the `use codelet_rpc_types::{...}` block is matched by at least one use-site elsewhere in the same file

