@done
@tui-component
@agent-view
@rust
@RPC-385
Feature: Spawned subordinate agents are not registered/visible in the Rust TUI
  """
  Backend broadcast: add a session_created broadcast sender to SessionManager (rust/sessions/src/session_manager.rs) alongside chunks_tx/logs_tx/status_changes_tx, with a session_created_tx() accessor. Fire it inside create_session_with_id right after sessions.write().insert(uuid, session), next to the existing broadcast_metadata_update() call. Payload should carry at least the SessionId (and name if convenient via SessionInfo).
  Transport surface: add fn session_created_rx(&self) -> broadcast::Receiver<...> to the FspecBackend trait (rust/fspec-tui/src/transport/mod.rs). Embedded backend returns session_manager.session_created_tx().subscribe(). Provide a safe default (e.g. a never-firing receiver) for remote/tarpc backend so it compiles; full remote-transport parity is an explicit OUT-OF-SCOPE follow-up.
  TUI subscriber: in rust/fspec-tui/src/app/bootstrap.rs::spawn_subscriber_tasks, add a fifth subscriber task that consumes session_created_rx and folds each event into Action::SessionCreated(id). Mirror the work_units_rx lag-recovery pattern (RecvError::Lagged -> debug log + continue, RecvError::Closed -> break).
  Idempotent append: make handle_session_created / AgentViewStore::append_session idempotent. Before appending, check whether a SessionContext with that SessionId already exists in open_sessions; if so, no-op (do not append a duplicate tab, do not steal focus). This makes the broadcast safe for ALL creation paths: user-initiated tabs already exist (no-op), spawned subordinates get added exactly once. Keep every touched Rust file < 300 lines; extract helpers if needed. Tests use the *_parity_rpc385.rs convention plus store-level unit tests for idempotency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManager broadcasts a session-created event whenever any session is created via create_session_with_id
  #   2. When a subordinate session is spawned (by AgentManager, not the TUI), the Rust TUI appends a new session tab for it
  #   3. Appending a session is idempotent: a session id that already has a tab does not create a duplicate tab
  #   4. TUI-initiated session creation continues to produce exactly one tab (no regression, no double-append)
  #   5. The session-created subscriber recovers from a lagged broadcast receiver without crashing (mirrors work_units_rx lag handling)
  #
  # EXAMPLES:
  #   1. An operator with no sessions open spawns a subordinate via AgentManager; a new agent tab appears for the subordinate session id
  #   2. create_session_with_id fires the session-created broadcast; a subscriber receives the new session id
  #   3. Two session-created events for the same id result in a single tab (the second is a no-op)
  #   4. A user creates a session via the create-session dialog; exactly one tab appears even though both the dialog and the broadcast feed Action::SessionCreated
  #   5. When the session-created subscriber reports a lagged receiver, it continues processing subsequent events instead of terminating
  #
  # ========================================
  Background: User Story
    As a operator running the Rust TUI
    I want to see a tab/session appear when an agent spawns a subordinate via AgentManager
    So that I can monitor and navigate to subordinate agents instead of them running invisibly

  Scenario: A spawned subordinate appears as a new tab in the TUI
    Given a running TUI with no sessions open
    When a subordinate session is spawned via AgentManager and its session-created event is delivered
    Then the TUI appends a new agent tab for the subordinate session id

  Scenario: A duplicate session-created event does not create a second tab
    Given a TUI that already has a tab for a session id
    When a second session-created event arrives for the same session id
    Then the TUI still shows exactly one tab for that session id

  Scenario: TUI-initiated session creation produces exactly one tab
    Given a user opens the create-session dialog and confirms a new session
    When both the dialog and the session-created broadcast feed Action::SessionCreated for the same id
    Then exactly one tab appears for the new session

  Scenario: The subscriber recovers from a lagged broadcast receiver
    Given the session-created subscriber whose receiver has lagged
    When the subscriber observes the lagged receiver error
    Then it continues processing subsequent session-created events instead of terminating
