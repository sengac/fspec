@BUG-582
@done
@tui
@rust
@infrastructure
@parity
@rpc
@tarpc
@RPC-009
@BUG-351
@BUG-356
@BUG-299
@BUG-354
@critical
Feature: App bootstrap sequence + subscriber tasks + action bus extensions (RPC-009)
  """
  BUG-356: the create_session(None) and Action::FocusNext scenarios were deleted. Bootstrap no longer creates a session (the agent session is created lazily) and Action::FocusNext no longer exists, so both had only been borrowing other tests' coverage ranges. Test file: rust/fspec-tui/tests/app_bootstrap_rpc009.rs.
  On `App::run()` perform EXACTLY this sequence in order: (1) `backend.list_work_units()` → seed left pane (also primes `stick_to_bottom = true` and selects index 0); (2) `backend.create_session(None)` → seed right pane's active session; (3) spawn THREE subscriber tasks reading `backend.work_units_rx()`, `backend.chunks_rx()`, and `backend.logs_rx()`, converting each message into an `Action::*` on the existing `mpsc::UnboundedSender<Action>` action bus. Each spawn uses `tokio::spawn` on the host runtime (NEVER `tokio::runtime::Builder` or `Runtime::new()` — preserves RPC-005 Q9). The chunks subscriber filters by the AgentReplView's active session id BEFORE sending Action::ChunkReceived. Subscriber tasks NEVER touch component state directly — every change flows through the action bus. App run loop drains action_rx and special-cases InputSubmitted/Interrupt/FocusNext BEFORE calling compositor.update; `Action::InputSubmitted(text)` spawns `backend.send_input(active_session, text)` on the host runtime; `Action::Interrupt` spawns `backend.interrupt(active_session)`; `Action::FocusNext` mutates the RootView's focused_pane field. Subscriber tasks honour broadcast::error::RecvError::Lagged(n) by logging at debug and continuing the loop (work_units subscriber additionally re-fetches a fresh snapshot via `backend.list_work_units()` after Lagged). Action enum delta in src/components/mod.rs adds `LoadWorkUnits`, `WorkUnitsLoaded(Vec<WorkUnitInfo>)`, `SessionCreated(SessionId)`, `ChunkReceived(SessionId, StreamChunk)`, `InputSubmitted(String)`, `Interrupt`, `FocusNext` — existing Quit/Redraw/Custom variants stay.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want App::run() to perform the explicit bootstrap sequence — list_work_units → seed left pane, create_session → seed right pane's active session, spawn three subscriber tasks for work_units_rx/chunks_rx/logs_rx using the host runtime — and to special-case dispatch InputSubmitted/Interrupt/FocusNext BEFORE compositor.update
    So that the App wires real data and a real session through the dual-transport seam end-to-end without ever calling tokio::runtime::Runtime::new

  Scenario: App bootstrap calls backend.list_work_units() and seeds the left pane
    Given a MockBackend seeded with [AUTH-001 done, AUTH-002 implementing]
    And an App constructed against that backend on an 80x24 TestBackend
    When the App's bootstrap runs
    Then MockBackend.list_work_units_calls equals 1
    And the WorkUnitsListView's items equals [AUTH-001 done, AUTH-002 implementing]
    And the WorkUnitsListView's state.selected() returns Some(0)

  Scenario: App bootstrap spawns three subscriber tasks via tokio::spawn on the host runtime
    Given an App constructed against a MockBackend on a `#[tokio::test]` runtime
    When the App's bootstrap runs
    Then the bootstrap subscriber tasks are alive on the current tokio Handle
    And one task drains `backend.work_units_rx()` and sends `Action::WorkUnitsLoaded(units)` to the action bus
    And one task drains `backend.chunks_rx()` and forwards every session's chunks to the action bus, transcript content as `Action::Transcript(Live(id, chunk))`
    And one task drains `backend.logs_rx()` and forwards records to the action bus or tracing layer
    And no `tokio::runtime::Builder` or `Runtime::new()` call appears in the App bootstrap path

  Scenario: work_units broadcast event becomes an Action::WorkUnitsLoaded on the action bus
    Given an App constructed against a MockBackend with bootstrap complete
    When the test calls `mock.push_work_units(vec![AUTH-001 done, AUTH-002 implementing, AUTH-003 backlog])`
    Then within 200ms the App's action bus receives an `Action::WorkUnitsLoaded` carrying the new three-entry list
    And the WorkUnitsListView's items equals the three-entry list after compositor.update is called

  Scenario: Transcript chunks for the active session become live transcript actions
    Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    When the test calls `mock.push_chunk(SessionId::new("s-mock-1"), StreamChunk::text("hello".into()))`
    Then within 200ms the App's action bus receives an `Action::Transcript(Live(SessionId::new("s-mock-1"), StreamChunk::text("hello".into())))`

  Scenario: Transcript chunks for a background session are forwarded as live transcript actions
    Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    When the test calls `mock.push_chunk(SessionId::new("s-other"), StreamChunk::text("not for us".into()))`
    Then within 200ms the App's action bus receives an `Action::Transcript(Live(SessionId::new("s-other"), ..))` because RPC-045 removed the active-session filter

  Scenario: Action::InputSubmitted dispatches backend.send_input and is forwarded to compositor.update
    Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    When the App processes `Action::InputSubmitted("hi".into())` on the action bus
    Then `MockBackend.send_input` is invoked exactly once with `(SessionId("s-mock-1"), "hi")`
    And the action is also forwarded into compositor.update so layers can react if needed

  Scenario: Action::Interrupt dispatches backend.interrupt
    Given an App with active_session = Some(SessionId("s-mock-1"))
    When the App processes `Action::Interrupt` on the action bus
    Then `MockBackend.interrupt` is invoked exactly once with `SessionId("s-mock-1")`
    And the App's `should_quit` flag is unchanged

  Scenario: Subscriber tasks honour RecvError::Lagged by logging at debug and continuing
    Given an App constructed against a MockBackend
    And the work_units broadcast channel is intentionally lagged by overflowing its capacity
    When the work_units subscriber task observes `RecvError::Lagged(n)`
    Then the task does NOT panic
    And the task logs at debug level with a "Lagged" message
    And the task subsequently re-fetches a snapshot via `backend.list_work_units()` and emits a fresh `Action::WorkUnitsLoaded`

  Scenario: Action enum gains the bootstrap variants while existing variants are preserved
    Given the Action enum in rust/fspec-tui/src/components/mod.rs
    Then it contains the existing variant Quit (TUI-127 removed Custom(String) and TUI-132 removed Redraw, LoadWorkUnits and FocusNext: nothing emitted or handled them)
    And it additionally contains WorkUnitsLoaded(Vec<WorkUnitInfo>)
    And it additionally contains SessionCreated(SessionId)
    And it additionally contains ChunkReceived(SessionId, StreamChunk)
    And it additionally contains InputSubmitted(SessionId, String)
    And it additionally contains Interrupt
    And the enum still derives Clone, Debug
    And the enum drops PartialEq and Eq because StreamChunk does not derive PartialEq
