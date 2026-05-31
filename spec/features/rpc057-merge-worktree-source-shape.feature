@done
@tui
@RPC-057
@rpc
@rust
@source-shape
Feature: /merge-worktree RPC surface source shape
  """
  Pin the source-shape contract that downstream slices in RPC-030 depend on:

  - The five merge/discard/prune/list/inspect RPC methods MUST be declared
    at every layer of the dual-transport stack (SessionManagerHandle trait
    + tarpc FspecService + FspecBackend trait + both transport forwarders).
  - Five new wire types (MergeStrategy, MergeStatus, MergeOutcome,
    SessionWorktreeInfo, SessionChangesSummary) MUST exist as public
    declarations in codelet-rpc-types.
  - MergeConfirmDialog MUST exist as a public dialog component with the
    documented constructor + render + handle_key surface.
  - All slash-command wiring for /merge-worktree MUST live in
    codelet/fspec-tui/src/app/dispatch_rpc057.rs (mirrors dispatch_rpc056)
    so the orchestrator dispatch_rpc020.rs stays under the 300-LoC ceiling.

  These tests run against source files at compile time — they catch
  refactors that accidentally collapse the dual-transport layering, drop
  any of the five RPC methods, or inline the /merge-worktree wiring back
  into dispatch_rpc020.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Five new wire types live in codelet-rpc-types: MergeStrategy enum, MergeStatus enum,
  #      MergeOutcome struct, SessionWorktreeInfo struct, SessionChangesSummary struct.
  #   2. SessionManagerHandle MUST expose default-impl methods for all five RPC operations.
  #   3. StubSessionManagerHandle MUST expose per-call counters for cross-transport parity tests.
  #   4. FspecService (tarpc) MUST declare async fns for all five operations.
  #   5. FspecBackend trait MUST expose async variants of all five methods.
  #   6. EmbeddedFspecBackend AND WebSocketFspecBackend MUST forward each method to the tarpc client.
  #   7. MergeConfirmDialog MUST live in codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs.
  #   8. All /merge-worktree wiring MUST live in codelet/fspec-tui/src/app/dispatch_rpc057.rs.
  #
  # ========================================

  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the layering of the /merge-worktree RPC surface
    So that no future refactor can collapse the dual-transport boundary or strand the dialog

  Scenario: All five merge/worktree wire types are exported from codelet-rpc-types
    Given the file codelet/rpc-types/src/lib.rs is compiled
    Then it declares a public enum named "MergeStrategy"
    And it declares a public enum named "MergeStatus"
    And it declares a public struct named "MergeOutcome"
    And MergeOutcome has fields named status, conflicts, merge_commit
    And it declares a public struct named "SessionWorktreeInfo"
    And SessionWorktreeInfo has fields named session_id, worktree_path, base_commit, head_commit, dirty
    And it declares a public struct named "SessionChangesSummary"
    And SessionChangesSummary has fields named files_changed, insertions, deletions, commits

  Scenario: SessionManagerHandle declares the five new methods
    Given the file codelet/core/src/session_manager_handle.rs is compiled
    Then it declares a trait method named "merge_session_worktree" returning Result<MergeOutcome, String>
    And it declares a trait method named "discard_session_worktree" returning Result<(), String>
    And it declares a trait method named "prune_orphaned_worktrees" returning Result<Vec<String>, String>
    And it declares a trait method named "list_session_worktrees" returning Vec<SessionWorktreeInfo>
    And it declares a trait method named "inspect_session_changes" returning Result<SessionChangesSummary, String>

  Scenario: StubSessionManagerHandle exposes per-call counters for all five methods
    Given the file codelet/core/src/session_manager_handle.rs is compiled
    Then StubSessionManagerHandle declares a method named "merge_session_worktree_calls" returning u64
    And StubSessionManagerHandle declares a method named "discard_session_worktree_calls" returning u64
    And StubSessionManagerHandle declares a method named "prune_orphaned_worktrees_calls" returning u64
    And StubSessionManagerHandle declares a method named "list_session_worktrees_calls" returning u64
    And StubSessionManagerHandle declares a method named "inspect_session_changes_calls" returning u64

  Scenario: FspecService declares the five new RPC methods
    Given the file codelet/rpc/src/lib.rs is compiled
    Then it declares an async fn named "merge_session_worktree" with return type Result<MergeOutcome, String>
    And it declares an async fn named "discard_session_worktree" with return type Result<(), String>
    And it declares an async fn named "prune_orphaned_worktrees" with return type Result<Vec<String>, String>
    And it declares an async fn named "list_session_worktrees" with return type Vec<SessionWorktreeInfo>
    And it declares an async fn named "inspect_session_changes" with return type Result<SessionChangesSummary, String>

  Scenario: FspecBackend declares the five new methods
    Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    Then it declares an async fn named "merge_session_worktree" on the FspecBackend trait returning Result<MergeOutcome>
    And it declares an async fn named "discard_session_worktree" on the FspecBackend trait returning Result<()>
    And it declares an async fn named "prune_orphaned_worktrees" on the FspecBackend trait returning Result<Vec<String>>
    And it declares an async fn named "list_session_worktrees" on the FspecBackend trait returning Result<Vec<SessionWorktreeInfo>>
    And it declares an async fn named "inspect_session_changes" on the FspecBackend trait returning Result<SessionChangesSummary>

  Scenario: Both transports implement the five new methods
    Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    Then each file contains an impl of "merge_session_worktree" that calls the corresponding tarpc client method
    And each file contains an impl of "discard_session_worktree" that calls the corresponding tarpc client method
    And each file contains an impl of "prune_orphaned_worktrees" that calls the corresponding tarpc client method
    And each file contains an impl of "list_session_worktrees" that calls the corresponding tarpc client method
    And each file contains an impl of "inspect_session_changes" that calls the corresponding tarpc client method

  Scenario: MergeConfirmDialog module exists with the documented entry points
    Given the file codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs exists
    Then it declares a public struct named "MergeConfirmDialog"
    And it declares an enum named "MergeConfirmDialogOutcome" with variants for Merge, Discard, Cancel
    And MergeConfirmDialog declares a constructor "new" taking a SessionId and a SessionChangesSummary
    And MergeConfirmDialog declares a method named "render" taking (&self, Rect, &mut Buffer)
    And MergeConfirmDialog declares a method named "handle_key" taking (&mut self, KeyCode, KeyModifiers) returning MergeConfirmDialogOutcome

  Scenario: /merge-worktree slash command wiring lives in dispatch_rpc057.rs
    Given the file codelet/fspec-tui/src/app/dispatch_rpc057.rs exists
    Then it declares a method named "handle_slash_merge_worktree"
    And it declares a method named "handle_inspect_changes_loaded"
    And it declares a method named "handle_merge_confirmed"
    And it declares a method named "handle_discard_confirmed"
    And it declares a method named "handle_cancel_merge_dialog"
    And it declares a method named "try_dispatch_rpc057"

  Scenario: MergeStrategy and MergeStatus use derive(Default) with default variant attribute (RPC-057 retro 2026-05-27)
    Given the codelet workspace inherits the lint level `-D warnings` which includes `clippy::derivable_impls`
    When I run `cargo clippy -p codelet-sessions -- -D warnings` against the post-fix worktree
    Then clippy exits with code 0 and emits no `clippy::derivable_impls` errors against MergeStrategy or MergeStatus
    Given MergeStrategy is declared in codelet/rpc-types/src/lib.rs with FastForward as the conceptual default and MergeStatus is declared with NoChanges as the conceptual default
    Then the MergeStrategy declaration in codelet/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the FastForward variant, with no remaining manual `impl Default for MergeStrategy` block
    Then the MergeStatus declaration in codelet/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the NoChanges variant, with no remaining manual `impl Default for MergeStatus` block
    Then the Default::default() values are byte-identical to the pre-fix manual impls: MergeStrategy::default() == MergeStrategy::FastForward and MergeStatus::default() == MergeStatus::NoChanges

