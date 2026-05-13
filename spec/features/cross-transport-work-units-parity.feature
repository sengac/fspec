@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@RPC-006
Feature: Cross-transport push parity for WorkUnitsUpdate
  """
  Architecture

  Both the embedded and WebSocket transports MUST produce
  semantically and bytewise identical `Vec<WorkUnitInfo>` payloads for
  the same underlying watcher state. This feature is the push-channel
  analogue of the RPC-005 `dual-transport-parity.feature` (which covered
  request/response parity).

  The bincode encoding of `Envelope::WorkUnitsUpdate(payload)` MUST be
  byte-identical across transports, since downstream pieces (RPC-009
  list view, RPC-010 daemon mode) rely on the wire format being stable
  irrespective of which transport produced the snapshot.

  References: spec/attachments/RPC-006/plan.md (Step 5);
              RPC-005 architecture rule [3] (single source of truth).
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want automated tests proving the embedded and WebSocket push channels emit byte-identical Vec<WorkUnitInfo> payloads for the same workspace mutation
    So that the future work-units list pane (RPC-009) can switch transports without observing different state

  Scenario: Both transports produce byte-identical WorkUnitsUpdate payloads for the same mutation
    Given the same temporary workspace observed by one shared WorkUnitsWatcher exposed through both an EmbeddedTransport and an rpc-server-backed WebSocket client
    When I mutate spec/work-units.json once and collect the resulting Vec<WorkUnitInfo> from EmbeddedTransport::work_units_rx() and from FspecWsClient::work_units_rx()
    Then the two Vec<WorkUnitInfo> values are equal under PartialEq and the bincode encoding of each via Envelope::WorkUnitsUpdate produces identical byte sequences
