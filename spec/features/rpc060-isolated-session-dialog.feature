@done
@dialog
@slash-command
@rust
@tui
@agent-view
@rpc
@RPC-060
Feature: Isolated session creation + AgentView /new isolated flow

  """
  Phase 7.7 of the RPC-030 roadmap. Wires backend.create_isolated_session(role) (already implemented in RPC-042) through to the Rust ratatui frontend via a new CreateSessionDialog (Priority::Foreground) and the existing SlashCommandAction::Isolation entry. Mirrors src/components/CreateSessionDialog.tsx (TUI-090) — three flat options Yes / Yes - Isolated / Cancel with Left/Right cyclic navigation. Trait + backend + service plumbing already exist (RPC-036/037/042); this card adds ONLY the UI dialog + dispatch wiring. Out of scope: SessionFooter isolation badge (RPC-045), worktree merge (RPC-057), auto-creating isolated sessions per work-unit (future).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle::create_isolated_session(role) and FspecService::create_isolated_session already exist (RPC-042/RPC-037) and return IsolatedSessionInfo { session_id, worktree_path, base_commit }.
  #   2. Both EmbeddedFspecBackend and WebSocketFspecBackend already forward create_isolated_session to the tarpc client (RPC-037). No new RPC plumbing is needed.
  #   3. A new CreateSessionDialog (Priority::Foreground) component renders three options: 'Yes', 'Yes - Isolated', 'Cancel' — mirroring src/components/CreateSessionDialog.tsx exactly (TUI-090).
  #   4. The dialog uses cyan accent (matching TS dialog borderColor='cyan') and renders title context-aware: 'Work on <id>?' when a work_unit is bound or 'Start New Agent?' when unattached.
  #   5. Left/Right arrow keys cycle through 3 options with wrap-around (matching TS dialog). Enter confirms selected option; Esc cancels.
  #   6. Dialog submit emits Action::CreateSessionSubmitted { isolated: bool }; cancel emits Action::CreateSessionCancelled. App::dispatch routes submit through handle_create_session_submitted which spawns either backend.create_session(None) or backend.create_isolated_session(None) and dispatches Action::SessionCreated on success.
  #   7. SlashCommandAction::Isolation (already in registry) opens the CreateSessionDialog with 'Yes - Isolated' pre-selected so /isolation is a one-keystroke shortcut for new isolated session.
  #   8. On create_isolated_session error, App::dispatch surfaces a `[error] create isolated session: <e>` notice into the current scrollback (or no-op when no session). The dialog pops regardless before the await completes (matches RPC-053 pause-dialog UX).
  #   9. The SessionFooter isolation badge is already wired (RPC-045) and lights up automatically when StreamChunk::IsolationStateChange arrives — this card does NOT modify SessionFooter.
  #   10. Integration test in codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs validates: dialog rendering snapshot, three-option cyclic nav, Enter on 'Yes - Isolated' emits CreateSessionSubmitted{isolated:true}, dispatch wires through to backend.create_isolated_session, error path emits notice.
  #
  # EXAMPLES:
  #   1. User opens /isolation slash command → CreateSessionDialog appears with 'Yes - Isolated' pre-selected → User presses Enter → Action::CreateSessionSubmitted{isolated:true} fires → backend.create_isolated_session(None) is called exactly once → Action::SessionCreated lands → new SessionContext appended to AgentViewStore → SessionFooter renders isolation badge after IsolationStateChange chunk arrives.
  #   2. User opens CreateSessionDialog → presses Left arrow → 'Cancel' highlights → presses Enter → dialog closes, no backend call fires, no session is created.
  #   3. User opens CreateSessionDialog → presses Right arrow once → 'Yes - Isolated' highlights → presses Enter → dialog closes, backend.create_isolated_session is called once with role=None, the new isolated session activates, and the session footer eventually shows the isolation badge.
  #   4. User opens CreateSessionDialog → presses Enter on default 'Yes' option → dialog closes, backend.create_session(None) is called (non-isolated path preserved).
  #   5. User opens CreateSessionDialog → presses Esc → dialog closes, no backend call fires.
  #   6. MockBackend's create_isolated_session returns Err('not a git repository') → after user picks 'Yes - Isolated' the dialog closes, then a `[error] create isolated session: not a git repository` notice appears in the focused session's scrollback.
  #
  # ========================================

  Background: User Story
    As a fspec TUI user with an open AgentView in a git repo
    I want to create a new isolated worktree-backed session via the CreateSessionDialog or /isolation slash command from the Rust ratatui frontend
    So that I can run experiments in an isolated git worktree without affecting my main checkout, with full parity with the TS Ink CreateSessionDialog

  # ---- Dialog component scenarios ----------------------------------

  Scenario: CreateSessionDialog defaults selection to "Yes" when opened without a preselection
    Given the CreateSessionDialog is constructed with preselect=None and no work_unit binding
    Then the selected option is "Yes"
    And the dialog title is "Start New Agent?"
    And the dialog accent is cyan

  Scenario: CreateSessionDialog renders work-unit-aware title when a work_unit is bound
    Given the CreateSessionDialog is constructed with preselect=None and work_unit_id "AUTH-001"
    Then the dialog title is "Work on AUTH-001?"

  Scenario: CreateSessionDialog can be preselected to "Yes - Isolated" via /isolation shortcut
    Given the CreateSessionDialog is constructed with preselect=Some(Isolated)
    Then the selected option is "Yes - Isolated"

  Scenario: CreateSessionDialog Right arrow cycles forward with wrap-around
    Given a CreateSessionDialog with selection "Yes"
    When the user presses Right arrow
    Then the selection becomes "Yes - Isolated"
    When the user presses Right arrow
    Then the selection becomes "Cancel"
    When the user presses Right arrow
    Then the selection becomes "Yes"

  Scenario: CreateSessionDialog Left arrow cycles backward with wrap-around
    Given a CreateSessionDialog with selection "Yes"
    When the user presses Left arrow
    Then the selection becomes "Cancel"
    When the user presses Left arrow
    Then the selection becomes "Yes - Isolated"

  Scenario: CreateSessionDialog Enter on "Yes" emits CreateSessionSubmitted{isolated:false}
    Given a CreateSessionDialog with selection "Yes"
    When the user presses Enter
    Then Action::CreateSessionSubmitted { isolated: false } is emitted
    And the dialog requests removal from the compositor

  Scenario: CreateSessionDialog Enter on "Yes - Isolated" emits CreateSessionSubmitted{isolated:true}
    Given a CreateSessionDialog with selection "Yes - Isolated"
    When the user presses Enter
    Then Action::CreateSessionSubmitted { isolated: true } is emitted
    And the dialog requests removal from the compositor

  Scenario: CreateSessionDialog Enter on "Cancel" emits CreateSessionCancelled
    Given a CreateSessionDialog with selection "Cancel"
    When the user presses Enter
    Then Action::CreateSessionCancelled is emitted
    And the dialog requests removal from the compositor

  Scenario: CreateSessionDialog Esc emits CreateSessionCancelled
    Given a CreateSessionDialog with any selection
    When the user presses Esc
    Then Action::CreateSessionCancelled is emitted
    And the dialog requests removal from the compositor

  # ---- Slash command + dispatch scenarios --------------------------

  Scenario: /isolation slash command opens the CreateSessionDialog with "Yes - Isolated" preselected
    Given an App with open session s-1
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then a CreateSessionDialog is pushed onto the compositor at Priority::Foreground with preselect=Some(Isolated)
    And no backend method is called

  Scenario: CreateSessionSubmitted{isolated:true} spawns backend.create_isolated_session
    Given an App with open session s-1 wired to a MockBackend whose create_isolated_session returns Ok(IsolatedSessionInfo { session_id: SessionId::new("iso-1"), worktree_path: "/tmp/.fspec/worktrees/iso-1", base_commit: "abc123" })
    When Action::CreateSessionSubmitted { isolated: true } is dispatched
    Then within 1 second backend.create_isolated_session is called exactly once with role=None
    And within 1 second Action::SessionCreated for SessionId "iso-1" is observed on the action bus
    And backend.create_session is NOT called

  Scenario: CreateSessionSubmitted{isolated:false} spawns backend.create_session
    Given an App with open session s-1 wired to a MockBackend whose create_session returns Ok(SessionId::new("plain-1"))
    When Action::CreateSessionSubmitted { isolated: false } is dispatched
    Then within 1 second backend.create_session is called exactly once with role=None
    And within 1 second Action::SessionCreated for SessionId "plain-1" is observed on the action bus
    And backend.create_isolated_session is NOT called

  Scenario: CreateSessionCancelled is a silent no-op
    Given an App with open session s-1
    When Action::CreateSessionCancelled is dispatched
    Then no backend method is called

  Scenario: create_isolated_session error emits an error notice into the focused session scrollback
    Given an App with open session s-1 wired to a MockBackend whose create_isolated_session returns Err("not a git repository")
    When Action::CreateSessionSubmitted { isolated: true } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] create isolated session: not a git repository" is observed on the action bus

  Scenario: create_isolated_session error with no open session is a silent no-op
    Given an App with NO open AgentView session wired to a MockBackend whose create_isolated_session returns Err("e")
    When Action::CreateSessionSubmitted { isolated: true } is dispatched
    Then within 1 second backend.create_isolated_session is called exactly once
    And no Action::EmitSessionNotice is observed on the action bus
