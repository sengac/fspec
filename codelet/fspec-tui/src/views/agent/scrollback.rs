//! ScrollbackList — windowed VirtualList-style scrollback widget for
//! AgentView (RPC-019, RPC-078, RPC-094).
//!
//! Feature: spec/features/rpc019-scrollback.feature
//!          spec/features/agentview-scrollback-wrap.feature
//!          spec/features/rpc094-agentview-scrollback-scroll.feature
//!
//! Tracks a [`ScrollState`] (offset + `stick_to_bottom`) and paints
//! only the slice of chunks that fit in the visible viewport. RPC-078:
//! `offset` is expressed in VISUAL ROWS (wrapped Lines) so a multi-row
//! chunk cannot push the most-recent line out of view.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::scrollback_paint::paint_scrollbar;
use super::RenderedChunk;
use crate::store::agent_view::chunk_wrap::wrap_source;

/// Scroll state for `ScrollbackList`. `stick_to_bottom = true` is the
/// chat-log default; PageUp drops out, End/PageDown re-enters.
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

/// Windowed scrollback panel. Per-frame work is O(viewport_height).
#[derive(Debug, Default)]
pub struct ScrollbackList {
    chunks: Vec<RenderedChunk>,
    scroll_state: ScrollState,
    /// Latest observed viewport height (rows).
    viewport_height: u16,
    /// Latest observed viewport width (cols). Drives per-chunk re-wrap
    /// from `ChunkSource::text` on resize (RPC-078).
    viewport_width: u16,
    /// RPC-094: cached layout rect from `render_count_visited`.
    /// `mouse_dispatch::handle_scrollback_mouse` hit-tests this.
    last_rect: Option<Rect>,
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

    /// Insert at `idx`, shifting subsequent chunks right. Mirrors
    /// [`Vec::insert`]. **RPC-093**: used by
    /// `chunk_processor::append_thinking` to splice a new thinking
    /// block BEFORE an in-flight assistant chunk.
    pub fn insert(&mut self, idx: usize, chunk: RenderedChunk) {
        self.chunks.insert(idx, chunk);
        self.rewrap_after_mutation(idx);
    }

    /// Shared post-mutation hook for `push` / `insert`: rewrap the
    /// touched chunk if viewport width is known, then re-anchor
    /// stick-to-bottom.
    fn rewrap_after_mutation(&mut self, idx: usize) {
        if self.viewport_width != 0 {
            if let Some(c) = self.chunks.get_mut(idx) {
                rewrap_chunk(c, self.viewport_width);
            }
        }
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Read-only access to the underlying chunks vector. **RPC-091**.
    pub fn chunks(&self) -> &[RenderedChunk] {
        &self.chunks
    }

    /// Mutable access to the underlying chunks vector. **RPC-091**:
    /// in-place accumulation of streaming Text + ToolResult attach.
    pub fn chunks_mut(&mut self) -> &mut Vec<RenderedChunk> {
        &mut self.chunks
    }

    /// Re-wrap a single chunk at index `i` (or 80 cols pre-render).
    /// **RPC-091**.
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

    /// Update cached viewport height. Idempotent.
    pub fn set_viewport_height(&mut self, h: u16) {
        if self.viewport_height != h {
            self.viewport_height = h;
            if self.scroll_state.stick_to_bottom {
                self.recompute_offset_for_stick();
            }
        }
    }

    /// Update cached viewport width. Re-wraps every chunk that carries
    /// a `ChunkSource` so resize never permanently truncates.
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

    /// RPC-020: drop every chunk and reset scroll state to default.
    pub fn reset(&mut self) {
        self.chunks.clear();
        self.scroll_state = ScrollState::default();
        self.viewport_height = 0;
        self.viewport_width = 0;
    }

    /// Render the visible window into `area`. Returns the number of
    /// chunks visited during layout. RPC-078: scrollback fills from
    /// the TOP; stick-to-bottom only kicks in when content overflows.
    /// RPC-094 + TS parity (`src/tui/components/VirtualList.tsx`
    /// lines 17-88): when content overflows we reserve a 2-col gutter
    /// (1-col gap + 1-col scrollbar), rewrap content narrower, then
    /// paint the scrollbar via [`paint_scrollbar`].
    pub fn render_count_visited(&mut self, area: Rect, buf: &mut Buffer) -> usize {
        // Pass 1: wrap at full width to detect overflow.
        self.set_viewport_width(area.width);
        self.set_viewport_height(area.height);
        self.last_rect = Some(area);
        if area.width == 0 || area.height == 0 || self.chunks.is_empty() {
            return 0;
        }
        let vh = area.height as usize;
        // Pass 2: when overflowing AND the terminal is at least 4 cols
        // wide, reserve a 2-col gutter (1 gap + 1 scrollbar) and rewrap.
        let reserve_gutter = self.total_visual_rows() > vh && area.width >= 4;
        let content_width = if reserve_gutter {
            area.width - 2
        } else {
            area.width
        };
        if reserve_gutter {
            self.set_viewport_width(content_width);
        }
        let total_rows = self.total_visual_rows();
        let skip_rows = if self.scroll_state.stick_to_bottom {
            total_rows.saturating_sub(vh)
        } else {
            self.scroll_state.offset
        };
        let visited = super::scrollback_paint::paint_chunk_rows(
            area,
            buf,
            &self.chunks,
            content_width,
            skip_rows,
        );
        if reserve_gutter && total_rows > vh {
            paint_scrollbar(area, buf, vh, total_rows, self.scroll_state);
        }
        visited
    }

    /// Sum of `chunk.lines.len()` across every chunk — the total
    /// visible row count once everything is unfurled.
    pub(crate) fn total_visual_rows(&self) -> usize {
        self.chunks.iter().map(|c| c.lines.len()).sum()
    }

    fn max_offset_for_viewport(&self) -> usize {
        let total = self.total_visual_rows();
        let vh = self.viewport_height as usize;
        if vh == 0 || total <= vh {
            0
        } else {
            total.saturating_sub(vh)
        }
    }

    fn recompute_offset_for_stick(&mut self) {
        self.scroll_state.offset = self.max_offset_for_viewport();
    }

    /// RPC-094: most-recent layout rect, set inside `render_count_visited`.
    pub fn last_rect(&self) -> Option<Rect> {
        self.last_rect
    }
}

/// Re-derive `chunk.lines` from `chunk.source` for the given width.
/// No-op when the chunk has no `ChunkSource`.
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

#[cfg(test)]
#[path = "scrollback_tests.rs"]
mod tests;
