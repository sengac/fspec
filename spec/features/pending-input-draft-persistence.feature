@done
@persistence
@session-management
@rpc
@multi-session
@agent-view
@tui
@rust
@RPC-052
Feature: Pending-input draft persistence on session switch
  """
  Phase 6.7 of RPC-030. Wires durable backend-backed storage for the per-session input draft.

  Current state (RPC-024): SessionContext.input_draft snapshots the MultiLineInput buffer synchronously on Shift+Left/Right cycle. Drafts are lost on process restart, session destroy, and don't sync across clients.

  Wire shape:
  - New Action variants:
  Action::PendingInputChanged(text)               // keystroke→backend (debounced)
  Action::SeedPendingInput { session_id, text }    // backend→input fold
  - AgentView (views/agent/dispatch.rs): when input.handle_event returns Continued AND the buffer text changed, emit Action::PendingInputChanged(self.input.value()).
  - App field: pending_input_save_handle: Option<JoinHandle<()>> — single in-flight debounced save per App.
  - App::dispatch(PendingInputChanged): abort any existing pending_input_save_handle, spawn a new task that sleeps 300ms then awaits backend.set_pending_input(session, Some(text)). Store handle.
  - Hydration on session activation: SessionCreated and AttachToSession spawn backend.get_pending_input(session) and on Ok(Some(text)) dispatch Action::SeedPendingInput.
  - Action::SeedPendingInput: ONLY seed the live MultiLineInput when session_id == agent_view_store.current_session(); always mirror into SessionContext.input_draft for the matching context.
  - Action::InputSubmitted: after the existing send_input + history persistence, spawn backend.set_pending_input(session, None).
  - Errors from set/get pending_input are silently logged via tracing — no scrollback notice.
  - SessionContext.input_draft mirror is preserved for the synchronous handle_session_cycle path (RPC-024 behaviour unchanged for Shift+Left/Right).

  Files touched:
  - codelet/fspec-tui/src/components/mod.rs (new Action variants)
  - codelet/fspec-tui/src/views/agent/dispatch.rs (emit PendingInputChanged on edit)
  - codelet/fspec-tui/src/app/state.rs (pending_input_save_handle field)
  - codelet/fspec-tui/src/app/dispatch.rs (route the new variants)
  - codelet/fspec-tui/src/app/dispatch_rpc052.rs (NEW — debounce + hydration helpers)
  - codelet/fspec-tui/src/app/mod.rs (pub mod dispatch_rpc052)
  - codelet/fspec-tui/src/app/dispatch_rpc020.rs (clear draft after InputSubmitted)
  - codelet/fspec-tui/src/app/dispatch_rpc026.rs (hydrate on AttachToSession)
  - codelet/fspec-tui/tests/common/mod.rs (MockBackend pending_input scripting)
  - codelet/fspec-tui/tests/pending_input_durability_rpc052.rs (NEW integration tests)

  Last-write-wins semantics: aborting the in-flight save is acceptable per the attachment risk note. No version counter.

  Dependencies:
  - RPC-037 already wired backend.{get,set}_pending_input on EmbeddedFspecBackend, WebSocketFspecBackend, MockBackend trait defaults, and FspecService.
  - RPC-051 introduced the AgentEscPressed dispatch_rpc051.rs pattern this card mirrors.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Typing into MultiLineInput triggers Action::PendingInputChanged on every edit
  #   2. Action::PendingInputChanged is debounced by 300ms before calling backend.set_pending_input(session, Some(text))
  #   3. A second PendingInputChanged within the debounce window cancels the previous pending save
  #   4. On session activation (SessionCreated/AttachToSession/SessionPrev/SessionNext) the App spawns backend.get_pending_input(session) and seeds the live MultiLineInput when the result is Some(text)
  #   5. Successful Action::InputSubmitted spawns backend.set_pending_input(session, None) to clear the durable draft
  #   6. SessionContext.input_draft mirrors the live MultiLineInput buffer so handle_session_cycle still works without a backend round-trip on every keystroke
  #   7. Errors from set_pending_input or get_pending_input are silently dropped (logged via tracing); they never panic the UI or emit notice/scrollback
  #   8. Hydration on session activation only seeds the live input when the activated session is still the focused session at the moment the result arrives
  #
  # EXAMPLES:
  #   1. User types 'hello' over 250ms then waits 350ms; backend.set_pending_input(s, Some("hello")) is called exactly once
  #   2. User types 'h', 'i' in two rapid edits within 100ms; only one debounced backend save fires after 300ms idle with the final value 'hi'
  #   3. User types a draft on session A, kills fspec, restarts, attaches to session A — the live input is restored to the draft text
  #   4. User submits the input via Enter — the durable draft is cleared (backend.set_pending_input(session, None) is called) and the next session activation does not restore old draft
  #   5. User switches via Shift+Right to a session whose backend pending_input is Some('partial sentence') — the live MultiLineInput shows 'partial sentence'
  #   6. User switches to a session whose backend pending_input is None — the live MultiLineInput shows an empty buffer
  #   7. Backend.set_pending_input returns Err — UI remains responsive and no scrollback notice is emitted
  #   8. Rapid Shift+Left/Right cycling between two sessions immediately reflects each session's last typed draft (mirrors RPC-024 behaviour preserved by SessionContext.input_draft)
  #   9. Hydration completes 100ms after the user has already switched away to a different session — the late result is dropped and the now-focused session's input is NOT overwritten
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView
    I want to have my per-session draft text persisted across session switches, process restarts, and multi-client sessions
    So that I do not lose work-in-progress text when navigating or reconnecting

  # ─────────────────────────────────────────────────────────────────────
  # Keystroke → backend save (debounced)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Typing fires Action::PendingInputChanged when the buffer text changes
    Given an App with a MockBackend
    And session s-1 is the current session
    And the live MultiLineInput is empty
    When the user types the character "h"
    Then Action::PendingInputChanged("h") is dispatched
    And the AgentView's MultiLineInput value equals "h"

  Scenario: Cursor-only key events do not fire Action::PendingInputChanged
    Given an App with a MockBackend
    And session s-1 is the current session
    And the live MultiLineInput contains "hello"
    And the cursor is at the end of the buffer
    When the user presses Left arrow with no modifiers
    Then no Action::PendingInputChanged is dispatched
    And the AgentView's MultiLineInput value still equals "hello"

  Scenario: Single PendingInputChanged triggers exactly one debounced backend save
    Given an App with a MockBackend
    And session s-1 is the current session
    When Action::PendingInputChanged("hello") is dispatched
    And the debounce timer of 300ms elapses
    Then within 1 second backend.set_pending_input is called exactly once with (s-1, Some("hello"))

  Scenario: Rapid PendingInputChanged events coalesce into a single backend save with the final value
    Given an App with a MockBackend
    And session s-1 is the current session
    When Action::PendingInputChanged("h") is dispatched
    And Action::PendingInputChanged("hi") is dispatched within 50ms
    And the debounce timer of 300ms elapses since the last edit
    Then within 1 second backend.set_pending_input is called exactly once with (s-1, Some("hi"))
    And backend.set_pending_input is NOT called with (s-1, Some("h"))

  Scenario: PendingInputChanged with no current session is a silent no-op
    Given an App with a MockBackend
    And there is NO current session
    When Action::PendingInputChanged("orphan") is dispatched
    And the debounce timer of 300ms elapses
    Then backend.set_pending_input is NEVER called

  Scenario: PendingInputChanged updates SessionContext.input_draft mirror immediately
    Given an App with a MockBackend
    And session s-1 is the current session
    When Action::PendingInputChanged("draft-mirror") is dispatched
    Then AgentViewStore.session_context_for(s-1).input_draft equals "draft-mirror" synchronously

  # ─────────────────────────────────────────────────────────────────────
  # Hydration → seed live input on session activation
  # ─────────────────────────────────────────────────────────────────────
  Scenario: SessionCreated hydrates the live input from backend.get_pending_input
    Given an App with a MockBackend
    And the MockBackend's get_pending_input is scripted to return Some("restored draft") for s-1
    When Action::SessionCreated(s-1) is dispatched
    And all pending tasks have drained
    Then backend.get_pending_input is called exactly once with s-1
    And the AgentView's MultiLineInput value equals "restored draft"

  Scenario: SessionCreated with backend returning None leaves the input empty
    Given an App with a MockBackend
    And the MockBackend's get_pending_input is scripted to return None for s-1
    When Action::SessionCreated(s-1) is dispatched
    And all pending tasks have drained
    Then backend.get_pending_input is called exactly once with s-1
    And the AgentView's MultiLineInput is empty

  Scenario: AttachToSession hydrates the live input from backend.get_pending_input
    Given an App with a MockBackend
    And session s-1 is open (already created)
    And the MockBackend's get_pending_input is scripted to return Some("attach-draft") for s-2
    When Action::AttachToSession(s-2) is dispatched
    And all pending tasks have drained
    Then backend.get_pending_input is called at least once with s-2
    And the AgentView's MultiLineInput value equals "attach-draft"

  Scenario: SeedPendingInput is dropped when the activated session is no longer focused
    Given an App with a MockBackend
    And session s-1 is the current session
    And the AgentView's MultiLineInput contains "typing-now"
    When Action::SeedPendingInput { session_id: s-2, text: "stale" } is dispatched
    Then the AgentView's MultiLineInput value still equals "typing-now"
    And AgentViewStore.session_context_for(s-1).input_draft equals "typing-now"

  Scenario: SeedPendingInput updates SessionContext.input_draft for the target session even when not focused
    Given an App with a MockBackend
    And open sessions are [s-1, s-2] and s-1 is focused
    When Action::SeedPendingInput { session_id: s-2, text: "background-draft" } is dispatched
    Then AgentViewStore.session_context_for(s-2).input_draft equals "background-draft"
    And the AgentView's MultiLineInput value still reflects s-1's buffer (unchanged)

  # ─────────────────────────────────────────────────────────────────────
  # Submit → clear the durable draft
  # ─────────────────────────────────────────────────────────────────────
  Scenario: InputSubmitted clears the durable draft via backend.set_pending_input(None)
    Given an App with a MockBackend
    And session s-1 is the current session
    When the user submits the input "hello world" via Enter
    And all pending tasks have drained
    Then within 1 second backend.set_pending_input is called with (s-1, None)

  # ─────────────────────────────────────────────────────────────────────
  # Error tolerance
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Backend.set_pending_input error does not panic and emits no scrollback notice
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's set_pending_input is scripted to return Err("network blip")
    When Action::PendingInputChanged("draft") is dispatched
    And the debounce timer of 300ms elapses
    And all pending tasks have drained
    Then the App must not panic
    And the session s-1 scrollback contains no chunks mentioning "set_pending_input"
    And the session s-1 scrollback contains no chunks mentioning "network blip"

  Scenario: Backend.get_pending_input error during hydration leaves the input empty without panicking
    Given an App with a MockBackend
    And the MockBackend's get_pending_input is scripted to return Err("decode failed") for s-1
    When Action::SessionCreated(s-1) is dispatched
    And all pending tasks have drained
    Then the App must not panic
    And the AgentView's MultiLineInput is empty

  # ─────────────────────────────────────────────────────────────────────
  # Session cycling preserves RPC-024 fast path
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Shift+Left/Right cycling uses SessionContext.input_draft (no backend round-trip per cycle)
    Given an App with a MockBackend
    And open sessions are [s-1, s-2] and s-1 is focused
    And the AgentView's MultiLineInput contains "draft-on-s1"
    When the user presses Shift+Right
    Then AgentViewStore.session_context_for(s-1).input_draft equals "draft-on-s1"
    And the focused session is s-2
    And the AgentView's MultiLineInput reflects s-2's SessionContext.input_draft
    When the user presses Shift+Left
    Then the focused session is s-1
    And the AgentView's MultiLineInput value equals "draft-on-s1"

  # ─────────────────────────────────────────────────────────────────────
  # Source shape
  # ─────────────────────────────────────────────────────────────────────
  Scenario: codelet/fspec-tui/src/app/dispatch_rpc052.rs hosts the new debounce + hydration helpers
    Given the file codelet/fspec-tui/src/app/dispatch_rpc052.rs exists
    When the file is compiled as part of codelet-fspec-tui
    Then it must declare impl App methods named handle_pending_input_changed, handle_seed_pending_input, and spawn_hydrate_pending_input
    And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_rpc052
    And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
