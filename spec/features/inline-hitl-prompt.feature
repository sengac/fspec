@done
@ts-parity
@rust
@pause-integration
@agent-view
@input
@tui
@RPC-411
Feature: Inline HITL prompt parity: replace Critical modal with TS-parity inline composer prompt
  """
  Store: per-session HITL slot in store/agent_view/hitl_state.rs (mirrors pause_state.rs) — hitl_prompt_by_session: HashMap<SessionId, HitlPromptState> where HitlPromptState holds the wire HitlRequest + the useHitlInput machine state {question_index, selected_option, answers: Vec<HitlAnswer>, other_active, show_empty_hint}. Faithful port of src/tui/hooks/useHitlInput.ts:134-262: wrap-around selection over options.len()+1 (virtual Other...), advance_or_submit accumulation, reset on clear. handle_pause_chunk's HITL arm dispatches Action::HitlPromptFetched{session_id, request} (replaces Action::OpenHitlDialog); Running/Idle (handle_pause_cleared) clears the slot.
  Rendering: views/agent/hitl_prompt.rs (mirrors pause_prompt.rs) painted from views/agent/input_area.rs — HITL slot consulted BEFORE the pause slot (TS InputTransition.tsx:385-388 priority) and before paint_input_or_spinner. Options mode: prompt_height = wrapped header rows + options + Other... + footer row; freeform/Other mode: header row (+ optional yellow hint row) + the SHARED composer MultiLineInput rendered via render_with_prompt with placeholder "Type your answer..." (draft state untouched — same TextArea). Exact TS colors/glyphs per InputTransition.tsx:393-463. AgentView caches last_hitl (session + mode) at render time like last_pause; hardware cursor visible ONLY in freeform/Other mode (inside the shared input).
  Keys: views/agent/hitl_keys.rs (mirrors pause_keys.rs), consulted in views/agent/dispatch.rs BEFORE handle_pause_prompt_key. Options mode: ↑/↓ → Action::HitlPromptNav{delta}; Enter → Action::HitlPromptEnter (App reducer reads authoritative slot state: Other-selected enters Other mode, else capture+advance-or-submit); Esc → Action::HitlCancelled; every other key consumed (paste too). Freeform/Other mode: Esc → Action::HitlOtherExit (Other) or HitlCancelled (plain freeform); plain Enter → capture the shared input value (empty → Action::HitlEmptySubmit sets hint); ALL other keys + paste fall through to the shared MultiLineInput (typing dispatches hint-clear when show_empty_hint). Ctrl+C still emits Interrupt. App reducers in app/dispatch_pause_hitl.rs: handle_hitl_submitted keeps fire-and-forget backend.send_hitl_response; cancel path sends {cancelled:true, answers:[]} then clears the slot — no path clears the slot without sending.
  Lockstep deletions/rewrites: components/hitl_dialog.rs DELETED (mod decl + lib.rs re-export of HitlDialog/HITL_DIALOG_ID removed); Action::OpenHitlDialog replaced by Action::HitlPromptFetched; source-shape test locks no construction site remains (mirror RPC-406 Action::PauseResumed lock). tests/pause_hitl_rpc053.rs rewritten: chunk-trigger scenarios assert the HITL store slot (not compositor layers) and Esc now SENDS {cancelled:true}; spec/features/pause-and-hitl-dialogs.feature HITL-modal scenarios rewritten against the slot. tests/agent_input_paste_routing_rpc403.rs HITL case rewritten: options mode paste consumed/ignored; freeform/Other mode paste inserts into the shared input. Wire shapes are RPC-410's HitlRequest{questions}/HitlResponse{cancelled,answers} — no rpc-types changes. Freeform Enter capture: hitl_keys reads the shared input value, clears it, and emits Action::HitlAnswerCaptured{session_id, text} so the reducer stays store-authoritative. All new files < 300 lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The HITL prompt renders INLINE in the composer input area (no modal, no border) while the focused session is Paused with a pending HitlRequest; it takes precedence over a simultaneous tool-pause slot, and a Running/Idle state change clears the stale prompt
  #   2. Options-question layout is exact TS parity: magenta ⏸, magenta [n/m] only when total questions > 1, bold header + literal ": " + question; ● selected / ○ unselected radios with radio+label green when selected and white otherwise; dim " — description"; virtual dim-italic "Other..." always appended; dim footer " (↑/↓ Navigate | Enter Select | Esc Cancel)"; ↑/↓ wrap across options + Other; no hotkeys, no Tab cycle, no scroll-select, no always-visible free-text row
  #   3. Questions advance one at a time with answers accumulated (advance-or-submit); on the last question all answers are submitted together as HitlResponse{cancelled:false, answers:[one per question]} via backend.send_hitl_response; Enter on an option captures {id, selected:[label]}, Enter in freeform/Other captures {id, selected:[], other:<text>}; no backward navigation
  #   4. Freeform mode (question without options, or Other... selected on an options question) reuses the SHARED composer MultiLineInput with placeholder "Type your answer...": the pre-existing composer draft is the initial answer text and is cleared when the answer is captured; Shift/Alt+Enter still inserts a newline; header line appends dim " (Enter Submit | Esc Back to options)" in Other mode and " (Enter Submit | Esc Cancel)" in plain freeform mode
  #   5. Empty/whitespace Enter in Other or freeform mode is rejected and shows the yellow hint "  ⚠ Please type a response or press Esc to go back"; any typing clears the hint
  #   6. Esc outside Other mode cancels the WHOLE request: backend.send_hitl_response(session, {cancelled:true, answers:[]}) is sent and the HITL slot clears; Esc in Other mode is local only (back to options, clears hint AND the shared input value, sends NOTHING); no code path may dismiss the HITL UI without submitting or cancelling; Esc-cancel does not clear a remaining composer draft, and an options-only round-trip preserves the draft text and cursor
  #   7. components/hitl_dialog.rs and HITL_DIALOG_ID are DELETED with a source-shape lock (no construction site for the old modal remains); the HITL prompt lives in a per-session store slot (store/agent_view/hitl_state.rs mirroring pause_state.rs) written by handle_pause_chunk's HITL arm; paste in options mode is consumed/ignored, paste in freeform/Other mode goes into the shared input
  #
  # EXAMPLES:
  #   1. A 3-question request shows "⏸ [1/3] Header: Question?" with magenta ⏸ and [1/3]; a single-question request shows no [n/m] progress marker
  #   2. Options list "Deploy"/"Skip" renders " ● Deploy" green with dim " — description", " ○ Skip" white, then dim-italic " ○ Other...", then dim footer " (↑/↓ Navigate | Enter Select | Esc Cancel)"; pressing ↓ three times wraps the selection back to Deploy
  #   3. On a 2-question request the user selects "Yes" on question 1 (advances to [2/2], nothing sent yet), then selects "No" on question 2 → ONE send_hitl_response with {cancelled:false, answers:[{q1,[Yes]},{q2,[No]}]} and the prompt clears
  #   4. User navigates to Other... and presses Enter → freeform mode with placeholder "Type your answer..." and dim header hint " (Enter Submit | Esc Back to options)"; empty Enter shows the yellow ⚠ hint; typing "ship it" clears the hint; Enter submits {id, selected:[], other:"ship it"} and the shared input is cleared
  #   5. With draft "deploy the fix" in the composer, a freeform HITL question shows the draft as the initial answer; Enter submits it as other and clears the input; alternatively, on an options-only question the user answers with Enter on an option and afterwards the draft "deploy the fix" and cursor are exactly as before
  #   6. User presses Esc on an options question → send_hitl_response(s-1, {cancelled:true, answers:[]}) exactly once, prompt clears; user presses Esc in Other mode → NOTHING sent, back to options list with hint and input cleared
  #   7. Paused chunk with both pause state and HITL request scripted → the inline HITL prompt shows (HITL wins) and no pause prompt renders; a later Running chunk clears the HITL prompt without any user action
  #   8. Typing letters (a/b/c), Tab, or scrolling the wheel on an options question never selects or submits anything; pasting text on an options question is swallowed and the composer draft is untouched; pasting in Other mode inserts into the shared input
  #   9. The source tree contains no hitl_dialog.rs, no HITL_DIALOG_ID, and no Action::OpenHitlDialog construction site — mirroring the RPC-406 pause-modal deletion lock
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView
    I want to answer request_user_input (HITL) prompts inline in the composer slot exactly like the TypeScript Ink TUI — multi-question [n/m] flow, radio options with a virtual Other... freeform entry reusing the shared composer input, and Esc that cancels the whole request
    So that I can answer agent questions without a modal stealing focus, and dismissing the prompt can never strand the backend in a Paused state

  # ─────────────────────────────────────────────────────────────────────
  # Rendering — TS InputTransition.tsx:385-463 parity
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Options question renders inline with radios, colors, Other entry, and footer
    Given the agent view is rendered with a focused session s-1
    And session s-1 has an inline HITL prompt with one question "Header" / "Question?" and options "Deploy — push to prod" and "Skip — do nothing"
    When the agent view renders a frame
    Then the input area's header row shows "⏸ Header: Question?" with a magenta "⏸ " glyph, a bold header, and no "[1/1]" progress marker
    And an option row shows " ● Deploy" with the radio and label green and " — push to prod" dim
    And an option row shows " ○ Skip" with the radio and label white
    And the row below the real options shows " ○ Other..." with the label dim and italic
    And the footer row shows " (↑/↓ Navigate | Enter Select | Esc Cancel)" in dim
    And no modal layer is mounted on the compositor

  Scenario: Multi-question request shows a magenta question progress marker
    Given the agent view is rendered with a focused session s-1
    And session s-1 has an inline HITL prompt with three options questions
    When the agent view renders a frame
    Then the input area's header row shows "⏸ [1/3] Header-1: Question-1?"
    And the "[1/3] " progress marker is magenta

  Scenario: Up and Down wrap the selection across options and the virtual Other entry
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt with one question with options "Deploy" and "Skip"
    When the user presses Down three times
    Then the HITL selection for s-1 is back on option 0
    When the user presses Up
    Then the HITL selection for s-1 is on the virtual "Other..." entry at index 2

  # ─────────────────────────────────────────────────────────────────────
  # Multi-question accumulation — useHitlInput.ts:134-151 parity
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Answers accumulate across questions and submit together after the last question
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt with two options questions "q-1" with options "Yes" and "No" and "q-2" with options "Yes" and "No"
    When the user presses Enter on question 1 with "Yes" selected
    Then backend.send_hitl_response has not been called
    And the prompt advances to question 2 with the selection reset to 0
    When the user presses Down then Enter on question 2
    Then backend.send_hitl_response is called exactly once with cancelled false and answers [{id q-1, selected [Yes]}, {id q-2, selected [No]}]
    And the HITL prompt for s-1 is cleared

  # ─────────────────────────────────────────────────────────────────────
  # Other... freeform mode — TOOL-018 parity
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Enter on Other activates freeform mode with the shared input, empty-submit hint, and submit
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt with one question "q-1" with options "Yes" and "No"
    When the user navigates the selection to "Other..." and presses Enter
    Then the prompt is in Other mode and the header hint shows " (Enter Submit | Esc Back to options)" in dim
    And the shared composer input renders with the placeholder "Type your answer..."
    When the user presses Enter with the shared input empty
    Then the yellow hint "  ⚠ Please type a response or press Esc to go back" is shown
    And backend.send_hitl_response has not been called
    When the user types "ship it"
    Then the empty-submit hint is cleared
    When the user presses Enter
    Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "ship it"}
    And the shared composer input is empty
    And the HITL prompt for s-1 is cleared

  Scenario: Esc in Other mode returns to the options list without sending anything
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt in Other mode with the empty-submit hint showing and "half an answer" in the shared input
    When the user presses Esc
    Then backend.send_hitl_response is NEVER called
    And the prompt is back in options mode with the empty-submit hint cleared
    And the shared composer input is empty
    And the HITL prompt for s-1 is still active

  # ─────────────────────────────────────────────────────────────────────
  # Freeform question — shared composer draft semantics
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Freeform question consumes the composer draft as the answer
    Given an App with a MockBackend and a focused session s-1
    And the composer input contains the draft "deploy the fix"
    And session s-1 has an inline HITL prompt with one freeform question "q-1" without options
    When the agent view renders a frame
    Then the header hint shows " (Enter Submit | Esc Cancel)" in dim
    When the user presses Enter
    Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "deploy the fix"}
    And the shared composer input is empty

  Scenario: Composer draft and cursor survive an options-only HITL round-trip
    Given the agent view is rendered with a focused session s-1
    And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    When the agent view renders a frame
    Then the rendered input area does not contain the text "deploy the fix"
    When the HITL prompt for s-1 is cleared as if the user answered
    And the agent view renders another frame
    Then the input buffer contains exactly "deploy the fix"
    And the cursor is at the end of "deploy the fix"

  # ─────────────────────────────────────────────────────────────────────
  # Esc/cancel correctness — the stranding-bug fix
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Esc on an options question cancels the whole request through the backend
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    When the user presses Esc
    Then backend.send_hitl_response is called exactly once with cancelled true and empty answers
    And the HITL prompt for s-1 is cleared
    And the composer draft is left untouched

  # ─────────────────────────────────────────────────────────────────────
  # Slot lifecycle — HITL wins over pause, Running clears
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Paused chunk stores a HITL slot that wins over the pause slot and Running clears it
    Given an App with a MockBackend and a focused session s-1
    And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    And all pending tasks have drained
    Then the HITL prompt slot for s-1 holds the fetched request
    And the AgentViewStore pause slot for s-1 is empty
    And no compositor layer is mounted for the HITL prompt
    When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    Then the HITL prompt slot for s-1 is empty

  # ─────────────────────────────────────────────────────────────────────
  # Removed invented behaviors + paste routing
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Hotkeys, Tab, scroll, and paste do nothing on an options question
    Given an App with a MockBackend and a focused session s-1
    And the composer input contains the draft "agent draft"
    And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    When the user types the characters "abc" and presses Tab
    Then backend.send_hitl_response is NEVER called
    And the HITL selection for s-1 is unchanged
    When a paste of "clipboard\ncontents" arrives
    Then the composer input still contains exactly "agent draft"
    And the HITL prompt for s-1 is still active

  Scenario: Paste in Other mode inserts into the shared input
    Given an App with a MockBackend and a focused session s-1
    And session s-1 has an inline HITL prompt in Other mode with an empty shared input
    When a paste of "pasted answer" arrives
    Then the shared composer input contains "pasted answer"
    And backend.send_hitl_response is NEVER called

  # ─────────────────────────────────────────────────────────────────────
  # Source shape — modal removal lock
  # ─────────────────────────────────────────────────────────────────────
  Scenario: The HITL modal is deleted and no construction site remains
    Given the codelet-fspec-tui crate sources
    Then the file rust/fspec-tui/src/components/hitl_dialog.rs does not exist
    And the crate sources never mention HITL_DIALOG_ID or HitlDialog
    And components/mod.rs declares no OpenHitlDialog action variant
    And the files hitl_state.rs, hitl_keys.rs, and hitl_prompt.rs each stay under 300 lines
