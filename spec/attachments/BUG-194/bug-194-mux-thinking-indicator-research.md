# BUG-194 Research — Mux: thinking indicator disappears when focus moves off a running agent pane

**Date:** 2026-09-27
**Status:** research complete, fix not yet implemented
**Symptom (user report):** With more than one view active in mux mode, when the agent-manager (agent session) pane's thinking indicator is on (agent is active), selecting a different view disables the thinking indicator even though the agent is still active. Selecting the agent view again makes the indicator appear.

## 1. Reproduction model

1. `/mux` with at least two panes, one of them an `Agent` pane.
2. Make the focused agent session `Running` (submit a prompt). The input row of the focused agent pane shows `⠙ Thinking... (Esc to stop)`.
3. Move focus to another pane (Shift+←/→ or mouse click):
   - Board pane → the Thinking indicator vanishes from the screen.
   - Another (idle) agent pane → the Thinking indicator vanishes too; the focused (idle) pane's composer appears.
4. The running session's `SessionStatus` is still `Running` (the agent keeps working).
5. Move focus back to that agent pane → the Thinking indicator reappears.

## 2. The two relevant pieces of state

### 2.1 Per-session status — CORRECT, untouched by focus

`AgentViewStore.session_status_by_session: HashMap<SessionId, SessionStatus>`
(`rust/fspec-tui/src/store/agent_view.rs`, field declared in the struct; written by
`app/dispatch_stream_chunks.rs` on `StreamChunk::SessionStateChange` and by
`Action::SessionStatusChanged` → `handle_session_session_status_changed`).

Focus changes NEVER mutate this map. The running session keeps reporting
`Running`. This is the correct source of truth for "the agent is active".

### 2.2 The visible "Thinking..." indicator — WRONG LAYER (the bug)

The visible indicator is **not** painted from the per-session status directly.
It is `AgentView::input_transition_state` — a **single**, view-level
`InputTransitionState` on the one shared `AgentView`
(`rust/fspec-tui/src/views/agent.rs`, field `input_transition_state`).

When that state is `Loading`, the input row paints the braille spinner
(`views/agent/input_transition.rs` → `render_input_transition` →
`paint_spinner(..., "Thinking", "(Esc to stop)")`; glyph math in
`components/spinner.rs`, 10 frames @ 80ms, DIM style).

`AgentView` is one object shared by ALL agent panes: `Navigator` owns a single
`agent: AgentView` (`views/navigator.rs`), and the mux render loop calls
`views.agent.render_session_pane(...)` once per agent pane
(`views/multiplex/render.rs`).

## 3. Mechanism — the focus-driven tick

In `views/agent/pane_render.rs`, `render_session_pane` ticks the animation
state machine **only for the focused pane** (documented there as
"BUG-163: tick the live-composer animation only for the focused pane —
spinner/transition state belongs to the live session"):

```rust
let (session_status, is_loading) = if pane.is_focused {
    self.tick_animation(store, sid.as_ref())
} else {
    let status = sid.as_ref().and_then(|s| store.session_status_for(s).copied());
    (status, matches!(status, Some(SessionStatus::Running)))
};
```

`tick_animation` (`views/agent/animation.rs`) reads the *focused pane's*
session status and feeds it to
`transition_driver::advance_transition` (`views/agent/transition_driver.rs`):

- focused session `Running`  → `InputTransitionState::Loading` (spinner shown)
- focused session `Idle`     → `Loading → Hiding → Showing → Idle`
  (the 5-char/17ms "finish" sweep that dismisses the indicator)
- focused session `Compacting` → `InputTransitionState::Compacting`

So the single shared `InputTransitionState` is always driven by
"whatever session the focused pane is showing".

### Exact bug sequence

1. Agent pane focused, session `Running` → focused pane ticks
   `tick_animation(sid=running)` → `Loading` → spinner painted. Correct.
2. User moves focus to the other pane (Shift+←/→ or mouse click).
   `App::handle_event` (after Navigator routing) calls
   `sync_mux_focus_to_session` (`app/dispatch_mux.rs`) →
   `switch_to_session_index` (`app/dispatch_session_cycle.rs`): the store's
   `current_session_index` follows the focused pane; the outgoing draft is
   snapshotted, the incoming draft restored. The running session's
   `SessionStatus` is untouched — still `Running`.
3. Next render: the focused pane is the other view (Board/Files/Checkpoints,
   or an idle agent pane). For every agent pane `pane.is_focused == false`, so
   **no agent pane ticks `tick_animation`** — they only read status and paint
   a ghost row:
   - If the focused pane is a non-agent view: the shared state stays stuck at
     `Loading` but the only place a spinner is painted is the focused agent
     pane's input row — which doesn't exist, so nothing is painted.
   - If the focused pane is an IDLE agent pane: that pane ticks with `Idle`
     status and **actively drives** the shared state
     `Loading → Hiding → Showing → Idle`, killing the indicator for the
     running session too.
4. User focuses the agent pane again → its session is still `Running` →
   `tick_animation` runs → `Loading` → spinner reappears.

### Why the per-pane header looks fine

The SessionHeader badges (`[R]`, tok/s, context-fill percent) ARE sourced
per-session in `views/agent/chrome_paint.rs` (`paint_header_and_role` reads
`store.token_state_for(sid)`, `store.model_info_for(sid)`, ...), so only the
input-row spinner/transition row is affected.

### Related but unaffected

- `AgentView::is_busy()` (`agent.rs`, backed by `spinner_started_at`) has no
  callers in the app today; the run-loop busy gate uses
  `App::is_session_busy` (`app/state/state_accessors.rs`), which reads the
  per-session status of the store's current session — correct but also
  focus-following (relevant to the redraw gate, not to this visual bug).
- The run loop keeps ticking 16ms draws while any session is busy, so frames
  are produced; the indicator simply isn't painted for the unfocused running
  pane.

## 4. Design origin

BUG-163 ("mux agent panes render distinct window sessions",
`spec/features/mux-agent-panes-render-distinct-window-sessions.feature`)
introduced "one live composer, many panes": the shared `MultiLineInput`
always holds the **focused** session's draft; unfocused agent panes paint a
read-only ghost of their persisted `input_draft`. Its example 4 even pins:

> "Agent 1's session is Idle and agent 2's session is Running: agent 2's pane
> (if unfocused) shows its ghost draft, **NOT a live 'Thinking' spinner** —
> the spinner is only painted in the focused pane."

That rule was written for the input **composer** (you can only type in one
place) and was implemented by gating the whole animation tick on focus —
which also suppressed the *thinking indicator* for any running session whose
pane is not focused. The assumption "the live session == the focused session"
(`pane_render.rs` doc comment) is exactly what breaks in mux, where the store's
current session is made to follow mux focus (`sync_mux_focus_to_session`).

## 5. Proposed fix (direction)

Make the thinking-indicator transition state **per-session** instead of
view-level/focus-driven:

1. Key `InputTransitionState` (and the `spinner_started_at` /
   `last_spinner_line` that feed it) **by session id** — either a
   `HashMap<SessionId, InputTransitionState>` on `AgentView`, or moved into
   the store per `SessionContext` (the store already holds per-session
   scrollback + drafts and is the natural home; a per-session transition slot
   on `SessionContext` keeps `AgentView`'s presentation fields slim).
2. In `render_session_pane`, tick that session's own transition state from
   **its own** session status for EVERY rendered agent pane (focused or not),
   not only the focused one.
3. Unfocused running panes paint a (dimmed is acceptable) "Thinking... /
   Compacting..." indicator row instead of the bare ghost draft; the live
   `MultiLineInput` composer remains exclusive to the focused pane (unchanged
   — typing still only reaches the focused pane, per mux keyboard isolation).
4. `App::is_input_animating` / the run-loop redraw gate should consider ANY
   per-session transition mid Hiding/Showing (or any session busy) so the
   finish animation completes even when the animating pane is unfocused.
5. Delete or repurpose the now-redundant view-level `input_transition_state`
   (and `AgentView::is_input_animating` accordingly), updating the
   `is_cursor_visible` gate to use the per-session state of the focused
   session.

### UX decision needed (red card)

For an UNFOCUSED running agent pane, the indicator should be painted as a
dimmed status row (ghost-pane style) rather than the full interactive spinner
row, so it remains visually subordinate to the focused pane. (Recommendation:
dimmed "Thinking... (running)" line in the ghost input row.)

## 6. Files that will be touched (planned)

- `rust/fspec-tui/src/views/agent/pane_render.rs` — per-pane tick from own status
- `rust/fspec-tui/src/views/agent/animation.rs` — per-session tick helper
- `rust/fspec-tui/src/views/agent/input_area.rs` — ghost row paints dimmed indicator
- `rust/fspec-tui/src/views/agent.rs` — remove/replace view-level transition state
- `rust/fspec-tui/src/store/agent_view/session_context.rs` — per-session transition slot (if store-home chosen)
- `rust/fspec-tui/src/app/state/state_accessors.rs` — `is_input_animating` considers all sessions
- `spec/features/mux-agent-panes-render-distinct-window-sessions.feature` — example 4 wording superseded
- New feature file for BUG-194 acceptance criteria

## 7. Existing tests that must keep passing

- `fspec-tui` mux tests: `tests/` mux-* (BUG-163/164/165/166/179/183 coverage)
- `thinking_indicator_animation_parity_rpc093.rs`,
  `thinking_streaming_parity_rpc093.rs`, `input_transition_tests.rs`,
  `transition_driver` unit tests
- `view_agent_unit_rpc018.rs` / source-shape tests that pin the 300-LoC ceilings
