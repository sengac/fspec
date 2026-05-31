@done
@multi-session
@pause-integration
@dialog
@rust
@tui
@agent-view
@rpc
@session-management
@persistence
@RPC-053
Feature: Pause / HITL UI (ConfirmDialog + HitlDialog end-to-end)
  """
  New Action variants in components/mod.rs: Action::OpenPauseDialog{session_id, state}, Action::OpenHitlDialog{session_id, request}, Action::PauseConfirmed{session_id, accept: bool}, Action::PauseTriple{session_id, choice: ApprovalChoice}, Action::PauseResumed{session_id}, Action::HitlSubmitted{session_id, response: HitlResponse}, Action::PauseCleared{session_id} (sent by chunk dispatcher on Running/Idle to pop any mounted dialog)
  Create codelet/fspec-tui/src/components/pause_dialog.rs hosting PauseDialog (priority Critical, uses Accent::Yellow via dialog_theme::render_dialog, 2-row layout for Confirm and 3-row layout for Triple). PauseDialog::new(session_id, state, action_tx) emits Action::PauseConfirmed/Action::PauseTriple/Action::PauseResumed when Enter/Esc fires; pop callback removes itself from Compositor via PAUSE_DIALOG_ID constant.
  Create codelet/fspec-tui/src/components/hitl_dialog.rs hosting HitlDialog (priority Critical, uses Accent::Cyan via dialog_theme::render_dialog). Renders question as title, header as first body row, options as labelled rows (hotkey letter prefix), and an optional free-text input row when allow_text_input. Tab/Down advances selection (looping through options + free-text row when present), Enter submits, hotkey letters (a..z, case-insensitive) jump directly to the matching option AND submit, Esc dismisses without submit. HITL_DIALOG_ID constant for idempotent push and self-pop.
  Create codelet/fspec-tui/src/app/dispatch_rpc053.rs hosting impl App methods: handle_pause_chunk(session_id) — fired from dispatch_rpc045 SessionStateChange{Paused} arm, spawns parallel backend.get_pause_state + get_hitl_request and dispatches OpenPauseDialog OR OpenHitlDialog (HITL wins on tie); handle_open_pause_dialog(session, state) — idempotent compositor push; handle_open_hitl_dialog(session, request) — idempotent compositor push; handle_pause_confirmed/handle_pause_triple/handle_pause_resumed — fire-and-forget backend writes, pop dialog before await (best-effort UX); handle_hitl_submitted — fire-and-forget backend.send_hitl_response, pop dialog before await; handle_pause_cleared — pop any mounted PauseDialog/HitlDialog for the session.
  Wire into dispatch_rpc045::handle_stream_chunk_state_updates: branch SessionStateChange{state} on Running/Idle to dispatch Action::PauseCleared (so any mounted dialog is popped on resume), AND on Paused to dispatch a NEW Action::PauseChunkReceived(session_id) which routes through handle_pause_chunk (the parallel get_pause_state/get_hitl_request fetcher). Route both new actions in app/dispatch.rs (or via try_dispatch_rpc053 catch-all helper).
  Extend MockBackend in tests/common/mod.rs with: pause_state scripted Mutex<Option<PauseState>>, hitl_request scripted Mutex<Option<HitlRequest>>, per-call counters + last-call captures for pause_resume, pause_confirm, pause_triple, send_hitl_response, get_pause_state, get_hitl_request. Helpers: script_pause_state, script_hitl_request, set_get_pause_state_error, set_get_hitl_request_error, set_pause_confirm_error, set_pause_triple_error, set_pause_resume_error, set_send_hitl_response_error. Implement FspecBackend overrides on MockBackend (they currently use the trait defaults).
  Integration test file: codelet/fspec-tui/tests/pause_hitl_rpc053.rs with @step comments mapped 1:1 to feature scenarios. Use #[tokio::test(flavor='current_thread')] consistent with rpc049/rpc050 tests. Drive Action::ChunkReceived(s, SessionStateChange{Paused}) to fire the chunk path; assert compositor.contains(PAUSE_DIALOG_ID) or HITL_DIALOG_ID after the fetcher tasks drain (use tokio::task::yield_now or short sleep).
  Source-shape regression test (similar to source_shape_rpc050.rs) at codelet/fspec-tui/tests/source_shape_rpc053.rs asserting: dispatch_rpc053.rs exists; pub mod dispatch_rpc053 declared in app/mod.rs; pause_dialog.rs and hitl_dialog.rs exist; pub mod declarations in components/mod.rs; app/dispatch.rs stays under 300 logical lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the chunk dispatcher receives StreamChunk::SessionStateChange { state: Paused }, the App spawns backend.get_pause_state(session) AND backend.get_hitl_request(session) in parallel and dispatches Action::OpenPauseDialog when get_pause_state returns Some, Action::OpenHitlDialog when get_hitl_request returns Some, or nothing when both return None
  #   2. Action::OpenPauseDialog{session_id, state} pushes a fresh PauseDialog at Priority::Critical onto the Compositor, idempotent on dialog-id collision (no duplicate push when the dialog is already mounted)
  #   3. PauseDialog of kind PauseKind::Confirm renders a 2-button layout (Accept / Deny) with Tab/Left/Right cycling focus, Enter committing the focused choice via backend.pause_confirm(session, accept: bool)
  #   4. PauseDialog of kind PauseKind::Triple renders a 3-button layout (Approve / Approve Session / Deny) with Tab/Left/Right cycling focus, Enter committing the focused choice via backend.pause_triple(session, choice: ApprovalChoice)
  #   5. Esc on PauseDialog calls backend.pause_resume(session) and pops the dialog from the Compositor (treated as dismiss-without-choosing per the attachment)
  #   6. Action::OpenHitlDialog{session_id, request} pushes a fresh HitlDialog at Priority::Critical onto the Compositor, idempotent on dialog-id collision
  #   7. HitlDialog renders the request.question as title, request.header as body text, and one row per HitlOption labelled with a hotkey letter (a, b, c, ...) plus the option label and description
  #   8. When request.allow_text_input is true HitlDialog renders an additional free-text input row below the options that gains focus on Tab cycling and accepts text edit keystrokes
  #   9. Pressing the hotkey letter for an option directly submits HitlResponse{ id: request.id, value: option.label } via backend.send_hitl_response and pops the dialog; Enter on the highlighted option does the same
  #   10. Enter while the free-text row is focused submits HitlResponse{ id: request.id, value: <typed text> } via backend.send_hitl_response and pops the dialog
  #   11. Esc on HitlDialog pops the dialog but does NOT submit a response (the agent loop stays blocked on wait_for_hitl_response until the user submits OR a new HITL request supersedes)
  #   12. Errors from backend.get_pause_state, backend.get_hitl_request, backend.pause_confirm, backend.pause_triple, backend.pause_resume, and backend.send_hitl_response are silently logged via tracing — no scrollback notice and no panic
  #   13. PauseDialog and HitlDialog are mutually exclusive — at most one is mounted at a time per session. When SessionStateChange{Paused} arrives and BOTH backend.get_pause_state and backend.get_hitl_request return Some, the HitlDialog wins (the HITL handler in the agent loop sets hitl_request AND set_status(Paused); pause_state is only set by the tool-pause path)
  #   14. When SessionStateChange{Running} or SessionStateChange{Idle} arrives, any mounted PauseDialog or HitlDialog for that session is popped from the Compositor (agent loop has resumed or cleared the request server-side; UI should not strand a stale dialog)
  #
  # EXAMPLES:
  #   1. User types 'rm -rf /' which triggers a Confirm pause; the agent emits SessionStateChange{Paused}; the dispatcher polls and opens a 2-button PauseDialog showing the prompt; user presses Enter on 'Accept' → backend.pause_confirm(session, true) is called exactly once and the dialog is removed
  #   2. User types a sensitive command that triggers a Triple pause (Approve/Approve-Session/Deny); user presses Right twice and Enter to select 'Deny' → backend.pause_triple(session, ApprovalChoice::Deny) is called exactly once and the dialog is removed
  #   3. PauseDialog is open with focus on 'Accept'; user presses Esc → backend.pause_resume(session) is called exactly once and the dialog is removed; backend.pause_confirm is NEVER called
  #   4. Agent loop's request_user_input handler sets hitl_request and emits SessionStateChange{Paused}; the dispatcher polls and opens HitlDialog with 2 options 'Yes' and 'No'; user presses 'a' (hotkey for 'Yes') → backend.send_hitl_response(session, HitlResponse{id:req.id, value:'Yes'}) is called exactly once and the dialog is removed
  #   5. HitlDialog with allow_text_input=true is open; user presses Tab until the free-text row is focused, types 'maybe later', and presses Enter → backend.send_hitl_response(session, HitlResponse{id:req.id, value:'maybe later'}) is called exactly once and the dialog is removed
  #   6. HitlDialog is open; user presses Esc → the dialog is removed; backend.send_hitl_response is NEVER called (the agent loop stays blocked until either a new HITL request arrives OR a follow-up user submit fires)
  #   7. SessionStateChange{Paused} arrives but BOTH backend.get_pause_state and backend.get_hitl_request return None (race/stale chunk) → no dialog is pushed and no further backend call is made
  #   8. SessionStateChange{Paused} arrives twice in rapid succession (e.g. agent toggles status) → only ONE PauseDialog is mounted on the Compositor (idempotent push)
  #   9. PauseDialog is open and SessionStateChange{Running} arrives (agent loop received its response and resumed) → the dialog is popped automatically; subsequent Pause dialogs for the next pause cycle work cleanly
  #   10. Backend.pause_confirm returns Err('network blip') after the user clicks Accept → the dialog is still popped (best-effort), no scrollback notice is emitted, no panic, and the error is logged via tracing
  #   11. Backend.get_hitl_request returns Err('decode failed') after SessionStateChange{Paused} arrives → no HitlDialog is mounted; the App must not panic and no scrollback notice fires
  #   12. Two sessions are open (s-1, s-2); s-1 is focused; backend.push_status_change(s-2, Paused) fires AND the SessionStateChange chunk for s-2 routes a polled pause dialog targeting s-2 — the dialog is mounted on the Compositor but the user is free to stay on s-1; the dialog dispatches its actions targeting s-2 (the bound session id) regardless of currently-focused session
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView
    I want to see and answer pause (tool-approval) and HITL (request_user_input) prompts via dedicated dialogs that round-trip through the FspecBackend trait
    So that I can approve or deny tool calls and answer agent questions without leaving the TUI or stalling the agent loop

  # ─────────────────────────────────────────────────────────────────────
  # Chunk-driven trigger: SessionStateChange { Paused } opens the right dialog
  # ─────────────────────────────────────────────────────────────────────
  Scenario: SessionStateChange{Paused} with pause_state Some opens a Confirm PauseDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "Run rm -rf /?", tool_call_id: Some("tc-1") } for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then backend.get_pause_state is called at least once with s-1
    And backend.get_hitl_request is called at least once with s-1
    And the Compositor contains a layer with id PAUSE_DIALOG_ID
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: SessionStateChange{Paused} with hitl_request Some opens a HitlDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Continue?", header: "Apply changes?", options: [Yes, No], allow_text_input: false } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id HITL_DIALOG_ID
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: SessionStateChange{Paused} with both pause_state Some and hitl_request Some — HITL wins
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Q", header: "H", options: [Yes, No], allow_text_input: false } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id HITL_DIALOG_ID
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: SessionStateChange{Paused} with both returning None pushes no dialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: Repeated SessionStateChange{Paused} chunks push only one PauseDialog (idempotent)
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    And Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched again
    And all pending tasks have drained
    Then exactly one layer with id PAUSE_DIALOG_ID is mounted on the Compositor

  Scenario: SessionStateChange{Running} pops any mounted PauseDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And a PauseDialog is mounted on the Compositor for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Running }) is dispatched
    Then the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: SessionStateChange{Idle} pops any mounted HitlDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Idle }) is dispatched
    Then the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # PauseDialog (Confirm kind): 2-button layout
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Confirm PauseDialog default focus is on Accept
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Confirm PauseDialog is mounted on the Compositor for s-1
    Then the PauseDialog's focused button is "Accept"

  Scenario: Pressing Enter on Accept calls backend.pause_confirm with accept=true
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_confirm is called exactly once with (s-1, true)
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: Tab cycles focus from Accept to Deny on a Confirm PauseDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    When the user presses Tab on the PauseDialog
    Then the PauseDialog's focused button is "Deny"

  Scenario: Pressing Enter on Deny calls backend.pause_confirm with accept=false
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Deny"
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_confirm is called exactly once with (s-1, false)
    And backend.pause_confirm is NOT called with (s-1, true)
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: Pressing Esc on a Confirm PauseDialog calls backend.pause_resume
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Confirm PauseDialog is mounted on the Compositor for s-1
    When the user presses Esc on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_resume is called exactly once with s-1
    And backend.pause_confirm is NEVER called
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # PauseDialog (Triple kind): 3-button layout
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Triple PauseDialog default focus is on Approve
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Triple PauseDialog is mounted on the Compositor for s-1
    Then the PauseDialog's focused button is "Approve"

  Scenario: Right arrow advances focus through Approve → Approve Session → Deny
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Approve"
    When the user presses Right arrow on the PauseDialog
    Then the PauseDialog's focused button is "Approve Session"
    When the user presses Right arrow on the PauseDialog
    Then the PauseDialog's focused button is "Deny"

  Scenario: Pressing Enter on Deny on a Triple PauseDialog calls backend.pause_triple with ApprovalChoice::Deny
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Deny"
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::Deny)
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID

  Scenario: Pressing Enter on Approve Session on a Triple PauseDialog calls backend.pause_triple with ApprovalChoice::ApproveSession
    Given an App with a MockBackend
    And session s-1 is the current session
    And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Approve Session"
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::ApproveSession)

  # ─────────────────────────────────────────────────────────────────────
  # HitlDialog: option selection (no free-text)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: HitlDialog renders one row per option with hotkey letters a, b, c
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No", "Maybe"] and allow_text_input=false
    Then the HitlDialog's rows include hotkey letters "a", "b", "c"
    And the HitlDialog's option labels include "Yes", "No", "Maybe"

  Scenario: Pressing the hotkey letter "a" submits the matching option
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    When the user presses the character "a" on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "Yes" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: Enter on the highlighted option submits that option
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    And the HitlDialog's selected row is index 1 (option "No")
    When the user presses Enter on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "No" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: Esc on a HitlDialog dismisses without submitting
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1
    When the user presses Esc on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is NEVER called
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # HitlDialog: free-text input
  # ─────────────────────────────────────────────────────────────────────
  Scenario: HitlDialog with allow_text_input=true renders a free-text input row below the options
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    Then the HitlDialog contains a free-text input row
    And the free-text input row's value is empty

  Scenario: Tab cycles past the options into the free-text input row
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    And the HitlDialog's selected row is the first option (index 0)
    When the user presses Tab on the HitlDialog three times
    Then the HitlDialog's selected row is the free-text input row

  Scenario: Typing into the free-text row updates its value
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with allow_text_input=true
    And the HitlDialog's selected row is the free-text input row
    When the user types "maybe later" into the free-text row
    Then the free-text row's value equals "maybe later"

  Scenario: Enter on the free-text row submits HitlResponse with the typed text
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and allow_text_input=true
    And the HitlDialog's free-text row contains "maybe later"
    And the HitlDialog's selected row is the free-text input row
    When the user presses Enter on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "maybe later" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # Error tolerance
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Backend.pause_confirm error after Accept still pops the dialog and emits no scrollback notice
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's pause_confirm is scripted to return Err("network blip")
    And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then the App must not panic
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    And the session s-1 scrollback contains no chunks mentioning "pause_confirm"
    And the session s-1 scrollback contains no chunks mentioning "network blip"

  Scenario: Backend.get_hitl_request error during the chunk-driven probe pushes no dialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Err("decode failed") for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the App must not panic
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    And the session s-1 scrollback contains no chunks mentioning "get_hitl_request"

  Scenario: Backend.send_hitl_response error after Enter still pops the dialog and emits no scrollback notice
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes"]
    When the user presses the character "a" on the HitlDialog
    And all pending tasks have drained
    Then the App must not panic
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    And the session s-1 scrollback contains no chunks mentioning "send_hitl_response"

  # ─────────────────────────────────────────────────────────────────────
  # Multi-session: dialog binds to its originating session
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A PauseDialog opened for a non-focused session still routes its actions to that session
    Given an App with a MockBackend
    And open sessions are [s-1, s-2] and s-1 is focused
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-2
    And the MockBackend's get_hitl_request is scripted to return None for s-2
    When Action::ChunkReceived(s-2, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id PAUSE_DIALOG_ID
    And the focused session is still s-1
    When the user presses Enter on the PauseDialog
    And all pending tasks have drained
    Then backend.pause_confirm is called exactly once with (s-2, true)
    And backend.pause_confirm is NOT called with (s-1, true)

  # ─────────────────────────────────────────────────────────────────────
  # Source shape
  # ─────────────────────────────────────────────────────────────────────
  Scenario: codelet/fspec-tui/src/app/dispatch_rpc053.rs hosts the new pause/HITL helpers
    Given the file codelet/fspec-tui/src/app/dispatch_rpc053.rs exists
    When the file is compiled as part of codelet-fspec-tui
    Then it must declare impl App methods named handle_pause_chunk, handle_open_pause_dialog, handle_open_hitl_dialog, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_rpc053
    And codelet/fspec-tui/src/components/mod.rs must declare pub mod pause_dialog and pub mod hitl_dialog
    And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
