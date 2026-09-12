@done
@rust
@agent-view
@tui
@mux
@input
@bug-179
@BUG-179
Feature: Mux agent input paste and terminal file drop routing
  """
  In mux mode (ViewMode::Mux, mux.config().enabled) the agent input area is dead to both
  clipboard paste (bracketed paste) and terminal file drop: the mux keyboard-isolation
  gate (Navigator::handle_mux_event) drops every non-Event::Key event, and crossterm 0.28
  has no file-drop Event variant — terminals synthesize drops as typed text or as a
  bracketed paste. The unified root-cause fix: (1) route Event::Paste through the same
  focused-pane forward path as keys (forward_mux_event_to_focused_pane), reusing the
  existing AgentView paste precedence/gate chain verbatim; (2) mirror the post-Navigator
  sync_mux_focus_to_session step from App::handle_event in App::handle_paste so the
  RPC-024/052 draft round-trip always targets the focused pane's window session. No
  file:// decoding, no drop toast — mux parity means identical semantics to single-view
  mode, which has no such behavior either.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A bracketed paste (Event::Paste) delivered while mux is active is routed to the FOCUSED pane only — same keyboard-isolation rule as keys (R2); the Board/Files/Checkpoints panes ignore it, the Agent pane inserts it into the live composer
  #   2. Composer precedence and gates in mux are IDENTICAL to single view: HITL freeform/exec-stdin/pause prompt routes win over the composer (dispatch.rs paste arms), the RPC-095 compacting block_edits gate suppresses paste, CRLF is normalized to LF, and PendingInputChanged is emitted only when the buffer changed
  #   3. After mux paste routing, App::handle_paste runs the same post-Navigator mux focus sync as App::handle_event (sync_mux_focus_to_session), so the RPC-024/052 draft round-trip always targets the focused pane's window session
  #   4. A paste must never mutate the ghost input_draft of an unfocused agent pane — the single shared MultiLineInput belongs to the focused pane's session only (BUG-163 model)
  #   5. No behavior change outside mux (R10): when mux.config().enabled is false the event flow is byte-for-byte unchanged and the existing RPC-403 paste routing tests stay green
  #   6. Scope is limited to the unified root-cause fix: route Event::Paste to the focused mux pane (reusing the existing AgentView paste precedence/gate chain) and mirror the mux focus sync in App::handle_paste. No file:// decoding, no drop toast — single-view mode has no such behavior, so mux parity means identical semantics. file://-URL handling and a dedicated drop UX are out of scope (documented as non-goals in the attached research)
  #
  # EXAMPLES:
  #   1. Repro (drop): drag a screenshot onto the agent pane's input area in mux mode — terminals deliver the drop as a bracketed paste (path string) or as typed characters; after the fix a paste-style drop inserts the path into the focused pane's composer exactly like single-view mode, while typed-character drops already flow to the focused pane under R2
  #   2. Repro: enter mux with /mux board agent agent, focus the agent pane (Shift+Right or click inside it), then Ctrl+V a 3-line snippet — today nothing is inserted; after the fix all 3 lines land in the composer with the cursor at the end
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the fix scope be limited to routing Event::Paste (bracketed paste / drop-as-paste) to the focused mux pane (the unified root-cause fix — typed-character drops already flow to the focused pane under R2), or should it also add file://-URL decoding or a drop-specific UX (e.g. a toast showing the dropped path)?
  #   A: Scope is limited to the unified root-cause fix: route Event::Paste to the focused mux pane (reusing the existing AgentView paste precedence/gate chain) and mirror the mux focus sync in App::handle_paste. No file:// decoding, no drop toast — single-view mode has no such behavior, so mux parity means identical semantics. file://-URL handling and a dedicated drop UX are out of scope (documented as non-goals in the attached research)
  #
  # ========================================
  Background: User Story
    As a TUI user supervising agents in mux mode
    I want to paste text or drop a file into the focused agent pane's input
    So that the composer receives the text exactly as it does in single-view Agent mode

  # Rule 1: paste routes to the focused agent pane and inserts into the composer
  @bug-179
  Scenario: multi-line paste into the focused agent pane is inserted into the composer
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the first agent pane is focused
    When I paste a 3-line snippet
    Then the composer holds all 3 lines separated by newlines
    And the cursor is at the end of the pasted text

  @bug-179
  Scenario: paste while the board pane is focused is ignored
  # Rule 1: paste is ignored by non-agent focused panes (the board pane has
  # no text input, matching single-view Board semantics)
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the board pane is focused
    When I paste a text snippet
    Then the paste is not consumed
    And the composer remains empty

  @bug-179
  Scenario: paste after click-to-focus lands in the newly focused session's composer
  # Rule 1 + Rule 3: click-to-focus converges the store to the clicked
  # pane's window session; a paste arriving after that lands in that
  # session's composer.
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    When I click inside the second agent pane
    And I paste a text snippet
    Then the composer holds the pasted text
    And the store's current session is the second pane's window session

  @bug-179
  Scenario: paste never mutates the ghost draft of an unfocused agent pane
  # Rule 4: a paste never mutates an unfocused agent pane's ghost draft —
  # the single shared MultiLineInput belongs to the focused pane's session only
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the first agent pane is focused
    And the second session's input draft holds the text "ghost-draft"
    When I paste a text snippet
    Then the composer holds the pasted text
    And the second session's input draft still holds the text "ghost-draft"

  @bug-179
  Scenario: paste while the focused session is compacting is suppressed
  # Rule 2: the compacting block_edits gate is preserved in mux (parity with
  # the single-view RPC-403 rule 5)
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the second agent pane is focused
    And the focused session is compacting
    And the composer holds the text "hello"
    When I paste a text snippet
    Then the composer still holds the text "hello"

  @bug-179
  Scenario: single-view paste is unchanged when the mux layout is disabled
  # Rule 5: no behavior change outside mux (R10) — the existing RPC-403
  # single-view paste routing stays green
    Given the agent view is active with the mux layout disabled
    When I paste a 3-line snippet
    Then the composer holds all 3 lines separated by newlines
    And the cursor is at the end of the pasted text

  @bug-179
  Scenario: a terminal file drop delivered as a bracketed paste inserts the path verbatim
  # Rule 6 / example 1: drop-as-paste equivalence — a terminal file drop
  # delivered as a bracketed paste is inserted verbatim (no file:// decoding,
  # no drop UX — mux parity means identical semantics to single-view mode)
    Given mux mode is active with the pane list board, agent and agent
    And two agent sessions are open
    And the first agent pane is focused
    When I drop a file that my terminal delivers as a bracketed paste with the text "file:///tmp/screenshot.png"
    Then the composer holds "file:///tmp/screenshot.png" verbatim
