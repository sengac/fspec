@critical
@infrastructure
@RPC-010
@rpc
@rust
@tarpc
@websocket
@workspace
Feature: fspec combined mode — TUI + always-on WS server in one process
  """
  RPC-010 (parent RPC-002, depends on RPC-009). Combined mode (`fspec` with
  no subcommand) starts the shared service + ratatui frontend (via
  EmbeddedFspecBackend) + ALWAYS-ON WebSocket server (via the SAME
  bind_and_serve from RPC-005 that daemon mode uses). This is the entry
  point the user invokes the most.

  Architecture invariants (locked by rules [3] [4] [5] [7] [23]):
  • SAME bind_and_serve as daemon mode — no second WS server.
  • PORT=<n> line goes to STDERR (stdout is reserved for the alt-screen
  TUI canvas; corrupting stdout would garble the ratatui buffer).
  • daemon.json is written on bootstrap so a sibling `fspec client`
  can autodiscover the running combined process.
  • EmbeddedFspecBackend is constructed with tokio::runtime::Handle::current()
  — preserving the RPC-005 Q9 host-supplied-runtime invariant.
  • On clean exit: App::run().await → JoinHandle::abort() →
  remove_daemon_json → return Ok(()) → tokio::main returns.

  Source artifacts:
  • codelet/fspec/src/combined.rs (NEW)
  • codelet/fspec/src/common.rs::build_service (NEW)
  • codelet/rpc-server/src/server.rs::bind_and_serve (existing — RPC-005)
  • codelet/fspec-tui/src/transport/embedded.rs::EmbeddedFspecBackend (existing — RPC-008)
  • codelet/fspec-tui/src/app.rs::App (existing — RPC-008/RPC-009)
  """

  Background: 
    Given the fspec binary has been built via `cargo build -p fspec --release`
    And a temp workspace exists with a seeded spec/work-units.json containing at least one WorkUnit

  @smoke
  @end-to-end
  @integration-test
  Scenario: Combined mode boots the TUI and starts the WS server in one process
    Given the developer has cd'd into the temp workspace
    When the developer spawns `fspec` as a subprocess with stdin/stdout/stderr piped
    Then within 5 seconds the child process is still running
    And the child has bound a WebSocket listener on 127.0.0.1:<ephemeral-port>
    And the same child has called ratatui::init() (alt-screen + raw mode active)

  @smoke
  @end-to-end
  Scenario: Combined mode emits the PORT=<n> banner on STDERR (not stdout)
    When the developer spawns `fspec` as a subprocess
    And waits until the WS server is listening
    Then exactly one line matching `^PORT=(\d+)$` appears on the child's STDERR
    And the same line does NOT appear anywhere on the child's STDOUT
    And the captured STDOUT contains only ratatui control codes / cell drawing

  @smoke
  Scenario: Combined mode does not corrupt the alt-screen TUI canvas
    When the developer spawns `fspec` and pipes its STDOUT to a buffer
    And the App has completed its bootstrap (left pane seeded, REPL session created)
    Then the captured STDOUT buffer contains NO occurrence of the literal text "PORT="
    And the captured STDOUT buffer contains NO occurrence of the literal text "listening"
    And the captured STDOUT buffer contains only ratatui's escape-sequence cell stream

  @end-to-end
  @integration-test
  Scenario: External WS client can attach to combined mode and call list_work_units
    Given the developer has spawned `fspec --workspace <temp-workspace>` as a subprocess
    And the test has parsed the `PORT=<n>` line from the child's STDERR
    When the test constructs a SECOND `WebSocketFspecBackend::connect(ws://127.0.0.1:<n>)` from the test process
    And the test calls `list_work_units().await` on that backend
    Then the call returns a non-empty Vec<WorkUnitInfo>
    And the Vec contains every WorkUnit seeded in the temp workspace's spec/work-units.json

  @end-to-end
  @integration-test
  Scenario: Combined mode writes daemon.json on bootstrap and removes it on clean exit
    Given the developer has set HOME to a tempdir BEFORE spawning the child
    When the developer spawns `fspec` as a subprocess
    And waits for the WS server to be listening
    Then the file at `<HOME>/.fspec/daemon.json` exists
    And that file is valid JSON with at minimum keys `port`, `pid`, `workspace`, `version`
    And `port` equals the listening port observed on STDERR
    And `pid` equals the child process's pid
    When the test sends SIGINT to the child
    And the child exits with code 0 within 5 seconds
    Then the file at `<HOME>/.fspec/daemon.json` no longer exists

  @end-to-end
  Scenario: Combined mode shutdown aborts the WS server JoinHandle before removing daemon.json
    Given the developer has spawned `fspec` as a subprocess
    And the test has attached an external WS client subscribed to work_units_rx
    When the test sends SIGINT to the child
    Then the external WS client observes a connection-closed error (not a hang) within 5 seconds
    And after the connection-closed error the child's daemon.json file is gone
    And finally the child process exits with code 0

  @parity
  @integration-test
  Scenario: Combined mode and daemon mode share the SAME bind_and_serve function
    """
    Source-shape regression: both combined.rs AND daemon.rs MUST call
    `codelet_rpc_server::bind_and_serve(addr, service)` — there is no
    second WS server, no parallel "embedded server" path. This is the
    invariant that lets RPC-011's `fspec status` health RPC be a single
    code path across both modes.
    """
    Given the file `codelet/fspec/src/combined.rs` exists
    And the file `codelet/fspec/src/daemon.rs` exists
    Then `combined.rs` contains exactly one call to `bind_and_serve(`
    And `daemon.rs` contains exactly one call to `bind_and_serve(`
    And no other file under `codelet/fspec/src/` calls `bind_and_serve(`

  @smoke
  Scenario: Combined mode uses tokio::runtime::Handle::current for the embedded backend
    """
    RPC-005 Q9 invariant preserved at the binary boundary. The
    EmbeddedFspecBackend constructor takes a non-defaulted Handle; the
    only source of that Handle in combined mode is the host's
    `#[tokio::main]` runtime accessed via `Handle::current()`.
    """
    Given the file `codelet/fspec/src/combined.rs` exists
    Then it contains the literal call `tokio::runtime::Handle::current()`
    And it contains no occurrence of `tokio::runtime::Builder`
    And it contains no occurrence of `Runtime::new`
    And the `EmbeddedFspecBackend::new(handle, service.clone())` construction is reachable from the file's top-level run function

  @smoke
  Scenario: Combined mode bootstraps with build_service constructed exactly once
    """
    Single-instance invariant from RPC-005 preserved at the binary
    boundary: `common::build_service(workspace)` produces one
    Arc<SharedFspecService> that is shared between bind_and_serve AND
    EmbeddedFspecBackend::new — the same watcher backs both transports.
    """
    Given the file `codelet/fspec/src/combined.rs` exists
    Then it calls `common::build_service(` exactly once
    And the returned Arc<SharedFspecService> is passed to both `bind_and_serve` and `EmbeddedFspecBackend::new`
