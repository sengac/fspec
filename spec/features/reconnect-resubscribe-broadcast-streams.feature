@done
@connection
@tui
@bug-fix
@streaming
@resilience
@rpc
@websocket
@reconnect
@rust
@RPC-415
Feature: Live streaming dies permanently after first auto-reconnect
  """
  Respawn-on-Reconnected design:
  - The 5 broadcast subscriber loops (work_units, chunks, logs, status_changes, session_created) in app/bootstrap.rs each terminate on RecvError::Closed. When the transport supervisor drops the old RPC client after a WS drop, the client's broadcast Senders are dropped, so every subscriber Receiver returns Closed and all 5 tasks exit permanently. spawn_subscriber_tasks() is only called once from App::bootstrap (bootstrap.rs:52), so nothing respawns them.
  - Fix: on Action::Reconnected, App::dispatch must re-invoke the SAME spawn_subscriber_tasks() code path so a fresh set of subscriber tasks is created, subscribed to the CURRENT client's *_rx() receivers (rebound to the new client via the shared Arc<RwLock<Option<FspecWsClient>>> client slot). No duplicated loop bodies (DRY) — bootstrap and reconnect share the single spawn helper.
  - Idempotency under flapping: before respawning, the handler must clear out the dead task handles so repeated Reconnected actions cannot accumulate N x 5 tasks; the old tasks self-exit on Closed and the JoinHandle vec is reset so live_subscriber_task_count stays at the fixed stream count. Each broadcast event is therefore delivered exactly once.
  - The existing one-shot list_work_units() refetch + create_session(None) behaviour in the Reconnected handler is preserved; the respawn is added alongside it.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On Action::Reconnected, the App respawns all broadcast subscriber tasks bound to the new RPC client's receivers
  #   2. After reconnect, every broadcast stream (work_units, chunks, logs, status_changes, session_created) delivers subsequently-emitted events to the App
  #   3. Repeated reconnect cycles (flapping) do not accumulate duplicate subscriber tasks; each stream delivers each event exactly once
  #   4. Respawn reuses the single spawn_subscriber_tasks code path (no duplicated subscriber loop bodies)
  #
  # EXAMPLES:
  #   1. Before reconnect the old subscriber loops have exited (RecvError::Closed) and no live stream reaches the App
  #   2. After a drop and successful reconnect, the new daemon emits a work_units update and the App's WorkUnitsList reflects it without restart
  #   3. After reconnect, an agent chunk streamed by the new daemon appears live in the session scrollback without restart
  #   4. Two disconnect/reconnect cycles in a row still deliver each broadcast event exactly once (no duplicates from leaked tasks)
  #
  # ========================================
  Background: User Story
    As a fspec power-user driving the Rust TUI over WebSocket
    I want to keep receiving live streaming data after the connection auto-recovers from a drop
    So that the board, agent output, logs, and status keep updating without restarting the TUI

  Scenario: Old subscriber loops are dead before reconnect
    Given an App bootstrapped against a backend whose broadcast senders are then closed
    When the backend's broadcast senders are closed so every subscriber receiver observes RecvError::Closed
    Then the original subscriber tasks have all exited and no live stream reaches the App

  Scenario: Each broadcast stream delivers a post-reconnect event to the App
    Given an App bootstrapped against a backend whose subscriber tasks have exited after a simulated disconnect
    When the App dispatches Action::Reconnected and the backend then emits one event on each of the work_units, chunks, logs, status_changes and session_created streams
    Then the App receives a WorkUnitsLoaded action carrying the post-reconnect work_units update
    And the App receives a ChunkReceived action for the post-reconnect chunk
    And the App receives a SessionStatusChanged action for the post-reconnect status change
    And the App receives a SessionCreated action for the post-reconnect session_created event

  Scenario: Respawn binds subscribers to the new client receivers
    Given an App bootstrapped against a backend whose original subscriber tasks have exited after a simulated disconnect
    When the App dispatches Action::Reconnected and the backend emits a work_units update from its current senders
    Then the respawned subscriber tasks are bound to the current receivers and deliver the update as a WorkUnitsLoaded action
    And the live subscriber task count returns to the full set of broadcast streams

  Scenario: Flapping reconnects do not accumulate duplicate subscriber tasks
    Given an App bootstrapped against a backend
    When the App dispatches Action::Reconnected twice in succession and the backend then emits a single work_units update
    Then the live subscriber task count equals the full set of broadcast streams and does not grow with each reconnect
    And the App receives exactly one WorkUnitsLoaded action for that update with no duplicate delivery
