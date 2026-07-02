@diff-viewer
@git-integration
@rpc
@RPC-355
Feature: Expose git changed-file status and per-file diff to the TUI transport
  """
  FspecBackend trait (transport/mod.rs) gains changed_files() + file_diff(path) with default impls (empty/None). embedded.rs adds one-line delegates; websocket.rs uses the client.read().await + BackendError::Disconnected guard pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. changed_files() returns staged files first, then unstaged, then untracked entries
  #   2. Each ChangedFile carries a change_type: untracked -> A; missing-from-workdir -> D; otherwise M
  #   3. changed_files() returns an empty Vec when the shared service has no cwd attached (no panic)
  #   4. file_diff(path) returns the unified diff text for a modified file
  #   5. file_diff(path) returns None when the file has no changes or no cwd is attached
  #   6. Both transports (embedded + websocket) expose identical changed_files and file_diff semantics
  #
  # EXAMPLES:
  #   1. get_staged_files_with_change_type returns a tracked-but-modified staged file as change_type M
  #   2. get_unstaged_files_with_change_type marks a tracked file deleted from the workdir as change_type D
  #   3. An untracked working-tree file appears in changed_files as change_type A with staged=false
  #   4. EmbeddedFspecBackend.changed_files() against a temp repo with one modified and one untracked file returns both entries
  #   5. EmbeddedFspecBackend.file_diff(path) for a modified file returns Some(diff) containing the changed lines
  #   6. changed_files() on a service constructed without with_cwd returns an empty Vec
  #   7. WebSocketFspecBackend.changed_files() crosses tarpc and returns the same entries as the embedded backend
  #
  # ========================================
  Background: User Story
    As a Rust TUI developer
    I want to request the working tree's changed files (each with a change type) and a per-file unified diff through the TUI transport backend
    So that the File Changes view can render a colored status list and a live diff pane from real git data

  Scenario: Embedded backend changed_files returns modified and untracked entries
    Given a SharedFspecService constructed via with_cwd against a git repo with one modified and one untracked file
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.changed_files().await is invoked
    Then the result contains an entry for the modified file and an entry for the untracked file

  Scenario: Embedded backend file_diff returns the unified diff for a modified file
    Given a SharedFspecService constructed via with_cwd against a git repo with a modified file
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.file_diff(path).await is invoked for the modified file
    Then the result is Some diff text containing the changed lines

  Scenario: changed_files returns an empty Vec when no cwd is attached
    Given a SharedFspecService constructed without with_cwd
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.changed_files().await is invoked
    Then the awaited result is an empty Vec

  Scenario: WebSocket backend changed_files crosses tarpc and matches the embedded backend
    Given an rpc-server bound to a SharedFspecService with a cwd repo containing one modified and one untracked file
    And a WebSocketFspecBackend connected to that server
    When backend.changed_files().await is invoked
    Then the result contains an entry for the modified file and an entry for the untracked file

  Scenario: file_diff returns the binary-file sentinel for a binary file
    Given a SharedFspecService constructed via with_cwd against a git repo with a committed-then-modified binary file
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.file_diff(path).await is invoked for the binary file
    Then the result is Some text equal to the binary-file sentinel
