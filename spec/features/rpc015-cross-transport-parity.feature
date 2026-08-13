@done
@RPC-015
@rust
@rpc
@checkpoint
@checkpoint-management
@parity
@tarpc
@websocket
Feature: RPC-015 cross-transport parity for FspecBackend::checkpoint_counts
  """
  RPC-015 (slice 1b of 3) — Both `EmbeddedFspecBackend` and `WebSocketFspecBackend`
  implement the new `checkpoint_counts()` method on `FspecBackend` and produce
  identical results against the same `SharedFspecService` cwd.

  Also pins the additive `napi::count_checkpoints` NAPI export source-shape so
  the existing TS `countCheckpoints` helper can converge on the shared Rust
  implementation at its own pace.

  Test pair: rust/fspec-tui/tests/checkpoint_counts_rpc015.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want EmbeddedFspecBackend and WebSocketFspecBackend to return identical CheckpointCounts for the same workspace cwd
    So that both the in-process ratatui host and the WebSocket-attached `fspec client` produce identical headers from a shared source of truth

  Scenario: EmbeddedFspecBackend::checkpoint_counts delegates through the shared service
    Given a SharedFspecService constructed via with_cwd against a git repo containing 1 manual + 1 auto checkpoint ref
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.checkpoint_counts().await is invoked
    Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })

  Scenario: WebSocketFspecBackend::checkpoint_counts crosses tarpc cleanly
    Given an rpc-server bound to the SAME shared service (cwd repo with 1 manual + 1 auto ref)
    And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    When backend.checkpoint_counts().await is invoked
    Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })

  Scenario: napi::count_checkpoints is wired through the same git helper
    Given rust/napi/src/git.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "pub fn count_checkpoints"
    And the function body contains the substring "codelet_git::ghost_commit::count_checkpoints"
