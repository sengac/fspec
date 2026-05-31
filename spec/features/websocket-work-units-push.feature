@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@streaming
@real-time
@RPC-006
Feature: WebSocket WorkUnitsUpdate envelope variant
  """
  Architecture

  RPC-006 lights up the first non-`Rpc` envelope variant on the
  WebSocket transport:
  Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)

  Per RPC-002 §5.2, push events ride a sibling `tokio::sync::broadcast`
  channel alongside the existing tarpc request/response surface — NOT a
  tarpc stream return. Tarpc keeps strict req/res semantics.

  Server-side (`codelet/rpc-server/src/server.rs::handle_connection`):
  per-connection task subscribes to the shared watcher, sends an INITIAL
  snapshot frame immediately on connection (no explicit subscribe RPC),
  then forwards every broadcast event as a bincode-encoded
  `Envelope::WorkUnitsUpdate(snapshot)` onto the WS sink.

  Client-side (`codelet/rpc-server/src/pump.rs::ClientInbound`): the
  envelope pump grows a second outbound channel — `WorkUnitsUpdate` frames
  are decoded and forwarded to a `broadcast::Sender<Vec<WorkUnitInfo>>`
  exposed via `FspecWsClient::work_units_rx()`.

  Wire format remains bincode-of-Envelope (RPC-005 architecture rule [5]).

  References: spec/attachments/RPC-006/plan.md (Step 3);
  spec/attachments/RPC-002/rpc-002-feasibility.md §5.2.
  """

  Background: User Story
    As a Rust developer building the new fspec frontend
    I want the WebSocket transport to emit bincode-encoded Envelope::WorkUnitsUpdate frames on connection and on every workspace mutation
    So that a remote ratatui frontend can observe live work-units state with the same guarantees as the embedded transport

  Scenario: WebSocket client receives an initial WorkUnitsUpdate frame on connection
    Given the rpc-server binary spawned bound to 127.0.0.1:0 over a temporary workspace whose spec/work-units.json declares two work units, with its ephemeral port read from stdout
    When a WebSocket client connects to that port and reads exactly one frame from the inbound channel before any file mutation
    Then the frame decodes with bincode into Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>) carrying the two work units from the workspace

  Scenario: WebSocket client receives a WorkUnitsUpdate frame on file mutation
    Given the rpc-server binary spawned bound to 127.0.0.1:0 over a temporary workspace and a connected WebSocket client whose initial snapshot frame has been consumed
    When I append a third work unit to spec/work-units.json and the client waits up to one second on FspecWsClient::work_units_rx()
    Then the receiver yields a Vec<WorkUnitInfo> containing all three work units and the corresponding inbound frame on the wire decoded with bincode as Envelope::WorkUnitsUpdate carrying the same payload

  Scenario: WorkUnitsUpdate frames are encoded with bincode and not JSON
    Given a connected WebSocket client and a workspace mutation that triggers exactly one push frame
    When the client captures the raw bytes of the inbound frame
    Then the captured bytes successfully decode with bincode into Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>) and the captured bytes are not valid UTF-8 JSON
