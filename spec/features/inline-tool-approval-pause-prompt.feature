@done
@tui
@input
@agent-view
@pause-integration
@rust
@ts-parity
@security
@RPC-406
Feature: Inline Tool-Approval Pause Prompt
  """
  Replaces the RPC-053 centered PauseDialog modal with the TS-parity inline
  prompt that swaps into the input area (TS: InputTransition.tsx:467-533
  rendering, AgentView.tsx:4521-4607 keys, 1310-1331 selection reset).
  Fixes the Esc-resumes security bug: Esc now DENIES (pause_triple(Deny) on
  Triple, pause_confirm(false) on Confirm) — Action::PauseResumed is
  unreachable from any pause-prompt key path.

  Architecture:
  - Store: per-session slot in AgentViewStore — pause_state_by_session:
  HashMap<SessionId, PauseState> + triple_pause_selection_by_session:
  HashMap<SessionId, usize>; accessors in store/agent_view/pause_state.rs
  (isolation_state.rs pattern). set_pause_state resets the selection when
  the kind changes; clear_pause_state removes the slot and the selection
  entry (an unset selection reads as 0).
  - Dispatch: app/dispatch_pause_hitl.rs — handle_pause_chunk's pause arm
  dispatches Action::PauseStateFetched{session_id, state} (replacing the
  deleted Action::OpenPauseDialog); handle_pause_cleared clears the slot
  (still pops the HITL dialog); handle_pause_confirmed/handle_pause_triple
  clear the slot; Action::PausePromptNav{session_id, delta} cycles the
  selection with wraparound; Action::PausePromptEnter{session_id} reads
  the authoritative selection from the store and maps 0/1/2 onto
  ApprovalChoice::{Approve, ApproveSession, Deny}. HITL still wins on tie.
  - Rendering: views/agent/pause_prompt.rs (prompt_height +
  render_pause_prompt) painted by views/agent/input_area.rs
  (paint_input_area, extracted from views/agent.rs to hold the 300-LoC
  ceiling) which consults the FOCUSED session's pause slot before
  paint_input_or_spinner and caches last_pause on the view. The prompt
  header wraps at the padded input-area width (char-slice wrap, no
  clipping); input-area height = prompt_height(state, width) = wrapped
  header rows + options row (Triple) or wrapped header rows + optional
  details line + Y/N row (Confirm) via the RPC-405 auto-grow seam.
  is_cursor_visible is false while paused (RPC-404 cursor containment).
  - Keys: views/agent/pause_keys.rs — handle_pause_prompt_key consulted
  right after the KeyEventKind::Press filter in views/agent/dispatch.rs;
  Ctrl+C still emits Interrupt; every other key is swallowed so nothing
  reaches the MultiLineInput. The TextArea state is never mutated while
  paused (tui-textarea separates state from rendering — render-only swap),
  so the draft text AND cursor survive the pause round-trip untouched.
  - Lockstep: components/pause_dialog.rs deleted (mod decl + lib.rs
  re-export removed); tests/pause_hitl_rpc053.rs pause-modal scenarios and
  spec/features/pause-and-hitl-dialogs.feature rewritten against the slot;
  HITL modal behavior untouched. Wire PauseKind::Continue stays collapsed
  to Confirm; middleware.rs response mapping unchanged; pause_resume RPC
  kept for other callers.
  """

  Background: User Story
    As a user of the Rust ratatui AgentView
    I want to see and answer tool-approval pause prompts inline in the input area exactly like the TypeScript Ink TUI, with Esc denying access
    So that I can approve or deny sensitive tool calls without a modal stealing focus, and dismissing the prompt can never silently grant access to sensitive files like .env

  # ─────────────────────────────────────────────────────────────────────
  # Rendering — TS InputTransition.tsx:467-533 parity
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Triple pause prompt renders inline with options, colors, and hint
    Given the agent view is rendered with a focused session s-1
    And session s-1 has a Triple pause slot with prompt "Read: Access to .env requires approval" and details ".env"
    When the agent view renders a frame
    Then the input area's first row shows "⏸ Read: Access to .env requires approval (.env)"
    And the "⏸ Read" prefix is cyan and the "(.env)" details are dim
    And the input area's second row shows "[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)"
    And "[Allow Once]" is green, "[Allow Session]" is blue, and "[Deny]" is red
    And "[Allow Once]" is rendered inverse because the selection defaults to 0
    And the navigation hint is dim

  Scenario: Confirm pause prompt renders yellow header, dim details line, and Y/N row
    Given the agent view is rendered with a focused session s-1
    And session s-1 has a Confirm pause slot with prompt "Bash: Run rm -rf build?" and details "rm -rf build"
    When the agent view renders a frame
    Then the input area's first row shows "⏸ Bash: Run rm -rf build?" in yellow
    And the input area's second row shows the dim details line "rm -rf build"
    And the input area's third row shows "[Y] Approve [N] Deny (Esc to cancel)"
    And "[Y] Approve" is green, "[N] Deny" is red, and "(Esc to cancel)" is dim

  Scenario: Confirm pause prompt without details omits the details line
    Given the agent view is rendered with a focused session s-1
    And session s-1 has a Confirm pause slot with prompt "Bash: Run build?" and no details
    When the agent view renders a frame
    Then the input area is 2 rows tall
    And the input area's second row shows "[Y] Approve [N] Deny (Esc to cancel)"

  Scenario: Pause prompt replaces the draft and paints no hardware cursor
    Given the agent view is rendered with a focused session s-1
    And the input buffer contains the draft "deploy the fix"
    And session s-1 has a Triple pause slot with prompt "Read: approval" and no details
    When the agent view renders a frame
    Then the rendered input area does not contain the text "deploy the fix"
    And the rendered input area contains "⏸ Read"
    And the cursor is not visible while the pause prompt is showing

  Scenario: Prompt renders only when the paused session is focused
    Given the agent view is rendered with open sessions s-1 and s-2 and s-1 focused
    And session s-2 has a Triple pause slot with prompt "Read: approval" and no details
    When the agent view renders a frame
    Then the input area shows the normal input, not a pause prompt
    When focus switches to session s-2 and the agent view renders a frame
    Then the input area's first row shows "⏸ Read: approval"

  Scenario: Long triple prompt header wraps instead of clipping
    Given the agent view is rendered on a narrow terminal with a focused session s-1
    And session s-1 has a Triple pause slot whose prompt and details exceed the input-area width
    When the agent view renders a frame
    Then the prompt header wraps across two or more input-area rows
    And no header characters are lost to clipping
    And the options row "[Allow Once] [Allow Session] [Deny]" and the hint still render below the wrapped header

  # ─────────────────────────────────────────────────────────────────────
  # Keys — TS AgentView.tsx:4521-4607 parity, Esc DENIES
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Left and Right cycle the triple selection with wraparound
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Triple pause slot
    When the user presses Right twice
    Then the triple pause selection for s-1 is 2
    When the user presses Right once more
    Then the triple pause selection for s-1 is 0
    When the user presses Left
    Then the triple pause selection for s-1 is 2

  Scenario: Enter sends the selected approval choice and resets the selection
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Triple pause slot
    When the user presses Right then Enter
    Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::ApproveSession)
    And the pause slot for s-1 is cleared
    And the triple pause selection for s-1 is reset to 0

  Scenario: Esc on a triple prompt denies instead of resuming
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Triple pause slot
    When the user presses Esc
    Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::Deny)
    And backend.pause_resume is NEVER called
    And the pause slot for s-1 is cleared

  Scenario: Y approves a confirm prompt
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Confirm pause slot
    When the user presses the character "y"
    Then backend.pause_confirm is called exactly once with (s-1, true)

  Scenario: N denies a confirm prompt
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Confirm pause slot
    When the user presses the character "n"
    Then backend.pause_confirm is called exactly once with (s-1, false)

  Scenario: Esc on a confirm prompt denies
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Confirm pause slot
    When the user presses Esc
    Then backend.pause_confirm is called exactly once with (s-1, false)
    And backend.pause_resume is NEVER called

  Scenario: Printable keys are swallowed while the pause prompt is showing
    Given an App with a MockBackend and a focused session s-1
    And the input buffer contains the draft "hello"
    And session s-1 has a Triple pause slot
    When the user types the characters "abc"
    Then the input buffer still contains exactly "hello"

  # ─────────────────────────────────────────────────────────────────────
  # Dispatch and store — chunk-driven slot lifecycle
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Paused chunk with pause state stores a per-session slot without mounting a modal
    Given an App with a MockBackend and a focused session s-1
    And the MockBackend's get_pause_state is scripted to return Some Triple PauseState for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore pause slot for s-1 holds the fetched PauseState
    And no compositor layer with id "pause-dialog" is mounted

  Scenario: Stale Paused chunk with no pause state sets no slot
    Given an App with a MockBackend and a focused session s-1
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore pause slot for s-1 is empty
    And no compositor layer with id "pause-dialog" is mounted

  Scenario: HITL wins on tie and no pause slot is set
    Given an App with a MockBackend and a focused session s-1
    And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id HITL_DIALOG_ID
    And the AgentViewStore pause slot for s-1 is empty

  Scenario: Running chunk clears the pause slot and resets the selection
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has a Triple pause slot with selection 2
    When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    Then the AgentViewStore pause slot for s-1 is empty
    And the triple pause selection for s-1 is reset to 0

  Scenario: Prompt keys route to the paused session in a multi-session app
    Given an App with a MockBackend and open sessions s-1 and s-2 with s-2 focused
    And session s-2 has a Triple pause slot
    When the user presses Enter
    Then backend.pause_triple is called exactly once with (s-2, ApprovalChoice::Approve)
    And backend.pause_triple is NOT called for s-1

  # ─────────────────────────────────────────────────────────────────────
  # Draft preservation — MultiLineInput parity (RPC-404/405 geometry)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Draft text and cursor survive the pause round-trip
    Given the agent view is rendered with a focused session s-1
    And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    And session s-1 has a Triple pause slot
    When the agent view renders a frame
    And the pause slot for s-1 is cleared as if the user denied
    And the agent view renders another frame
    Then the input buffer contains exactly "deploy the fix"
    And the cursor is at the end of "deploy the fix"
    And the rendered input area shows the draft text again

  # ─────────────────────────────────────────────────────────────────────
  # Source shape — modal removal + Esc-deny lock
  # ─────────────────────────────────────────────────────────────────────
  Scenario: The pause modal is deleted and resume is unreachable from the prompt
    Given the codelet-fspec-tui crate sources
    Then the file codelet/fspec-tui/src/components/pause_dialog.rs does not exist
    And components/mod.rs declares no OpenPauseDialog action variant
    And views/agent/pause_keys.rs and app/dispatch_pause_hitl.rs never construct Action::PauseResumed from a pause-prompt key path
    And the files pause_prompt.rs, pause_keys.rs, input_area.rs, and store/agent_view/pause_state.rs each stay under 300 lines
