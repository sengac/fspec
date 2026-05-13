@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@RPC-005
Feature: WebSocket transport daemon for the FspecService tarpc surface
  """
  Architecture

  codelet/rpc-server is a minimal WebSocket daemon binary using tokio-tungstenite. It binds 127.0.0.1:0, reports its ephemeral port on stdout, traces to stderr, and shuts down on ctrl_c. The same shared FspecService implementation hosted by codelet/rpc-embedded is reached over the network via an Envelope codec.

  Wire format: bincode is the default; the Envelope enum carries Rpc | Event | LogEvent | WorkUnitsUpdate | CmdReq | CmdRes variants but only Rpc is implemented in this card. All other variants are reserved-but-rejected (server logs a warning and refuses to dispatch).

  References: spec/attachments/RPC-002/rpc-002-feasibility.md sections 5, 6.
  """

  Background: User Story
    As a Rust developer building the new fspec frontend
    I want to reach the FspecService trait via a WebSocket transport with a defensive envelope
    So that the TUI can run remotely without forking the service surface and without accepting unimplemented frame variants

  Scenario: WebSocket transport returns WorkUnitInfo via the rpc-server binary
    Given the rpc-server binary has been spawned bound to 127.0.0.1:0 with its ephemeral port read from stdout, and the shared FspecService implementation it hosts is seeded with a fixture of two WorkUnitInfo records
    When I connect a tokio-tungstenite WebSocket client to that port, obtain an FspecServiceClient over the WebSocket transport, and call list_work_units on the client
    Then the call returns Ok with a Vec<WorkUnitInfo> equal to the fixture

  Scenario: WebSocket frames are encoded with bincode by default
    Given the rpc-server is running with default configuration
    When a WebSocket client sends a list_work_units RPC request and receives the response while the bytes of both frames are captured
    Then the captured frame bytes successfully decode with bincode into the expected Envelope::Rpc value and the captured frame bytes are not valid UTF-8 JSON

  Scenario: Reserved envelope variants are rejected by the server
    Given the rpc-server is running
    When a WebSocket client sends a frame whose Envelope variant is one of Event, LogEvent, WorkUnitsUpdate, CmdReq, or CmdRes
    Then the server records the unsupported variant by name in its rejection log and does not invoke any FspecService method as a result of that frame
