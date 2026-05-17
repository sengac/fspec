//! ScrollbackList — windowed VirtualList-style scrollback widget for
//! AgentView (RPC-019).
//!
//! Feature: spec/features/rpc019-scrollback.feature
//!
//! Replaces the flat `Vec<RenderedChunk>` + manual scroll math that
//! lived inline in `views/agent.rs` until RPC-018. The new widget
//! tracks a [`ScrollState`] (offset + `stick_to_bottom`) and paints
//! only the slice of chunks that fit in the visible viewport — total
//! chunk count does NOT affect per-frame work.
//!
//! Mirrors the consumer semantics of `src/tui/components/VirtualList.tsx`
//! (the TS Ink widget that backs AgentView's scrollback) but stays
//! deliberately small — the lazy / group / scroll-mode / velocity
//! features of the TS widget are deferred to a later RPC slice.
//!
//! Owned by AgentView (single-task mutation per RPC-009 tenere): all
//! `push` / `scroll_*` calls happen on the App task.
//!
//! ```text
//! ┌────────── ScrollbackList ───────────┐
//! │ chunk 0 ... line 0                  │  ← offset = first visible chunk idx
//! │ chunk 0 ... line 1                  │
//! │ chunk 1 ... line 0                  │
//! │ ...                                 │  height = viewport_height
//! └─────────────────────────────────────┘
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};

use super::RenderedChunk;

/// Scroll state extracted from `views/agent.rs` so the widget owns
/// both fields together. `stick_to_bottom = true` is the natural
/// chat-log default — new chunks auto-scroll the viewport so the
/// latest content stays visible. Pressing PageUp drops out of stick
/// mode; pressing PageDown / End re-enters it once the offset catches
/// the tail.
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    /// First visible chunk index (in chunks, not lines).
    pub offset: usize,
    /// When true, `push` auto-bumps `offset` so the latest chunk
    /// stays in view, and rendering snaps to the bottom.
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

/// Windowed scrollback panel.
///
/// Owns a `Vec<RenderedChunk>` (lines pre-rendered as
/// [`ratatui::text::Line`] objects). The `render` algorithm walks at
/// most `viewport_height` chunks per frame; the total chunk count
/// does not bound the work.
#[derive(Debug, Default)]
pub struct ScrollbackList {
    chunks: Vec<RenderedChunk>,
    scroll_state: ScrollState,
    /// Latest observed viewport height (in rows). Updated each
    /// `render` call and by callers that want to keep stick-mode
    /// arithmetic correct before the first paint (`set_viewport_height`).
    viewport_height: u16,
}

impl ScrollbackList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a chunk. When `stick_to_bottom` is true the offset
    /// auto-advances so the latest chunk stays in view (assuming the
    /// caller has set a `viewport_height` ≥ 1).
    pub fn push(&mut self, chunk: RenderedChunk) {
        self.chunks.push(chunk);
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Visible window from `offset` outward, capped by `viewport_lines`.
    /// Used by tests + by the AgentView render path.
    pub fn visible_window(&self, viewport_lines: u16) -> Vec<RenderedChunk> {
        let off = self.scroll_state.offset;
        let mut out = Vec::new();
        let mut rows_used: u16 = 0;
        for chunk in self.chunks.iter().skip(off) {
            if rows_used >= viewport_lines {
                break;
            }
            out.push(chunk.clone());
            rows_used = rows_used.saturating_add(chunk.lines.len() as u16);
        }
        out
    }

    /// Public read-only handle on the scroll cursor — used by the
    /// AgentView render path (to know whether to print a "scrolled-up"
    /// hint) and by tests.
    pub fn scroll_state(&self) -> ScrollState {
        self.scroll_state
    }

    /// Update the cached viewport height. Idempotent. Called both by
    /// `render` (with `area.height`) and by tests / callers that need
    /// to seed the stick-mode arithmetic before the first paint.
    pub fn set_viewport_height(&mut self, h: u16) {
        if self.viewport_height != h {
            self.viewport_height = h;
            if self.scroll_state.stick_to_bottom {
                self.recompute_offset_for_stick();
            }
        }
    }

    /// PageUp: drop out of stick mode and step `offset` back by exactly
    /// `lines` (capped at 0).
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_state.stick_to_bottom = false;
        self.scroll_state.offset = self.scroll_state.offset.saturating_sub(lines);
    }

    /// PageDown / End: step `offset` forward by `lines`, capped at the
    /// tail. Reaching the tail re-enters stick mode.
    pub fn scroll_down(&mut self, lines: usize) {
        let max_off = self.max_offset_for_viewport();
        self.scroll_state.offset = self
            .scroll_state
            .offset
            .saturating_add(lines)
            .min(max_off);
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

    /// RPC-020: drop every chunk and reset the scroll state to its
    /// default (offset=0, stick_to_bottom=true). Called by App::dispatch
    /// on `SlashCommandSelected(Clear)`.
    pub fn reset(&mut self) {
        self.chunks.clear();
        self.scroll_state = ScrollState::default();
        self.viewport_height = 0;
    }

    /// Render the visible window into `area`. Returns the number of
    /// chunks visited during layout (exposed for the source-shape
    /// "render only lays out the visible window" assertion).
    pub fn render_count_visited(&mut self, area: Rect, buf: &mut Buffer) -> usize {
        self.set_viewport_height(area.height);
        if area.width == 0 || area.height == 0 || self.chunks.is_empty() {
            return 0;
        }
        let off = self.scroll_state.offset;
        let mut y = area.y;
        let mut visited = 0usize;
        for chunk in self.chunks.iter().skip(off) {
            if y >= area.y.saturating_add(area.height) {
                break;
            }
            visited = visited.saturating_add(1);
            for line in &chunk.lines {
                if y >= area.y.saturating_add(area.height) {
                    break;
                }
                let row = Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                };
                Paragraph::new(line.clone()).render(row, buf);
                y = y.saturating_add(1);
            }
        }
        visited
    }

    fn max_offset_for_viewport(&self) -> usize {
        // When the total number of chunks (treated as 1-line each — line
        // multi-row handling is a future RPC slice's concern) fits in
        // the viewport, the offset stays at 0.
        let total = self.chunks.len();
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
}

impl Widget for &mut ScrollbackList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.render_count_visited(area, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::text::{Line, Span};

    fn chunk(seq: u64, body: &str) -> RenderedChunk {
        RenderedChunk {
            seq,
            lines: vec![Line::from(Span::raw(body.to_string()))],
        }
    }

    #[test]
    fn default_state_is_offset_zero_stick_true() {
        let s = ScrollState::default();
        assert_eq!(s.offset, 0);
        assert!(s.stick_to_bottom);
    }

    #[test]
    fn push_with_stick_mode_keeps_latest_chunks_visible() {
        let mut list = ScrollbackList::new();
        list.set_viewport_height(3);
        for i in 0..10 {
            list.push(chunk(i, &format!("c{i}")));
        }
        assert!(list.scroll_state().stick_to_bottom);
        assert_eq!(list.scroll_state().offset, 7);
    }

    #[test]
    fn scroll_up_disables_stick_and_caps_at_zero() {
        let mut list = ScrollbackList::new();
        list.set_viewport_height(3);
        for i in 0..10 {
            list.push(chunk(i, "x"));
        }
        list.scroll_up(2);
        assert_eq!(list.scroll_state().offset, 5);
        assert!(!list.scroll_state().stick_to_bottom);
        list.scroll_up(100);
        assert_eq!(list.scroll_state().offset, 0);
    }

    #[test]
    fn scroll_down_caps_at_max_and_re_enables_stick() {
        let mut list = ScrollbackList::new();
        list.set_viewport_height(3);
        for i in 0..10 {
            list.push(chunk(i, "x"));
        }
        list.scroll_up(5);
        list.scroll_down(5);
        assert_eq!(list.scroll_state().offset, 7);
        assert!(list.scroll_state().stick_to_bottom);
    }
}
