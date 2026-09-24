//! ScrollbackList — windowed scrollback widget for AgentView (RPC-019/
//! 078/094). Viewport-slice paint; `offset` in VISUAL ROWS. SELECT-mode
//! lives in `scrollback_select`, live text-selection (COPY-006) in `copy`,
//! the render pass (BUG-190: row reflow cache) in `scrollback_render`, the
//! visual-row cap + oldest-content trim (BUG-192) in `scrollback_trim`.
//!
//! Features: rpc019-scrollback, agentview-scrollback-wrap,
//! rpc094-agentview-scrollback-scroll, scrollback-row-reflow-cache,
//! agentview-scrollback-unbounded-growth-cap-total-visual-rows.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::reflow_cache::RowReflowCounter;
use super::RenderedChunk;
use crate::store::agent_view::chunk_wrap::wrap_source;

pub use select::{SelectionMode, TurnDir};

/// Scroll state. `stick_to_bottom = true` is the chat-log default; PageUp drops out, End/PageDown re-enters.
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    /// First visible visual row index (across all wrapped Lines).
    pub offset: usize,
    /// When true, render is bottom-anchored.
    pub stick_to_bottom: bool,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            stick_to_bottom: true,
        }
    }
}

/// Windowed scrollback panel. Per-frame work is O(viewport_height) and —
/// since BUG-190 — O(unchanged-rows) re-wrap: steady-state frames blit
/// cached reflected rows from the `RowReflowCache`.
#[derive(Debug, Default)]
pub struct ScrollbackList {
    /// `pub(super)`: `scrollback_tail` derives total-row math.
    pub(super) chunks: Vec<RenderedChunk>,
    pub(super) scroll_state: ScrollState,
    /// Latest observed viewport height (rows).
    pub(super) viewport_height: u16,
    /// Latest viewport width (cols). Drives per-chunk re-wrap on resize (RPC-078).
    viewport_width: u16,
    /// RPC-094: cached layout rect; `mouse_dispatch` hit-tests this.
    last_rect: Option<Rect>,
    // RPC-381 SELECT-mode state; logic lives in `scrollback_select`.
    selection_mode: SelectionMode,
    selected: Option<usize>,
    selected_seq: Option<u64>,
    /// COPY-005/006: viewport-space REVERSED-overlay spans (empty = none).
    selection_highlight_spans: Vec<crate::mouse::selection::RowSpan>,
    /// COPY-006: live text selection (anchor/cursor); None when inactive.
    selection: Option<crate::mouse::selection::Selection>,
    /// COPY-006: content width (viewport minus gutter) cached at last render — shared clamp.
    content_width: u16,
    /// BUG-190: per-(chunk content, width) reflected-row cache so the
    /// 60fps busy-state repaint blits cached rows instead of re-wrapping
    /// + re-grapheme-scanning every visible row every frame.
    reflow_cache: super::reflow_cache::RowReflowCache,
    /// BUG-192: trimmed rows accumulated across trims (shown in the
    /// marker line). Reset to 0 by `reset`.
    trimmed_rows_total: usize,
}

impl ScrollbackList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a chunk; re-wrap from `ChunkSource` if width known.
    pub fn push(&mut self, chunk: RenderedChunk) {
        let idx = self.chunks.len();
        self.chunks.push(chunk);
        self.rewrap_after_mutation(idx);
    }

    /// Insert at `idx`, shifting right (RPC-093: `append_thinking` splices a thinking block).
    pub fn insert(&mut self, idx: usize, chunk: RenderedChunk) {
        self.chunks.insert(idx, chunk);
        self.rewrap_after_mutation(idx);
    }

    /// Post-mutation hook for `push`/`insert`: rewrap touched chunk, re-anchor stick, re-pin select.
    fn rewrap_after_mutation(&mut self, idx: usize) {
        if self.viewport_width != 0 {
            if let Some(c) = self.chunks.get_mut(idx) {
                rewrap_chunk(c, self.viewport_width);
            }
        }
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        }
        self.resolve_selection_from_seq(); // RPC-381: re-pin selection.
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Read-only access to the underlying chunks vector. **RPC-091**.
    pub fn chunks(&self) -> &[RenderedChunk] {
        &self.chunks
    }

    /// Mutable access to the underlying chunks vector. **RPC-091**.
    pub fn chunks_mut(&mut self) -> &mut Vec<RenderedChunk> {
        &mut self.chunks
    }

    /// Re-wrap a single chunk at index `i` (or 80 cols pre-render). **RPC-091**.
    pub fn rewrap_at(&mut self, i: usize) {
        let width = if self.viewport_width != 0 {
            self.viewport_width
        } else {
            80
        };
        if let Some(chunk) = self.chunks.get_mut(i) {
            rewrap_chunk(chunk, width);
        }
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        }
    }

    /// Visible window from `offset` outward, capped by `viewport_lines`.
    pub fn visible_window(&self, viewport_lines: u16) -> Vec<RenderedChunk> {
        let off = self.scroll_state.offset;
        let mut out = Vec::new();
        let mut rows_used: u16 = 0;
        let mut row_idx: usize = 0;
        for chunk in self.chunks.iter() {
            let chunk_rows = chunk.lines.len();
            if row_idx + chunk_rows <= off {
                row_idx += chunk_rows;
                continue;
            }
            if rows_used >= viewport_lines {
                break;
            }
            out.push(chunk.clone());
            rows_used = rows_used.saturating_add(chunk_rows as u16);
            row_idx += chunk_rows;
        }
        out
    }

    pub fn scroll_state(&self) -> ScrollState {
        self.scroll_state
    }

    /// BUG-190: read-only reflow-cache counters (reflow misses vs cache
    /// blits) for tests + profiling.
    pub fn reflow_cache(&self) -> RowReflowCounter {
        self.reflow_cache.counters()
    }

    /// Update cached viewport height. Idempotent.
    pub fn set_viewport_height(&mut self, h: u16) {
        if self.viewport_height != h {
            self.viewport_height = h;
            if self.scroll_state.stick_to_bottom {
                self.recompute_offset_for_stick();
            }
        }
    }

    /// Update cached viewport width. Re-wraps every `ChunkSource` chunk so resize never permanently truncates.
    pub fn set_viewport_width(&mut self, w: u16) {
        if self.viewport_width != w {
            self.viewport_width = w;
            if w != 0 {
                for chunk in self.chunks.iter_mut() {
                    rewrap_chunk(chunk, w);
                }
            }
            if self.scroll_state.stick_to_bottom {
                self.recompute_offset_for_stick();
            }
        }
    }

    /// PageUp: drop stick mode, step `offset` back by `lines`.
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_state.stick_to_bottom = false;
        self.scroll_state.offset = self.scroll_state.offset.saturating_sub(lines);
    }

    /// PageDown/End: step `offset` forward, cap at tail, re-enter stick.
    pub fn scroll_down(&mut self, lines: usize) {
        let max_off = self.max_offset_for_viewport();
        self.scroll_state.offset = self.scroll_state.offset.saturating_add(lines).min(max_off);
        if self.scroll_state.offset >= max_off {
            self.scroll_state.stick_to_bottom = true;
        }
    }

    pub fn jump_to_top(&mut self) {
        self.scroll_state.stick_to_bottom = false;
        self.scroll_state.offset = 0;
    }

    pub fn jump_to_bottom(&mut self) {
        self.scroll_state.stick_to_bottom = true;
        self.recompute_offset_for_stick();
    }

    /// TUI-102: jump to an absolute visual-row offset and exit stick mode.
    /// Called by `Action::ScrollbackJumpToOffset` from scrollbar click/drag.
    pub fn jump_to_offset(&mut self, offset: usize) {
        self.scroll_state.stick_to_bottom = false;
        let max_off = self.max_offset_for_viewport();
        self.scroll_state.offset = offset.min(max_off);
    }

    /// RPC-020: drop every chunk and reset scroll state to default.
    pub fn reset(&mut self) {
        self.chunks.clear();
        self.scroll_state = ScrollState::default();
        self.viewport_height = 0;
        self.viewport_width = 0;
        self.clear_selection(); // RPC-381.
        self.reflow_cache.reset(); // BUG-190: drop cached reflected rows.
        self.trimmed_rows_total = 0; // BUG-192: clear the trim counter + marker.
    }

    /// BUG-192: total visual rows across every chunk (the value the
    /// `MAX_SCROLLBACK_VISUAL_ROWS` cap bounds).
    pub fn total_rows(&self) -> usize {
        self.total_visual_rows()
    }

    /// Render the visible window into `area`; returns chunks visited.
    /// RPC-078 fills from the TOP; RPC-094 reserves a 2-col gutter +
    /// scrollbar on overflow. **BUG-190**: chunk rows paint through the
    /// `RowReflowCache` (cached blit when the row's content + width are
    /// unchanged, one-time reflect otherwise) — impl in
    /// `scrollback_render.rs`.
    pub fn render_count_visited(&mut self, area: Rect, buf: &mut Buffer) -> usize {
        render::render_count_visited(self, area, buf)
    }

    /// RPC-094: most-recent layout rect, set inside `render_count_visited`.
    pub fn last_rect(&self) -> Option<Rect> {
        self.last_rect
    }
}

/// Re-derive `chunk.lines` from `chunk.source` for `width`; no-op without `ChunkSource`.
fn rewrap_chunk(chunk: &mut RenderedChunk, width: u16) {
    if let Some(source) = chunk.source.as_ref() {
        chunk.lines = wrap_source(source, width);
    }
}

impl Widget for &mut ScrollbackList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.render_count_visited(area, buf);
    }
}

#[path = "scrollback_copy.rs"]
mod copy;
#[path = "scrollback_render.rs"]
mod render;
#[path = "scrollback_tail.rs"]
mod scrollback_tail;
#[path = "scrollback_select.rs"]
mod select;
#[cfg(test)]
#[path = "scrollback_tests.rs"]
mod tests;
#[path = "scrollback_trim.rs"]
pub mod trim;

pub use trim::{TrimResult, MAX_SCROLLBACK_VISUAL_ROWS};
