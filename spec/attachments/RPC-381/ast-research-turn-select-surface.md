# AST Research — RPC-381 Turn-Selection Mode (Rust port)

Structural analysis (AstGrep + Read) of the Rust AgentView surfaces that this card touches.

## ScrollbackList public mutators (`views/agent/scrollback.rs`)
Existing `pub fn ... (&mut self)` surface (AstGrep `pub fn $NAME(&mut self, ...)`):
- `push(chunk)` :61
- `insert(idx, chunk)` :71
- `rewrap_at(i)` :107
- `set_viewport_height(h)` :147
- `set_viewport_width(w)` :158
- `scroll_up(lines)` :173
- `scroll_down(lines)` :179
- `jump_to_top()` :187, `jump_to_bottom()` :192, `reset()` :198

Fields: `chunks: Vec<RenderedChunk>`, `scroll_state: ScrollState`, `viewport_height/width`,
`last_rect`. **No selection field exists** → add `selection_mode` + `selected` here.

`RenderedChunk { seq: u64, lines: Vec<Line>, source: Option<ChunkSource> }`
(`rendered_chunk.rs:85`). `seq` is the stable per-turn id to key selection by.

## Render path (`views/agent/scrollback_paint.rs`)
- `paint_chunk_rows(area, buf, chunks, content_width, skip_rows) -> usize` :60 — the row
  painter. Arrow-bar rows for the selected chunk must be injected here (or in a wrapper
  that knows the selected index + the row offsets).
- `paint_scrollbar(...)` :23 — gutter; unaffected.

## Header badge (already present, wired off)
- `header_build.rs:87-92` renders ` [SELECT]` (cyan) when `is_select_mode == true`.
- `chrome_paint.rs:60` constructs `SessionHeader { ... is_select_mode: false, ... }` —
  HARDWIRED. Must be wired to the real flag.

## Dispatch (`views/agent/dispatch.rs`)
`handle_event` order: mouse → Ctrl+R → mode views → popups → Esc(:182 `AgentEscPressed`)
→ Ctrl+C → PageUp/PageDown/End/Home → Shift-arrows → MultiLineInput.
- **No Tab branch** at the agent level (Tab only consumed inside `slash_command_popup.rs`,
  `file_search_popup.rs`, `merge_confirm_dialog.rs`).
- Arrow keys: handled via `InputEventOutcome::Ignored` → `ScrollbackLineUp/Down` (:251-260).
  In select mode these must instead drive turn navigation.

## Action enum + App reducer
- `enum Action` in `components/mod.rs` (e.g. `ScrollbackPageUp` :291, `ScrollbackLineUp`
  :301). Add `ToggleTurnSelectMode`, `TurnNavUp`, `TurnNavDown` here.
- App reducer `app/dispatch.rs:227-241` handles `Scrollback*` by calling `scroll_focused`
  / `current_session_context_mut().scrollback...`. New actions reduce the same way:
  mutate `agent_view.turn_select_mode` and the focused `ctx.scrollback`.
- Esc cascade entry: `app/dispatch_esc_cascade.rs` + `handle_agent_esc_pressed()`
  (`dispatch_model_thinking_dialogs.rs:284`). Select-mode exit is decided in
  `views/agent/dispatch.rs` BEFORE emitting `AgentEscPressed` (consume locally).

## AgentView struct (`views/agent.rs:85`)
`#[derive(Default)] pub struct AgentView { input, action_tx, ..., slash_popup, file_popup,
resume_view, search_view, ... }`. Add `turn_select_mode: bool` here (presentation state,
mirrors TS `isTurnSelectMode`). `bool` defaults to `false` so `#[derive(Default)]` holds.

## Per-session state (`store/agent_view/session_context.rs`)
`SessionContext { id, scrollback: ScrollbackList, scrollback_next_seq, ... }`. Selection
on the `ScrollbackList` is therefore automatically per-session.

## Conclusion
All integration points are existing, well-isolated files. The change is additive:
2 new fields, 3 new actions, 1 dispatch branch, 1 render augmentation, 1 header wiring fix.
No new crates or cross-cutting refactors required.
