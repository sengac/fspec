# AST Research — MUX-002 (agent-slot window cycling)

Research date: 2026-08-27. Scope: `rust/fspec-tui/src/views/multiplex/`,
`rust/fspec-tui/src/views/navigator.rs`,
`rust/fspec-tui/src/store/agent_view.rs` + `store/agent_view/navigation.rs`,
`rust/fspec-tui/src/app/dispatch_session_cycle.rs`,
`rust/fspec-tui/src/app/dispatch_create_session_dialog.rs`.

## Key entities (via AstGrep + direct reading)

### `MultiplexLayout` (views/multiplex/mod.rs)
- Fields: `config: MuxConfig`, `pane_rects: Vec<Rect>`, `divider_rect`,
  `is_dragging`, `drag_width`, `focus: MuxFocus`, `pre_mux_view`.
- **No `window_start` state exists yet** — MUX-002 must add it.
- `cycle_pane_next()` (line 209): wraps `focus` 0..n (ring).
  MUX-002: right-edge behavior must change (prompt/rotate instead of wrap).
- `cycle_pane_prev()` (line 216): wraps from 0 to n-1.
  MUX-002: left edge must STOP (no wrap).
- `set_pane_list(panes, split_percent)` (line 239): replaces the pane list.
- `MuxConfig.panes: Vec<MuxPaneKind>` — the fixed SLOT layout.
  Agent slots = positions where `panes[i] == MuxPaneKind::Agent`.

### `render_with_stores` (views/multiplex/render.rs:32)
- Computes rects from `config.panes` (ALL slots, including unfilled agent
  slots — MUX-002 must filter unfilled agent slots out of the rect list
  and let the remaining panes absorb the space).
- Renders each pane by kind; `MuxPaneKind::Agent` renders the SINGLE
  `AgentView` bound to the store's current session. MUX-002: agent slot i
  must render the session at `window_start + i` — requires per-slot session
  selection (AgentView is one instance; the slot's session must be
  focused/selected in `AgentViewStore` for that pane's render, or a
  per-slot render context must be introduced).
- `paint_footer` (line 104) labels panes by kind — must reflect the
  filtered slot list.

### `classify_key` (views/multiplex/keys.rs:32)
- Shift+Left/Right → `KeyDecision::FocusPrev` / `FocusNext`; everything
  else → `Forward`. MUX-002: the Navigator must intercept the
  rightmost-pane case (prompt/rotate) and the leftmost-pane case (stop)
  before calling `cycle_pane_next/prev`.

### `Navigator::handle_mux_event` (views/navigator.rs:142)
- Executes `KeyDecision`: `FocusPrev` → `mux.cycle_pane_prev()`,
  `FocusNext` → `mux.cycle_pane_next()`. This is the single place to
  implement the edge behavior.
- `forward_mux_event_to_focused_pane` (line 244) forwards to the focused
  pane by kind; agent pane → `agent_view.handle_event`.

### `AgentViewStore` (store/agent_view.rs)
- `open_sessions() -> &[SessionContext]` (line 118): the ordered list of
  open sessions — the window's source.
- `current_session_index()` (line 126), `focus_session_index(i)` (line 168),
  `current_session() -> Option<&SessionId>` (line 198).
- `navigation.rs`: `navigate_next()` / `navigate_prev()` return
  `NavTarget::{Session, CreateDialog, Board}` — the non-mux end-of-list
  semantics that MUX-002 must mirror at the mux right edge.

### `App::handle_session_cycle` (app/dispatch_session_cycle.rs:86)
- Non-mux Shift+Left/Right reducer: `NavTarget::CreateDialog` →
  `request_create_session_dialog_no_auto()` + `handle_open_create_session_dialog(None)`
  (the exact dialog path MUX-002 must reuse at the mux right edge, with NO
  work-unit attachment).
- `handle_open_agent_view` (line 53): BoardView Shift+Right path — probes
  `first_open_session_id()` before mounting the dialog (RPC-097 reopen #2).

### `App::handle_open_create_session_dialog` (app/dispatch_create_session_dialog.rs:39)
- Mounts `CreateSessionDialog` on the Compositor (Priority::Foreground),
  idempotent on `CREATE_SESSION_DIALOG_ID`. Dialog overlays the full
  screen (R9: coexists with mux).
- `handle_create_session_submitted` (line 70): on confirm, creates the
  session and flips to Agent view. MUX-002: when mux is active, the
  confirm handler must instead advance the mux window so the new session
  lands in the last agent slot and focus moves to it (view stays Mux).

## Gaps to fill (implementation checklist)
1. `MultiplexLayout.window_start: usize` + clamp helper
   (`max(0, sessions - agent_slots)`), re-clamped on session
   create/close and render.
2. Rect computation: filter unfilled agent slots; remaining panes absorb
   the space (extend `calculate_pane_rects` or pre-filter the pane list).
3. Per-agent-slot session binding: agent slot i renders
   `open_sessions[window_start + i]` (AgentView is a single instance —
   decide: focus the slot's session for its render pass, or introduce a
   per-slot render context).
4. `Navigator::handle_mux_event` edge interception:
   - rightmost pane + Shift+Right → CreateSessionDialog (no attachment),
     regardless of pane kind.
   - rightmost AGENT pane, all agent slots filled + Shift+Right → window
     forward; Shift+Left → window backward (fall through to focus move
     when window can't rotate back).
   - leftmost pane + Shift+Left → stop (no wrap).
5. `handle_create_session_submitted` mux-aware branch: advance window,
   focus new agent pane, stay in Mux view.
6. Session close: window re-clamp (no layout removal).
