@rpc
@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@file-watcher
@RPC-006
Feature: Pure-Rust WorkUnitsWatcher in codelet-core
  """
  Architecture

  RPC-006 lifts the cross-platform `notify`-based work-units watcher out
  of `rust/napi/src/work_units_watcher.rs` into a new pure-Rust module
  at `rust/core/src/work_units.rs`. The lift drops the
  `ThreadsafeFunction` Node bridge and replaces it with a
  `tokio::sync::broadcast::Sender<Vec<WorkUnitInfo>>` so the same watcher
  instance can fan out to multiple subscribers (an embedded RPC reader
  and the WebSocket fan-out task in rust/rpc-server).

  Public surface (pure-Rust):
  pub fn read_snapshot(workspace: &Path) -> Result<Vec<WorkUnitInfo>>
  pub struct WorkUnitsWatcher with new(&Path) -> Result<Self>,
  snapshot() -> Vec<WorkUnitInfo>,
  subscribe() -> tokio::sync::broadcast::Receiver<Vec<WorkUnitInfo>>

  Broadcast capacity is a bounded `broadcast::channel(64)` per
  architecture note 12 — lagging subscribers receive `RecvError::Lagged`
  and resync on the next snapshot (acceptable because every payload is a
  full snapshot, not an incremental delta).

  References: spec/attachments/RPC-006/plan.md (Step 1);
  spec/attachments/RPC-002/rpc-002-feasibility.md §5.1;
  spec/attachments/RPC-006/ast-research-watcher-lift.md §1.
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want a pure-Rust WorkUnitsWatcher reachable from codelet-core
    So that both the embedded transport and the WebSocket server can subscribe to live workspace updates without pulling NAPI into the dependency arrow

  Scenario: WorkUnitsWatcher publishes a new snapshot on file mutation
    Given a temporary workspace observed by a WorkUnitsWatcher and a broadcast::Receiver<Vec<WorkUnitInfo>> obtained via watcher.subscribe()
    When I append a third work unit to spec/work-units.json and wait up to one second on the receiver
    Then the receiver yields a Vec<WorkUnitInfo> containing all three work units in the order they appear in the file

  Scenario: Multiple subscribers each observe every broadcast on file mutation
    Given a temporary workspace observed by a single WorkUnitsWatcher and two independent broadcast::Receiver values obtained via two separate watcher.subscribe() calls
    When I mutate spec/work-units.json once and wait up to one second on each receiver
    Then both receivers yield equal Vec<WorkUnitInfo> values reflecting the post-mutation state
