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
Feature: Embedded transport push channel for work-units updates
  """
  Architecture

  After the RPC-006 watcher lift, `SharedFspecService::new(Arc<WorkUnitsWatcher>)`
  reads from a real `WorkUnitsWatcher` instead of the RPC-005 hard-coded
  fixture. The embedded transport exposes a transport-agnostic API that
  the future ratatui frontend (RPC-008/RPC-009) consumes regardless of
  whether it is in-process or remote:

    EmbeddedTransport::work_units_rx() -> tokio::sync::broadcast::Receiver<Vec<WorkUnitInfo>>

  This method returns the watcher's broadcast subscription DIRECTLY — no
  envelope encoding, no fan-out task on the embedded read path (zero-cost
  push per RPC-002 §5.1). The runtime-Handle invariant from RPC-005 Q9 is
  preserved: any spawning that does occur uses `self.handle.spawn(...)`.

  References: spec/attachments/RPC-006/plan.md (Steps 2, 4);
              spec/attachments/RPC-002/rpc-002-feasibility.md §5.1;
              RPC-005 architecture rule [4].
  """

  Background: User Story
    As a Rust developer building the new fspec frontend
    I want the embedded transport to expose a broadcast::Receiver of live work-unit snapshots and a list_work_units RPC backed by the real watcher
    So that an in-process ratatui app can render live work-units state without forking the type system or the runtime

  Scenario: list_work_units returns a live snapshot from the real WorkUnitsWatcher
    Given a temporary workspace whose spec/work-units.json file declares two work units and a SharedFspecService constructed from `Arc::new(WorkUnitsWatcher::new(workspace)?)`
    When I construct an EmbeddedTransport with the current tokio runtime Handle, obtain an FspecServiceClient, and call list_work_units on the client
    Then the call returns Ok with a Vec<WorkUnitInfo> equal to the live snapshot derived from the spec/work-units.json file and not equal to the RPC-005 default_fixture

  Scenario: Embedded transport exposes the watcher's broadcast subscription directly
    Given a SharedFspecService backed by a real WorkUnitsWatcher and an EmbeddedTransport built from the current tokio runtime Handle
    When I call EmbeddedTransport::work_units_rx() to obtain a broadcast::Receiver<Vec<WorkUnitInfo>>, mutate spec/work-units.json once, and wait up to one second on the receiver
    Then the receiver yields the post-mutation Vec<WorkUnitInfo> and the transport source contains no bincode encode call on the embedded push path
