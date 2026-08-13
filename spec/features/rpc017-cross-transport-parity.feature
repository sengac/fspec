@done
@RPC-017
@rust
@rpc
@tui
@persistence
@parity
@work-units
Feature: RPC-017 cross-transport parity for FspecBackend::move_work_unit_up/_down
  """
  RPC-017 (slice 2 of 3) — Both `EmbeddedFspecBackend` and
  `WebSocketFspecBackend` implement the new `move_work_unit_up` /
  `move_work_unit_down` methods on the `FspecBackend` trait. Both
  transports delegate to the SAME shared `FspecServiceImpl` (RPC-005
  rule 4), which in turn delegates to
  `codelet_core::work_units_write::move_work_unit(cwd, id, direction)`.

  Cwd discovery flows through `SharedFspecService::with_cwd` (added in
  RPC-015). When no cwd is attached the RPC returns Err rather than
  silently succeeding so the TUI surfaces a tracing diagnostic.

  Also pins the additive `napi::move_work_unit_up / _down` exports'
  source shape so the TS shim can converge on the shared Rust helper
  at its own pace.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want EmbeddedFspecBackend and WebSocketFspecBackend to return identical results from move_work_unit_up/_down against the same SharedFspecService cwd
    So that both the in-process ratatui host and the WebSocket-attached `fspec client` persist reorders through the same shared helper

  Scenario: EmbeddedFspecBackend.move_work_unit_up persists through the shared helper
    Given a SharedFspecService::with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002", "C-003"]
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.move_work_unit_up("C-003".into()).await is invoked
    Then the awaited result is Ok(())
    And the workspace's spec/work-units.json now has states.backlog == ["A-001", "C-003", "B-002"]

  Scenario: WebSocketFspecBackend.move_work_unit_down crosses tarpc cleanly
    Given an rpc-server bound to a SharedFspecService::with_cwd whose states.backlog == ["A-001", "B-002", "C-003"]
    And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    When backend.move_work_unit_down("A-001".into()).await is invoked
    Then the awaited result is Ok(())
    And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001", "C-003"]

  Scenario: Both transports return Err for done-column targets
    Given a SharedFspecService::with_cwd whose states.done == ["DONE-001", "DONE-002"]
    And both EmbeddedFspecBackend and WebSocketFspecBackend wrapping that shared service
    When each transport calls move_work_unit_up("DONE-001".into()).await
    Then both calls return Err

  Scenario: napi::move_work_unit_up is wired through the same shared helper
    Given rust/napi/src/work_units_watcher.rs after RPC-017 lands
    When a developer reads the file source raw
    Then the file contains the substring "pub fn move_work_unit_up"
    And the file contains the substring "pub fn move_work_unit_down"
    And both function bodies contain the substring "codelet_core::work_units_write::move_work_unit"

  Scenario: FspecService::move_work_unit_up delegates through SharedFspecService::cwd to the shared helper
    Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    When client.move_work_unit_up(context::current(), "B-002") is invoked
    Then the call returns Ok(())
    And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]

  Scenario: FspecService::move_work_unit_down delegates through SharedFspecService::cwd to the shared helper
    Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    When client.move_work_unit_down(context::current(), "A-001") is invoked
    Then the call returns Ok(())
    And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]

  Scenario: FspecService::move_work_unit_up returns Err when no cwd is attached
    Given a SharedFspecService constructed via new() WITHOUT with_cwd
    When client.move_work_unit_up(context::current(), "A-001") is invoked
    Then the call returns Err
