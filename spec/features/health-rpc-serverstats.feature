@done
@workflow-automation
@websocket
@tarpc
@rust
@cli
@rpc
@RPC-011
Feature: Health RPC + ServerStats extensions (HealthInfo lifted type + lag counters + watcher event timestamp)

  """
  FspecService gains one new RPC: health() -> HealthInfo. HealthInfo is a new lifted type in
  codelet-rpc-types, cfg-gated napi(object) like its siblings (WorkUnitInfo, SessionInfo).
  ServerStats grows the underlying atomics + mutex. FspecBackend trait gains an async health()
  method; embedded reads ServerStats directly, WebSocket goes through tarpc.
  """

  Background: User Story
    As a power-user driving the rust binary
    I want the daemon/client trio to survive interruptions and report health
    So that I can day-drive the frontend without rough edges

  Scenario: FspecService.health returns HealthInfo via tarpc
    Given a tarpc client connected to FspecServiceImpl
    When the client calls health(context::current()).await
    Then it receives a HealthInfo struct over the wire
    And HealthInfo fields are: uptime_secs: i64, connected_clients: i64, last_watcher_event_secs_ago: Option<i64>, lag_chunks: i64, lag_logs: i64, lag_work_units: i64, version: String
    And the version field equals env!("CARGO_PKG_VERSION") of the daemon process

  Scenario: HealthInfo is a lifted type with cfg-gated napi(object)
    Given the codelet-rpc-types crate
    When inspecting HealthInfo's definition
    Then it carries #[cfg_attr(feature = "napi", napi(object))]
    And it implements Serialize + Deserialize + Clone + Debug
    And it lives in codelet/rpc-types/src/lib.rs alongside WorkUnitInfo / SessionInfo

  Scenario: FspecBackend trait gains health on both transports
    Given the FspecBackend trait
    When inspecting its method surface
    Then it has an `async fn health(&self) -> Result<HealthInfo>` method
    And EmbeddedFspecBackend implements health by reading ServerStats directly (no RPC round-trip)
    And WebSocketFspecBackend implements health by calling self.client.client().health(context::current()).await

  Scenario: ServerStats lag counters fire when broadcast subscribers lag
    Given a daemon with the chunks broadcast capacity set to 1024
    And a single slow WS subscriber that NEVER drains its receiver
    When 1025 chunk frames are pushed onto chunks_tx in rapid succession
    Then the slow subscriber's recv() yields RecvError::Lagged(1)
    And ServerStats.lag_chunks is incremented by at least 1 in the chunks_fanout task
    And a tracing::warn record is emitted with target="codelet_rpc_server::server" and field skipped>=1
    And that warning rides the logs broadcast as a LogRecord visible to OTHER (non-lagging) clients

  Scenario: ServerStats.last_watcher_event_at updates on each watcher snapshot
    Given a daemon with an empty workspace
    When the workspace mutates and the watcher fires a new snapshot
    Then ServerStats.last_watcher_event_at.lock() is updated to the current Instant in work_units_fanout
    And subsequent health() calls report last_watcher_event_secs_ago = Some(elapsed.as_secs())
