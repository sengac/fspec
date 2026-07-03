//! RPC-094 — shared scrollback dispatch helper, split out of `dispatch.rs`
//! so that file stays under the 300-LoC source-shape ceiling while keeping
//! canonical rustfmt formatting.

use super::state::App;
use crate::views::agent::TurnDir;

impl App {
    /// RPC-094: shared scrollback dispatch helper.
    pub(crate) fn scroll_focused(&mut self, delta: i64) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            // COPY-006 rule [7]: scrolling clears any live text selection.
            ctx.scrollback.selection_clear();
            if delta < 0 {
                ctx.scrollback.scroll_up(delta.unsigned_abs() as usize);
            } else if delta > 0 {
                ctx.scrollback.scroll_down(delta as usize);
            }
        }
    }

    /// RPC-381: drive the focused session's scrollback in/out of item
    /// mode to MATCH the AgentView's `turn_select_mode` flag. The flag
    /// itself is flipped in `views/agent/dispatch.rs` when Tab/Esc is
    /// pressed (so a standalone AgentView reflects the mode); this
    /// reducer only mirrors that onto the per-session scrollback. On
    /// enable the scrollback auto-selects the last turn; on disable the
    /// selection is cleared.
    pub(crate) fn handle_toggle_turn_select_mode(&mut self) {
        let enabled = self.navigator.agent.turn_select_mode;
        if !enabled {
            // RPC-382: leaving SELECT mode also tears down the modal.
            self.navigator.agent.turn_modal_seq = None;
        }
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            if enabled {
                ctx.scrollback.enter_item_mode();
            } else {
                ctx.scrollback.exit_item_mode();
            }
        }
    }

    /// RPC-382: open the turn content modal for the focused session's
    /// currently selected turn. The reducer is the single source of
    /// truth for the turn `seq`: it reads `selected_seq` from the
    /// focused scrollback and stores it on the AgentView
    /// (`navigator.agent` IS the view — same mirror pattern RPC-381 uses
    /// for `turn_select_mode`). No-op (modal stays closed) when nothing
    /// is selected.
    pub(crate) fn handle_open_turn_modal(&mut self) {
        let seq = self
            .agent_view_store
            .current_session_context()
            .and_then(|c| c.scrollback.selected_seq());
        self.navigator.agent.turn_modal_seq = seq;
        // RPC-383: opening always resets the scroll offset to the top.
        self.navigator.agent.turn_modal_offset = 0;
    }

    /// RPC-382: close the turn content modal (stays in SELECT mode).
    pub(crate) fn handle_close_turn_modal(&mut self) {
        self.navigator.agent.turn_modal_seq = None;
    }

    /// RPC-383: `true` for any of the six `Action::TurnModal*` scroll
    /// variants. Hosted on `App` so `dispatch.rs` keeps a single match
    /// arm (source-shape ceiling) without bloating `components/mod.rs`.
    pub(crate) fn is_turn_modal_scroll(action: &crate::components::Action) -> bool {
        use crate::components::Action;
        matches!(
            action,
            Action::TurnModalScrollUp
                | Action::TurnModalScrollDown
                | Action::TurnModalPageUp
                | Action::TurnModalPageDown
                | Action::TurnModalHome
                | Action::TurnModalEnd
        )
    }

    /// RPC-383: dispatch one of the `Action::TurnModal*` scroll variants
    /// onto the modal offset. Centralised here (rather than six match
    /// arms in `dispatch.rs`) so that file stays under the 300-LoC
    /// source-shape ceiling.
    pub(crate) fn dispatch_turn_modal_scroll(&mut self, action: &crate::components::Action) {
        use crate::components::Action;
        match action {
            Action::TurnModalScrollUp => self.scroll_turn_modal(-1),
            Action::TurnModalScrollDown => self.scroll_turn_modal(1),
            Action::TurnModalPageUp => {
                let page = self.turn_modal_page();
                self.scroll_turn_modal(-page);
            }
            Action::TurnModalPageDown => {
                let page = self.turn_modal_page();
                self.scroll_turn_modal(page);
            }
            Action::TurnModalHome => self.jump_turn_modal(false),
            Action::TurnModalEnd => self.jump_turn_modal(true),
            _ => {}
        }
    }

    /// RPC-383: scroll the OPEN turn content modal by `delta` rows
    /// (negative = up). Clamps `turn_modal_offset` to
    /// `[0, total_rows - viewport_rows]` so the last page stays fully
    /// visible and offset never goes negative. No-op when the modal is
    /// closed; the underlying turn selection is never touched.
    pub(crate) fn scroll_turn_modal(&mut self, delta: i64) {
        if self.navigator.agent.turn_modal_seq.is_none() {
            return;
        }
        // COPY-008 rule [6]: scrolling the modal clears any live selection.
        self.navigator.agent.turn_modal_selection = None;
        let (total, viewport) = self.turn_modal_metrics();
        let max_off = total.saturating_sub(viewport);
        let cur = self.navigator.agent.turn_modal_offset as i64;
        let next = (cur + delta).clamp(0, max_off as i64);
        self.navigator.agent.turn_modal_offset = next as usize;
    }

    /// RPC-383: jump the modal offset to the top (`0`) or bottom (last
    /// page). No-op when the modal is closed.
    pub(crate) fn jump_turn_modal(&mut self, to_bottom: bool) {
        if self.navigator.agent.turn_modal_seq.is_none() {
            return;
        }
        // COPY-008 rule [6]: jumping the modal clears any live selection.
        self.navigator.agent.turn_modal_selection = None;
        let (total, viewport) = self.turn_modal_metrics();
        self.navigator.agent.turn_modal_offset = if to_bottom {
            total.saturating_sub(viewport)
        } else {
            0
        };
    }

    /// RPC-383: a viewport-page step for the modal (PageUp/PageDown).
    pub(crate) fn turn_modal_page(&self) -> i64 {
        self.turn_modal_metrics().1.max(1) as i64
    }

    /// RPC-383: `(total_wrapped_rows, body_viewport_rows)` for the open
    /// modal, computed from the focused turn's full text and the last
    /// render area. Mirrors `TurnContentModal::render`'s geometry so the
    /// reducer clamps against the SAME page the user sees. Returns
    /// `(0, 1)` when the modal is closed / no area cached.
    fn turn_modal_metrics(&self) -> (usize, usize) {
        use crate::components::dialog_theme_rows::turn_modal_geometry;
        let Some(seq) = self.navigator.agent.turn_modal_seq else {
            return (0, 1);
        };
        let Some(area) = self.navigator.agent.last_render_area else {
            return (0, 1);
        };
        let text = self
            .agent_view_store
            .current_session_context()
            .and_then(|c| c.scrollback.full_text_for_seq(seq))
            .unwrap_or_default();
        let geom = turn_modal_geometry(area, &text);
        (geom.total_rows, geom.viewport_rows)
    }

    /// RPC-381: move the focused session's turn selection one step in
    /// `dir`, clamping at the first / last turn.
    pub(crate) fn handle_turn_nav(&mut self, dir: TurnDir) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            ctx.scrollback.navigate_turn(dir);
        }
    }

    /// COPY-006/010: begin a live text selection on the focused scrollback
    /// at `cell` (drag start), anchored precisely at the pressed cell.
    pub(crate) fn handle_selection_begin(&mut self, cell: crate::mouse::selection::Cell) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            ctx.scrollback.selection_begin(cell);
        }
    }

    /// COPY-010: begin a whole-line selection on the focused scrollback
    /// at `cell` (long-press).
    pub(crate) fn handle_selection_begin_line(&mut self, cell: crate::mouse::selection::Cell) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            ctx.scrollback.selection_begin_line(cell);
        }
    }

    /// COPY-006: extend the live selection's cursor to `cell` (drag).
    pub(crate) fn handle_selection_extend(&mut self, cell: crate::mouse::selection::Cell) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            ctx.scrollback.selection_extend(cell);
        }
    }

    /// COPY-006: clear the live selection + highlight (Esc / scroll).
    pub(crate) fn handle_selection_clear(&mut self) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            ctx.scrollback.selection_clear();
        }
    }

    /// COPY-006 rule [3]: reconstruct the selected text (COPY-004) and
    /// write it to the clipboard via OSC 52 (COPY-001). The selection is
    /// NOT cleared (rule [2]) so the highlight persists after the copy.
    pub(crate) fn handle_selection_commit(&mut self) {
        let text = self.agent_view_store.current_session_context().map(|c| {
            let spans = c.scrollback.selection_spans_for_content_width();
            c.scrollback.selected_text(&spans)
        });
        if let Some(text) = text {
            if text.is_empty() {
                return;
            }
            if let Err(e) = self.clipboard.copy(&text) {
                tracing::warn!("OSC 52 clipboard copy failed: {e}");
            }
        }
    }

    /// COPY-007: write the composer's prompt-free selected text to the
    /// terminal clipboard via OSC 52 (COPY-001), reusing the COPY-006
    /// clipboard writer. Empty text is a no-op.
    pub(crate) fn handle_copy_to_clipboard(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if let Err(e) = self.clipboard.copy(&text) {
            tracing::warn!("OSC 52 clipboard copy failed: {e}");
        }
    }

    /// COPY-006 test seam: replace the OSC 52 clipboard writer with an
    /// injected sink so integration tests can assert the exact bytes.
    /// Not `#[cfg(test)]` — integration tests compile without that cfg,
    /// matching the existing `next_pending_task` / `try_recv_action`
    /// pub test-seam convention on `App`.
    pub fn set_clipboard_writer_for_test(
        &mut self,
        writer: Box<dyn std::io::Write + Send>,
    ) {
        self.clipboard = crate::mouse::clipboard::Osc52Clipboard::new(writer);
    }

    /// COPY-006 test seam: drive the AgentView's long-press recognizer
    /// tick from an integration test (the production run loop calls
    /// `poll_selection_tick` from its 16ms render-tick arm — see
    /// `app/events.rs`). Fans any emitted gestures onto the action bus,
    /// exactly like the production tick. Public for the same reason as
    /// `set_clipboard_writer_for_test`: integration tests compile without
    /// the `#[cfg(test)]` cfg.
    pub fn poll_selection_tick_for_test(&mut self) {
        self.navigator.agent.poll_selection_tick();
    }
}
