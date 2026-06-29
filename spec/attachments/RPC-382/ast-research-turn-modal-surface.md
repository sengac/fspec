# AST Research — RPC-382 Turn Content Modal (Rust port)

Builds on RPC-381 (turn-select mode, now DONE). Structural surface for the modal.

## Modal state location
- `AgentView` (`views/agent.rs:85`) already holds `turn_select_mode: bool` (RPC-381).
  Add `turn_modal_seq: Option<u64>` here (defaults `None`; `#[derive(Default)]` holds).

## Selected turn → which turn to show
- RPC-381 added `ScrollbackList::selected_seq() -> Option<u64>` and `selected_index()`
  (`views/agent/scrollback_select.rs:98-105`). Enter reads `selected_seq` to know which
  turn to open.
- Full content source: `RenderedChunk.source: Option<ChunkSource>` where
  `ChunkSource.text` is the full un-wrapped body (`rendered_chunk.rs:54-73`); `ChunkKind`
  (`rendered_chunk.rs:21-45`) + `ChunkSource.color` give the role title/color. When
  `source` is `None`, fall back to joining `RenderedChunk.lines`.
- Add a read accessor, e.g. `ScrollbackList::full_text_for_seq(seq) -> Option<(String, ChunkKind, Color)>`
  (or split accessors) in scrollback_select.rs so the renderer can build the modal.

## Existing overlay convention to mirror
- `views/agent/confirm_dialog.rs`: `pub struct ConfirmDialog` (:38) with
  `pub fn render(&self, area: Rect, buf: &mut Buffer)` (:196) — centered popup, snapshot
  test at :251. `merge_confirm_dialog.rs` is a second example. The new
  `views/agent/turn_modal.rs` `TurnContentModal` follows this shape (centered, bordered
  block titled by role, body = full text wrapped to inner width). Keep < 300 lines.

## Render hook
- `views/agent.rs:208 render_with_store(...)`: paints header (:247) → scrollback (:258-260)
  → footer (:261) → input (:271). Add, AFTER input, an overlay paint:
  `if let Some(seq) = self.turn_modal_seq { TurnContentModal{...}.render(area, buf) }`.
  (Mode-views `resume_view`/`search_view` early-return at :210-217; the modal is NOT an
  early return — it overlays the normal AgentView chrome.)

## Dispatch surface (RPC-381 baseline to extend)
- `views/agent/dispatch_select.rs:19-40 handle_turn_select_key`: currently
  `KeyCode::Enter => Some(EventResult::consumed())` (suppress, :32) and
  `KeyCode::Esc => { self.turn_select_mode=false; emit(ToggleTurnSelectMode); consumed }`
  (:33-37). RPC-382 changes:
  - Enter → emit `OpenTurnModal(selected_seq)` (needs the focused scrollback's
    `selected_seq`; thread it in or read via a helper). Still consume.
  - Esc → if `turn_modal_seq.is_some()` emit `CloseTurnModal` + consume (stay in select
    mode); else existing exit-select-mode path.
- `views/agent/dispatch.rs:187-191` Tab branch (RPC-381): on disable, also clear
  `turn_modal_seq` (clear locally; the reducer mirror already runs).
- Focus gate: while `turn_modal_seq.is_some()`, the turn-nav/scrollback routing in
  `dispatch.rs:195-199` must NOT run (only Esc/Tab act). Add the guard there.

## Actions + reducer
- `components/mod.rs enum Action`: add `OpenTurnModal(u64)`, `CloseTurnModal` (bumps the
  components/mod.rs source-shape line budget in `tests/scrollback_scroll_rpc094.rs` like
  RPC-381 did — expect a small +N budget update).
- App reducer (`app/dispatch.rs` near RPC-381's ToggleTurnSelectMode arm, helper in
  `app/dispatch_scroll.rs`): `OpenTurnModal(seq)` → set `navigator.agent.turn_modal_seq =
  Some(seq)`; `CloseTurnModal` → set `None`. ToggleTurnSelectMode disable already clears
  it via the view; ensure the reducer disable path also clears for robustness.

## Conclusion
Additive on top of RPC-381: 1 new field, 2 new actions, 1 new widget file
(`turn_modal.rs`), Enter/Esc/Tab routing edits in dispatch_select.rs/dispatch.rs, a render
overlay hook, and a scrollback full-text accessor. No cross-cutting refactors.
