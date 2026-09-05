@done
@tool-022
@tools
@tui
@session
@rpc
@tool-execution
@agent-view
Feature: Surface exec-session stdin prompts in the TUI composer slot (HITL integration)
  """
  DETERMINISTIC REDESIGN (vtcode-aligned; a content heuristic was rejected — vtcode's exec core never inspects command output, exec_support.rs:244-258 attaches steering to any still-running command; the TUI 'input required' shimmer is driven by HITL modal state, not process output, state.rs:337-343). P1 LLM-side signal (codelet-tools unified_exec): UnifiedExecResult gains quiet_seconds: Option<u64> (skip_serializing_if Option::is_none) — None when exited, Some(seconds since last output, floored) when still running. A fixed steering line ('Command still running. If it needs input, send it via the write action. Poll with a short yield_time_ms to check for new output.') is appended to output UNCONDITIONALLY on every still-running result (vtcode next_wait_args/next_action_hint analogue). Quiet time is computed deterministically: ProcessEntry gains last_output_micros (tokio monotonic, set at spawn and by the reader task on each read and by poll drains); quiet_secs = (now - last_output_micros) floored, else yield_elapsed when the drain itself took the whole window. The field threads through ExecToolFacadeWrapper into ExecOperationResult (facade/wrapper.rs) so Codex sees it; no facade/codex.rs schema change. P2 TUI inline prompt, pipe sessions (PTY stays a stub — P3): wire ExecStdinRequest { exec_session_id, command, quiet_seconds, ts_ms } in rpc-types (NAPI + serde, no hint/content field); BackgroundSession.exec_stdin_request: RwLock<Option<ExecStdinRequest>> mirroring hitl_request (~388), NO status flip, NO response channel; SessionManagerHandle get_exec_stdin_request + write_exec_stdin(session, exec_session, text) -> text+'\n' via the unified_exec global ProcessStore stdin_tx (codelet-tools is already a codelet-sessions dependency), clean error naming the exec session id for unknown/exited sessions; exposed in codelet-rpc embedded+websocket and codelet-napi session_bindings. Detector: per-exec-session task spawned with the reaper fires when child alive + quiet >= QUIET_THRESHOLD_SECS (3s); pushes via a per-agent-session callback registry (analogous to set_tool_progress_callback, tools/src/tool_progress.rs); 30s per-exec-session cooldown, never while a request is stored. TUI: AgentViewStore slot exec_stdin_by_session (store/agent_view/exec_stdin_state.rs mirroring hitl_state.rs); FspecBackend get/write (embedded.rs, websocket.rs, MockBackend); Actions ExecStdinPromptFetched/ExecStdinSubmit/ExecStdinDismissed; render in input_area.rs::paint_input_area precedence HITL > exec-stdin > pause > composer, SHARED MultiLineInput (placeholder 'Type to send to the command…', magenta ⌨, dim '(Enter Send | Esc Dismiss)', prompt line shows command display + 'has been quiet for Ns — waiting for input?'); keys in views/agent/exec_stdin_keys.rs consulted AFTER hitl_keys, BEFORE pause keys. Invariants: errors silently logged via tracing, composer draft preserved, only focused pane renders, slot cleared on session exit/focus-loss/None re-probe. Accepted tradeoff (decision §9.1): time-based trigger also fires for quiet non-interactive commands; mitigated by dismissibility, cooldown, and the quiet_seconds display. Secrets (decision §9.3): nothing content-derived crosses any surface, so no masking code exists; typed answers go straight to stdin and never appear in scrollback/store/wire.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. P2 overlay semantics (research §7.1/§7.6): the exec-stdin prompt is a non-blocking overlay — it does NOT change SessionStatus (stays Running), does NOT block any tool or the agent loop, and has NO response channel (no blocking wait, unlike HITL). Esc dismisses the prompt only (session keeps running, no cancel semantics). The user's typed text is written to the exec session's stdin exactly as if the LLM had called the write action (appends newline when the text doesn't end with one).
  #   2. P2 session state + handle methods: BackgroundSession gains exec_stdin_request: RwLock<Option<ExecStdinRequest>> (mirror of hitl_request at background_session.rs ~388; NO status flip, NO response channel). SessionManagerHandle gains get_exec_stdin_request(session_id) -> Option<ExecStdinRequest> (mirror of get_hitl_request at handle_impl.rs:968) and write_exec_stdin(session_id, exec_session_id, text) -> Result<(), String> which clones the exec session's stdin sender from the unified_exec global ProcessStore (codelet-tools is already a dependency of codelet-sessions) and sends text + newline (appended when absent, matching write-action semantics). Errors: unknown agent session -> 'Session not found'; unknown/executed exec session -> clean error naming the exec session id (NOT the -1 reaper-race noise, research §9.6). Exposed through codelet-rpc (embedded + websocket) and codelet-napi session_bindings.
  #   3. P2 TUI keys: a new exec_stdin_keys module (mirroring hitl_keys.rs) is consulted AFTER the HITL keys (HITL still wins) and BEFORE the pause keys. Enter captures the shared input text, clears it, and submits (fire-and-forget write_exec_stdin; slot cleared on success; on error the slot is kept and the error is logged via tracing — never a scrollback notice); Esc dismisses the prompt (ExecStdinDismissed, session keeps running); typing/paste routes into the shared input. Only the focused agent pane renders/handles the prompt; ghost panes show nothing (mux mode, research §9.4).
  #   4. P1 LLM-side signal (deterministic, vtcode-aligned — NO content heuristics): UnifiedExecResult gains `quiet_seconds: Option<u64>` — a pure timing fact. None when the process has exited; Some(seconds since last output, floored) when still running. There is NO waiting_for_input boolean and NO output-content inspection (vtcode's exec core never inspects what a command printed: it attaches deterministic next_wait_args steering to any still-running command, exec_support.rs:244).
  #   5. P1 steering hint (vtcode attach_long_command_wait_steering analogue): when the process is still running, the exec result output gains a fixed steering line: 'Command still running. If it needs input, send it via the write action. Poll with a short yield_time_ms to check for new output.' This is attached UNCONDITIONALLY to every still-running result — no content inspection, no conditional branch on output. It mirrors vtcode's next_wait_args + next_action_hint (exec_support.rs:244-258) which is attached to every still-running command regardless of what it printed.
  #   6. P2 wire type (deterministic): rpc-types gains ExecStdinRequest { exec_session_id: String (the unified_exec session id, NOT the agent session id), command: String (command display, already stored per ProcessEntry), quiet_seconds: u64 (seconds since last output when the detector fired — a deterministic timing fact, cooldown keying + display), ts_ms: u64 (detector fire time) }. NO hint/content field — nothing derived from output content crosses the wire. NAPI + serde object mirroring HitlRequest (rpc-types/src/lib.rs ~1202-1258).
  #   7. P2 detection trigger (deterministic, time-based — NO output-content inspection): a per-exec-session background task (spawned with the reaper at run) fires when the child is ALIVE AND the session has been quiet for >= QUIET_THRESHOLD_SECS (3s, measured from the entry's last-output timestamp — a fact stamped by the reader task, deterministic). It pushes an ExecStdinRequest to the owning agent session via a per-session callback (registry analogous to set_tool_progress_callback in tools/src/tool_progress.rs). Cooldown: re-fire at most every 30s per exec session, and never while a request is already stored for that agent session. Known consequence (accepted): the overlay also appears for legitimately quiet non-interactive commands (sleep, builds) — see decision rule 'DECISION (research §9.1)'. The LLM path and the user path both write the same stdin_tx; mpsc serialization makes concurrent writes safe.
  #   8. P2 TUI render contract: the exec-stdin prompt renders in the SAME composer input area as HITL (input_area.rs::paint_input_area) with precedence HITL > exec-stdin > pause > composer. Visual: magenta '⌨' glyph + bold command display + dim 'has been quiet for Ns — waiting for input?' on the prompt line (N from quiet_seconds; NO output content shown), then the SHARED composer MultiLineInput (placeholder 'Type to send to the command…'), then dim footer '(Enter Send | Esc Dismiss)'. Reuses the shared MultiLineInput exactly as HITL freeform does: composer draft + cursor survive the round-trip (paint over, never mutate, except the deliberate submit capture).
  #   9. P2 TUI store + lifecycle: AgentViewStore gains exec_stdin_by_session: HashMap<SessionId, ExecStdinRequest> (mirrors hitl_prompt_by_session in store/agent_view/hitl_state.rs). FspecBackend gains get_exec_stdin_request + write_exec_stdin (embedded + websocket + MockBackend). The slot is ephemeral: cleared when the exec session no longer exists (TUI re-probes get_exec_stdin_request on focus switch and clears on None), cleared on session exit/focus-loss, and never re-shown while a HITL prompt occupies the slot for that agent session. Per-agent-session isolation (multiple sessions can show exec prompts independently).
  #   10. DECISION (research §9.1 re-resolved under determinism): the P2 trigger is TIME-BASED ONLY (quiet >= 3s + child alive + stdin piped). No prompt-shape/content detection — a content heuristic was rejected as non-deterministic. Consequence accepted: the overlay WILL also appear for quiet non-interactive commands (sleep, tail -f, long builds). Mitigations (all deterministic, all in scope): (a) the overlay is dismissible (Esc, no side effects), (b) 30s per-exec-session cooldown limits re-firing, (c) never shown while a HITL prompt occupies the slot, (d) quiet_seconds display tells the user how long the command has been silent so a false positive is recognizable. A smarter deterministic refinement (e.g. PTY-based read-wait signals, or an LLM-declared 'this session expects input' flag surfaced via a tool parameter) is explicitly P3+ scope, not this card.
  #   11. DECISION (research §9.3 'mask secrets' — resolved by elimination): because no output content is ever surfaced (P1 exposes only the deterministic quiet_seconds + a fixed steering string; P2's ExecStdinRequest carries no hint/content field, and the TUI prompt line shows only the command display + quiet_seconds), there is NO secret-masking code to write and no content echo risk. Typed answers never appear in scrollback, the TUI store, or the exec-stdin wire type — they go straight to the session stdin_tx. The only channel that could echo typed text is the LLM's own tool output (the process echoing the answer in its next poll), which is pre-existing behavior of the exec tool, unchanged by this card. If a future card ever surfaces the prompt line itself (P3+), masking must be re-decided for that content.
  #
  # EXAMPLES:
  #   1. P1 — LLM sees the timing fact + steering: agent runs `printf 'y/n: '; sleep 30` via the exec tool. After ~4s it polls the session; the result carries session_id (no exit_code), the command's printed output, quiet_seconds >= 3, and the fixed steering line telling it to send input via the write action if needed. No claim about what the command is waiting for — the LLM knows what it launched.
  #   2. P2 — TUI user answers inline: agent runs a command that goes quiet for 3s while still running. The composer input area shows a magenta ⌨ prompt '<command> has been quiet for 5s — waiting for input?' with the shared text input. The user types 'y' and presses Enter; the text plus newline flows into the running command's stdin and the prompt slot clears. The agent kept streaming the whole time — no pause, no status change.
  #   3. P2 — Esc dismisses without side effects: the exec-stdin prompt is showing; user presses Esc. The prompt disappears, the command keeps running, nothing is cancelled or killed, and the composer draft the user had before the prompt appeared is still there.
  #   4. P2 (integration) — backend round-trip: the agent session's stored exec-stdin request is None while no live exec session is quiet; once a live session goes quiet >= 3s while its child is alive, the detector stores a request (with that session's command display + quiet_seconds) and the TUI's probe returns it; the TUI's write path sends the typed text to that exec session's stdin and the backend reports success. Mirrors the HITL get_hitl_request / send_hitl_response round-trip.
  #
  # ========================================
  Background: User Story
    As a TUI user or LLM agent
    I want to see and answer an interactive stdin prompt from a running unified_exec session
    So that answer waiting commands (passwords, y/n, REPLs) without killing the session or blind-polling

  # ======================================================================
  # P1 — LLM-side deterministic signal in unified_exec results
  # ======================================================================
  @unit
  @happy-path
  @TOOL-022
  Scenario: Still-running exec result carries the quiet_seconds timing fact
    Given a unified_exec session is running a command and has not exited
    When an exec result is produced for that session
    Then the result has a session_id and no exit_code
    And the result carries quiet_seconds describing how long the process has been quiet

  @unit
  @happy-path
  Scenario: Still-running exec result includes the fixed steering line
    Given a unified_exec session is running a command and has not exited
    When an exec result is produced for that session
    Then the result output includes the fixed steering line
    And the steering line tells the LLM to send input via the write action if needed
    And the steering line is present regardless of what the command printed

  @unit
  @regression
  Scenario: Exited exec result carries no quiet_seconds and no steering line
    Given a unified_exec command printed output and then exited
    When an exec result is produced for that session
    Then the result has an exit_code and no session_id
    And the result carries no quiet_seconds
    And the result output includes no steering line

  @unit
  Scenario: quiet_seconds grows as the process stays quiet
    Given a unified_exec session is running a silent command
    When the process has been quiet for at least 1 extra second and an exec result is produced via a poll
    Then the result quiet_seconds is at least 3
    And the result quiet_seconds is at most 8

  @unit
  @edge-case
  Scenario: quiet_seconds is a floored whole number
    Given a unified_exec session is running a command and has been quiet for 4.9 seconds
    When an exec result is produced for that session
    Then the result quiet_seconds is 4

  @tui
  @happy-path
  Scenario: TUI renders the exec-stdin prompt in the composer slot
  # ======================================================================
  # P2 — TUI inline prompt (pipe sessions, deterministic time-based trigger)
  # ======================================================================
    Given an agent session has a pending exec-stdin request for command "git commit" quiet for 5 seconds
    And the agent view is focused on that agent session
    When the agent view paints the input area
    Then the input area shows the exec-stdin prompt line with a magenta keyboard glyph
    And the prompt line shows the command display and how long it has been quiet
    And the prompt line shows no command output content
    And the input area shows the shared input with placeholder "Type to send to the command…"
    And the input area shows the dim footer "(Enter Send | Esc Dismiss)"
    And the exec-stdin prompt is painted in the composer slot ahead of the pause slot

  @tui
  @priority
  Scenario: HITL prompt takes precedence over the exec-stdin prompt
    Given an agent session has a pending exec-stdin request
    And an agent session has a pending HITL request
    When the agent view paints the input area
    Then the input area shows the HITL prompt
    And the input area does not show the exec-stdin prompt line

  @tui
  @integration
  Scenario: Enter submits the typed text to the exec session stdin and clears the prompt
    Given an agent session has a pending exec-stdin request for exec session "exec-abc"
    And the user types "y" into the shared input
    When the user presses Enter
    Then the backend write_exec_stdin is called with text "y" for exec session "exec-abc"
    And the exec-stdin prompt slot is cleared for the agent session
    And the shared input is cleared after submit

  @integration
  @happy-path
  Scenario: write_exec_stdin appends a newline to the typed text
    Given a live unified_exec session "exec-abc" has been quiet while running
    When the backend writes "yes" to exec session "exec-abc" stdin
    Then the exec session receives exactly "yes" plus a newline on its stdin

  @tui
  @regression
  Scenario: Esc dismisses the exec-stdin prompt without cancelling anything
    Given an agent session has a pending exec-stdin request
    And the shared input holds the draft "draft-text"
    When the user presses Esc
    Then the exec-stdin prompt slot is cleared for the agent session
    And the agent session status remains running
    And the shared input draft is preserved

  @tui
  @regression
  Scenario: No exec-stdin prompt while no exec session is waiting
    Given an agent session with no pending exec-stdin request
    When the agent view probes the backend for the exec-stdin request
    Then the backend returns no request
    And the input area shows the plain composer

  @integration
  @e2e
  Scenario: Backend round-trip surfaces the request only while a live exec session is quiet
    Given a live unified_exec session "exec-live" has been quiet for 3 seconds while running
    When the agent session detector fires for that exec session
    Then the agent session stores an exec-stdin request for "exec-live" with its command display and quiet seconds
    When the TUI probes the agent session for its exec-stdin request
    Then the TUI receives the stored request
    When the exec session exits
    Then the agent session has no stored exec-stdin request

  @integration
  @error
  Scenario: write_exec_stdin on an unknown exec session returns a clean error
    Given an agent session exists
    When the backend writes "x" to exec session "nope" stdin
    Then the backend returns an error naming the unknown exec session
    And the error does not contain the reaper race exit code noise

  @integration
  @regression
  Scenario: The exec-stdin overlay does not flip the agent session status
    Given a live unified_exec session "exec-live" has been quiet for 3 seconds while running
    When the agent session detector fires for that exec session
    Then the agent session status remains running
    And no Paused chunk was emitted for the agent session

  @unit
  @edge-case
  Scenario: Detector respects the cooldown and an already-stored prompt
    Given an agent session detector fired for exec session "exec-cool" 10 seconds ago
    When the exec session is quiet again while running
    Then the detector does not fire again before the 30 second cooldown elapses
    Given an agent session detector fired for exec session "exec-cool" 60 seconds ago
    When the exec session is quiet again while running
    Then the detector fires again

  @tui
  @regression
  Scenario: Slot is cleared when the exec session no longer exists
    Given an agent session has a pending exec-stdin request for exec session "exec-gone"
    And exec session "exec-gone" has exited
    When the agent view re-probes the backend on focus
    Then the backend returns no request
    And the exec-stdin prompt slot is cleared for the agent session

  @tui
  @edge-case
  Scenario: Ghost panes do not render the exec-stdin prompt
    Given agent session A has a pending exec-stdin request
    And agent session B is focused while agent session A is a ghost pane
    When the agent view paints all panes
    Then the focused pane input area shows no exec-stdin prompt line
    And only the focused agent pane renders an exec-stdin prompt when it has one

  @integration
  @happy-path
  Scenario: Bash delegation surfaces the exec session steering for still-running commands
    Given an agent session on any provider runs a command via the Bash tool that will not exit quickly
    When the Bash tool executes the command via the unified exec session machinery
    Then the command runs as a live unified exec session with piped stdin and pager suppression
    And the quiet detector is armed for that exec session and pushes an exec-stdin request to the agent session once the command is quiet for 3 seconds or more
    And the Bash tool preserves its one-shot contract by blocking until the command exits, returning the formatted stdout, stderr, and exit code
