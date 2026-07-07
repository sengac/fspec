//! RPC-381 — turn-selection (SELECT) mode for [`ScrollbackList`].
//!
//! Feature: spec/features/agentview-turn-select-mode.feature
//!
//! Extracted from `scrollback.rs` to keep that file under the 300-LoC
//! source-shape ceiling. A "turn" is exactly one [`RenderedChunk`]
//! (keyed by its stable `seq`), so navigation = move the selected
//! chunk index by ±1 and highlight = frame that chunk's visible rows
//! with the ▼ / ▲ arrow bars (painted by `scrollback_paint`).
//!
//! Selection is remembered by `seq` (not index) so it stays pinned to
//! the SAME turn when new chunks stream in — a port of the TS
//! `VirtualList.tsx:258-274` selection-preservation logic.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::ScrollbackList;

/// RPC-381: scrollback selection mode. `Scroll` is the default chat-log
/// line/page scroll; `Item` is the Tab-toggled turn-selection (SELECT)
/// mode where a whole [`super::RenderedChunk`] (= one conversation
/// turn) is the selectable unit. Port of the TS `VirtualList`
/// `selectionMode` (`'scroll' | 'item'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Line/page scrolling (default).
    #[default]
    Scroll,
    /// Turn-by-turn item selection.
    Item,
}

/// RPC-381: direction for [`ScrollbackList::navigate_turn`]. `Up` moves
/// the selection to the previous (older) turn; `Down` to the next
/// (newer) turn. Both clamp at the list ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDir {
    /// Move to the previous (older) turn.
    Up,
    /// Move to the next (newer) turn.
    Down,
}

impl ScrollbackList {
    /// RPC-381: enter SELECT (item) mode and auto-select the last turn,
    /// mirroring the TS `scroll → item` transition with `scrollToEnd`.
    pub fn enter_item_mode(&mut self) {
        self.selection_mode = SelectionMode::Item;
        self.select_last_turn();
    }

    /// RPC-381: leave SELECT mode and clear the selection.
    pub fn exit_item_mode(&mut self) {
        self.selection_mode = SelectionMode::Scroll;
        self.selected = None;
        self.selected_seq = None;
    }

    /// RPC-381: drop any selection and return to Scroll mode (used by
    /// `reset` when the conversation is cleared).
    pub(super) fn clear_selection(&mut self) {
        self.exit_item_mode();
    }

    /// RPC-381: select the most-recent (last) turn. No-op on an empty
    /// list (selection becomes `None`).
    pub fn select_last_turn(&mut self) {
        let last = self.chunks.len().checked_sub(1);
        self.set_selected(last);
    }

    /// RPC-381: move the selection one turn `Up` (older) or `Down`
    /// (newer), clamping at the first / last turn. Keeps the selected
    /// turn visible by re-anchoring the scroll offset.
    pub fn navigate_turn(&mut self, dir: TurnDir) {
        let Some(current) = self.selected else {
            return;
        };
        let last = match self.chunks.len().checked_sub(1) {
            Some(n) => n,
            None => return,
        };
        let next = match dir {
            TurnDir::Up => current.saturating_sub(1),
            TurnDir::Down => current.saturating_add(1).min(last),
        };
        self.set_selected(Some(next));
        self.scroll_selected_into_view();
    }

    /// RPC-381: the current selection mode.
    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode
    }

    /// RPC-381: the selected turn's index into `chunks`, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// RPC-381: the selected turn's stable `seq`, if any.
    pub fn selected_seq(&self) -> Option<u64> {
        self.selected_seq
    }

    /// RPC-382: the FULL text of the turn with stable `seq`, for the
    /// turn content modal. Prefers the wrappable `ChunkSource::text`
    /// (the un-truncated body); falls back to joining the cached
    /// `RenderedChunk::lines` to plain strings when `source` is `None`
    /// (legacy pre-rendered chunks). Returns `None` when no turn has
    /// that `seq`.
    pub fn full_text_for_seq(&self, seq: u64) -> Option<String> {
        let chunk = self.chunks().iter().find(|c| c.seq == seq)?;
        if let Some(src) = chunk.source.as_ref() {
            // RPC-391: a diff card keeps the FULL (uncollapsed) diff in
            // `full_text`; prefer it so the modal shows every line.
            if let Some(full) = src.full_text.as_ref() {
                return Some(full.clone());
            }
            return Some(src.text.clone());
        }
        let joined = chunk
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        Some(joined)
    }

    /// RPC-382: the `ChunkKind` (role tag) of the turn with stable
    /// `seq`, used by the modal to color its title. `None` when the
    /// turn has no `ChunkSource` (legacy pre-rendered) or no match.
    pub fn kind_for_seq(&self, seq: u64) -> Option<crate::views::agent::ChunkKind> {
        self.chunks()
            .iter()
            .find(|c| c.seq == seq)
            .and_then(|c| c.source.as_ref())
            .map(|s| s.kind.clone())
    }

    /// RPC-381: in Item mode, frame the selected turn with the ▼/▲
    /// gray arrow bars. No-op in Scroll mode or with no selection.
    pub(super) fn paint_selection_overlay(
        &self,
        area: Rect,
        buf: &mut Buffer,
        content_width: u16,
        skip_rows: usize,
    ) {
        if self.selection_mode != SelectionMode::Item {
            return;
        }
        if let Some(sel) = self.selected {
            crate::views::agent::scrollback_paint::paint_selection_arrow_bars(
                area,
                buf,
                self.chunks(),
                content_width,
                skip_rows,
                sel,
            );
        }
    }

    /// RPC-416: after a caller removes a chunk directly via
    /// `chunks_mut`, re-pin the SELECT-mode selection from its remembered
    /// `seq` (cleared if the turn is gone) and re-anchor stick-to-bottom.
    pub fn reanchor_after_removal(&mut self) {
        self.resolve_selection_from_seq();
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        }
    }

    /// Set the selected index AND remember its `seq` so the selection
    /// can be re-pinned after a chunk mutation.
    fn set_selected(&mut self, idx: Option<usize>) {
        self.selected = idx;
        self.selected_seq = idx.and_then(|i| self.chunks.get(i)).map(|c| c.seq);
    }

    /// RPC-381: after a `push` / `insert`, re-resolve the selected index
    /// from the remembered `seq` so the selection stays on the SAME
    /// turn. When the seq no longer exists (the turn was removed) the
    /// selection is cleared.
    pub(super) fn resolve_selection_from_seq(&mut self) {
        if self.selection_mode != SelectionMode::Item {
            return;
        }
        let Some(seq) = self.selected_seq else {
            return;
        };
        self.selected = self.chunks.iter().position(|c| c.seq == seq);
    }

    /// RPC-381: adjust `scroll_state.offset` so the selected chunk's row
    /// span — plus the two framing arrow-bar rows — stays inside the
    /// viewport. Port of the `getVisibleRange` + scroll-to-keep-visible
    /// logic (`VirtualList.tsx:368-420`); because a turn is one chunk,
    /// the range reduces to "first visual row of chunk N .. last".
    fn scroll_selected_into_view(&mut self) {
        let Some(sel) = self.selected else {
            return;
        };
        let vh = self.viewport_height as usize;
        if vh == 0 {
            return;
        }
        // First visual row of the selected chunk, and its row count.
        let mut first_row = 0usize;
        for chunk in self.chunks.iter().take(sel) {
            first_row += chunk.lines.len();
        }
        let chunk_rows = self.chunks.get(sel).map(|c| c.lines.len()).unwrap_or(0);
        // Reserve the +2 arrow-bar rows (one above, one below).
        let top = first_row.saturating_sub(1);
        let bottom = first_row + chunk_rows; // inclusive of the ▲ bar row
        let offset = self.scroll_state.offset;
        if top < offset {
            self.scroll_state.stick_to_bottom = false;
            self.scroll_state.offset = top;
        } else if bottom >= offset + vh {
            self.scroll_state.stick_to_bottom = false;
            self.scroll_state.offset = bottom.saturating_sub(vh).saturating_add(1);
        }
    }
}
