@critical
@infrastructure
@RPC-010
@rpc
@rust
@tarpc
@websocket
@tui
Feature: fspec client mode — frontend-only WS attach with autodiscovery
  """
  RPC-010 (parent RPC-002, depends on RPC-009). Client mode
  (`fspec client`) is a PURE WebSocket frontend — it does NOT construct
  a SharedFspecService, does NOT bind a WS listener, and does NOT
  embed a backend. Its only job is to discover a running daemon (or use
  an explicit `--connect` URL), open a WebSocketFspecBackend, and run
  the SAME `App` used by combined mode.

  daemon.json autodiscovery resolution order (rule [10]):
  1. `$XDG_RUNTIME_DIR/fspec/daemon.json` (if XDG_RUNTIME_DIR is set)
  2. `<HOME>/.fspec/daemon.json` (fallback)

  Disconnect dialog (rules [12] [25]):
  • On WS connection drop mid-session, the App surfaces a
  `Priority::Critical` dialog with text including
  "daemon disconnected", "q to quit", "r to reconnect".
  • `q` → exit App.
  • `r` → FULL bootstrap: drop old backend, abort the three
  subscriber tasks, fresh `WebSocketFspecBackend::connect(url)`,
  fresh `list_work_units` + `create_session` + respawn subscribers.
  • Single attempt only; full retry/backoff is RPC-011.

  Source artifacts:
  • rust/fspec/src/client.rs (NEW)
  • rust/fspec-tui/src/transport/websocket.rs::WebSocketFspecBackend (existing — RPC-008)
  • rust/fspec-tui/src/app.rs::App + bootstrap (existing — RPC-009)
  """

  Background: 
    Given the fspec binary has been built via `cargo build -p fspec --release`
    And a temp workspace exists with a seeded spec/work-units.json

  @smoke
  @end-to-end
  @integration-test
  Scenario: `fspec client --connect ws://...` opens the WS connection and runs the App
    Given the developer has spawned `fspec daemon --workspace <temp-workspace>` as a subprocess
    And the test has captured the daemon's listening port `<P>` from STDOUT
    When the developer spawns `fspec client --connect ws://127.0.0.1:<P>` as a subprocess
    Then within 5 seconds the daemon's `ServerStats.service.list_work_units_calls` counter has incremented by 1
    And within 5 seconds the daemon has accepted exactly one new WebSocket connection
    And the client child is still running

  @smoke
  @end-to-end
  @integration-test
  Scenario: Plain `fspec client` (no --connect) reads ~/.fspec/daemon.json for autodiscovery
    Given the developer has set HOME to a tempdir BEFORE spawning the daemon
    And the developer has spawned `fspec daemon` as a subprocess
    And the file at `<HOME>/.fspec/daemon.json` now exists with the daemon's port
    When the developer spawns plain `fspec client` (no --connect flag) with the same HOME
    Then within 5 seconds the daemon's `list_work_units_calls` counter has incremented by 1
    And the client did not need to be told the port explicitly

  @smoke
  Scenario: `fspec client` prefers $XDG_RUNTIME_DIR/fspec/daemon.json when set
    Given XDG_RUNTIME_DIR is set to `<XDG>` BEFORE spawning the daemon
    And HOME is set to a DIFFERENT tempdir `<HOME>` (with NO daemon.json)
    And the developer has spawned `fspec daemon` which wrote daemon.json under `<XDG>/fspec/daemon.json`
    When the developer spawns plain `fspec client` with the same XDG_RUNTIME_DIR + HOME
    Then the client resolved its connect URL from `<XDG>/fspec/daemon.json` (not `<HOME>/.fspec/daemon.json`)
    And the client successfully bootstrapped against the daemon

  @smoke
  @error
  Scenario: Plain `fspec client` fails fast when no daemon.json exists
    Given HOME is set to a fresh tempdir with no `.fspec/` directory
    And XDG_RUNTIME_DIR is unset
    When the developer spawns plain `fspec client` (no --connect flag)
    Then the child exits with a non-zero code within 2 seconds
    And the child's STDERR contains the substring `no daemon.json found`
    And the child's STDERR contains the substring `--connect`

  @smoke
  Scenario: Client mode emits nothing on STDOUT or STDERR under normal operation
    """
    Client mode runs the ratatui frontend; STDOUT is the alt-screen
    canvas. STDERR is reserved for panic backtraces only (logs go to
    `~/.fspec/client.log` instead).
    """
    Given a daemon is running and reachable
    When the developer spawns `fspec client --connect ws://127.0.0.1:<P>` and captures both streams
    Then the captured STDERR contains no log lines from `init_tracing_client`
    And every tracing event from the client process is written to `<HOME>/.fspec/client.log` instead

  @smoke
  Scenario: Client mode does not construct a SharedFspecService
    Given the file `rust/fspec/src/client.rs` exists
    Then it contains no call to `common::build_service`
    And it contains no call to `bind_and_serve`
    And it contains no construction of `EmbeddedFspecBackend`
    And it does call `WebSocketFspecBackend::connect`

  @smoke
  Scenario: Client mode does not call ratatui::init directly — App owns the TerminalGuard
    Given the file `rust/fspec/src/client.rs` exists
    Then the file constructs an App via `App::new(Arc::new(backend))` exactly once
    And the file calls `.bootstrap().await` on that App before `.run().await` (RPC-009 sequence)
    And the file calls `.run().await` on that App exactly once
    And the file does not call `ratatui::init` directly (TerminalGuard inside App owns it)

  @end-to-end
  @integration-test
  Scenario: WS disconnect mid-session surfaces a critical-priority disconnect dialog
    Given the developer has spawned `fspec daemon` and captured port `<P>`
    And the developer has spawned `fspec client --connect ws://127.0.0.1:<P>`
    And the client has finished bootstrap (left pane seeded)
    When the test sends SIGKILL to the daemon
    And the WebSocketFspecBackend's underlying connection drops
    Then within 5 seconds the client's Compositor has a layer at `Priority::Critical`
    And that layer's rendered body contains the substring `daemon disconnected`
    And that layer's rendered body contains the substring `q to quit`
    And that layer's rendered body contains the substring `r to reconnect`

  @end-to-end
  @integration-test
  Scenario: Pressing `q` in the disconnect dialog quits the App
    Given a client is showing the disconnect dialog after a daemon kill
    When a synthetic `Key('q')` event is dispatched to the App
    Then the App's `should_quit` flag flips to true
    And `App::run` returns Ok(())
    And the client child process exits with code 0

  @end-to-end
  @integration-test
  Scenario: Pressing `r` performs a full reconnect bootstrap (new session, fresh subscribers)
    """
    Locked design (Q4 answer): reconnect = drop old WebSocketFspecBackend,
    abort the three subscriber tasks, `WebSocketFspecBackend::connect(url)`,
    same bootstrap sequence as RPC-009 — old session_id is discarded.
    Single attempt; full retry/backoff is RPC-011.
    """
    Given a client is showing the disconnect dialog after a daemon kill
    And the test has started a FRESH daemon at the SAME port `<P>`
    And the test has recorded the client's pre-disconnect active session id `<S1>`
    When a synthetic `Key('r')` event is dispatched to the App
    Then the App constructs a new `WebSocketFspecBackend::connect(ws://127.0.0.1:<P>)`
    And the App calls `list_work_units` on the new backend (observed via fresh daemon's `list_work_units_calls` counter incrementing to 1)
    And the App calls `create_session(None)` on the new backend (yielding a new session id `<S2>` != `<S1>`)
    And the App respawns the three subscriber tasks (`subscriber_task_count() == 3`)
    And the disconnect dialog is removed from the compositor
    And the client renders the work-units list pane successfully

  @end-to-end
  Scenario: Reconnect attempt does not loop — single try only in this card
    Given a client is showing the disconnect dialog after a daemon kill
    And NO fresh daemon has been started
    When a synthetic `Key('r')` event is dispatched to the App
    Then the new `WebSocketFspecBackend::connect` attempt fails with a connection-refused error
    And the disconnect dialog remains on the compositor (still showing `r to reconnect`)
    And the App does NOT enter a retry loop or backoff timer
    And the user must press `q` to exit, or `r` again to manually retry once

  @end-to-end
  @integration-test
  @smoke
  Scenario: Client bootstrap observability — daemon-side counter increments by 1
    """
    Locked design (Q2 answer): client-mode smoke tests use daemon-side
    observability ONLY — no pty harness, no terminal-buffer cell parsing.
    This proves the client connected and performed its RPC-009 bootstrap
    sequence without requiring tui-test infrastructure.
    """
    Given the developer has spawned `fspec daemon` and the test holds an external WS observer
    And the daemon's `list_work_units_calls` counter starts at 0
    When the developer spawns `fspec client --connect ws://127.0.0.1:<P>` as a subprocess
    Then within 5 seconds the daemon's `list_work_units_calls` counter equals exactly 1
    And the assertion succeeded without parsing the client's STDOUT or attaching a pty
