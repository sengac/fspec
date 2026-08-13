//! RPC-382 — `TurnContentModal`: centered overlay showing one turn's
//! FULL content.
//!
//! Feature: spec/features/agentview-turn-content-modal.feature
//! Feature: spec/features/agentview-turn-content-modal-fullscreen-scroll.feature
//!
//! Opened by pressing Enter on the selected turn while in SELECT mode
//! (RPC-381). Ports the TypeScript `TurnContentModal`
//! (`src/tui/components/TurnContentModal.tsx`): a FIXED full-screen
//! overlay (`terminalWidth-4` × `terminalHeight-6`, centered) titled by
//! the turn's role (colored by its [`ChunkKind`]), whose body is the
//! turn's full `ChunkSource::text` wrapped to the modal's inner width
//! and rendered in a SCROLLABLE viewport.
//!
//! RPC-383 brings parity with the reference:
//!   * fixed full-screen rect (no shrink-to-content) via
//!     `dialog_theme_rows::fixed_dialog_rect` + `render_dialog_at`;
//!   * a scroll offset (`offset`) windows the wrapped body instead of
//!     hard-clipping it — no content is silently dropped;
//!   * the canonical `scrollback_paint::paint_scrollbar` paints the
//!     scrollbar when the body overflows the viewport;
//!   * a dim centered footer `↑↓ Scroll | Esc Close`.
//!
//! The scroll offset itself lives on `AgentView.turn_modal_offset`; the
//! modal is constructed per-frame with that offset via [`with_offset`].

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::dialog_theme::{render_dialog_at, Accent, DialogRow};
use crate::components::dialog_theme_rows::{build_dialog, fixed_dialog_rect, turn_modal_geometry};
use crate::views::agent::scrollback::ScrollState;
use crate::views::agent::scrollback_paint::paint_scrollbar;
use crate::views::agent::ChunkKind;

/// Footer hint mirroring the TS reference (`TurnContentModal.tsx:171`).
pub const MODAL_FOOTER: &str = "\u{2191}\u{2193} Scroll | Esc Close";

/// A read-only modal rendering one turn's full text in a fixed
/// full-screen, scrollable viewport.
pub struct TurnContentModal {
    title: String,
    accent: Accent,
    body: String,
    /// RPC-383: first visible visual row of the wrapped body.
    offset: usize,
    /// RPC-393 (WARNING #4): true only when this turn is an Edit/Write diff
    /// card. Gates ALL diff styling so a plain turn whose body merely looks
    /// line-numbered is never diff-styled.
    is_diff: bool,
}

impl TurnContentModal {
    /// Build a modal for `text`, titled by `kind`'s role. `kind` is
    /// `None` for legacy chunks without a `ChunkSource`; those fall back
    /// to a neutral "Turn" title. The scroll offset starts at 0.
    pub fn new(text: impl Into<String>, kind: Option<ChunkKind>) -> Self {
        let (title, accent) = title_and_accent(kind.as_ref());
        let is_diff = matches!(kind, Some(ChunkKind::ToolCall { is_diff: true, .. }));
        Self {
            title: title.to_string(),
            accent,
            body: text.into(),
            offset: 0,
            is_diff,
        }
    }

    /// RPC-383: set the scroll offset (first visible visual row). The
    /// caller (`AgentView::render_with_store`) passes
    /// `turn_modal_offset`; the render path clamps it so the last page
    /// stays fully visible.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// The role-derived title (e.g. `You` / `Agent`). Exposed for tests.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The full body text. Exposed for tests.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// RPC-383: the largest valid scroll offset for `total_rows` wrapped
    /// rows in a `viewport_rows`-tall body viewport — the last page is
    /// fully visible (offset never exceeds `total - viewport`).
    pub fn max_offset(total_rows: usize, viewport_rows: usize) -> usize {
        total_rows.saturating_sub(viewport_rows)
    }

    /// Render the modal as a FIXED full-screen overlay inside `area`
    /// (`area.width-4` × `area.height-6`, centered). The body is wrapped
    /// to the inner content width and WINDOWED by `self.offset`; a
    /// scrollbar is painted when it overflows, and a dim centered footer
    /// shows the scroll/close hint.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 6 {
            return;
        }
        let rect = fixed_dialog_rect(area);
        // RPC-383: the fixed-rect → inner-width → viewport-rows →
        // overflow-narrowed content_width pipeline is shared with the App
        // scroll reducer (`turn_modal_geometry`) so the painted page and
        // the clamped offset stay in lockstep.
        let geom = turn_modal_geometry(area, &self.body);
        let viewport_rows = geom.viewport_rows;
        let content_width = geom.content_width;
        let all = self.styled_rows(content_width);
        let total_rows = all.len();
        let offset = self.offset.min(Self::max_offset(total_rows, viewport_rows));

        let rows: Vec<DialogRow> = all
            .into_iter()
            .skip(offset)
            .take(viewport_rows)
            .map(|spans| DialogRow {
                spans,
                selectable: false,
                selected: false,
            })
            .collect();

        let dialog = build_dialog(self.accent, &self.title, rows, MODAL_FOOTER, 0);
        render_dialog_at(rect, buf, &dialog);

        // Paint the canonical scrollbar in the body's rightmost column.
        // Body content starts at rect.y + 1 border + 1 padding + 1 title
        // + 1 gap = rect.y + 4 (spacious layout used whenever overflow is
        // possible — the modal is full-screen, so always tall enough).
        if total_rows > viewport_rows {
            let bar_area = Rect {
                x: rect.x + 2,
                y: rect.y + 4,
                width: content_width as u16 + 1,
                height: viewport_rows as u16,
            };
            let state = ScrollState {
                offset,
                stick_to_bottom: false,
            };
            paint_scrollbar(bar_area, buf, viewport_rows, total_rows, state);
        }
    }

    /// RPC-393: wrap the entire body to `width` and style each visual row.
    /// For a diff card every hard line is parsed ONCE and wrapped
    /// continuation-safe (CRITICAL #3); for a non-diff card the body is plain
    /// raw text (WARNING #4 — never diff-styled). Replaces the pre-RPC-393
    /// `wrap_all` + per-row `style_modal_row`; NEVER clips, so scrolling can
    /// reach every row.
    fn styled_rows(&self, width: usize) -> Vec<Vec<ratatui::text::Span<'static>>> {
        use crate::store::agent_view::diff_decode::style_modal_lines;
        let mut rows: Vec<Vec<ratatui::text::Span<'static>>> = Vec::new();
        for hard in self.body.split('\n') {
            rows.extend(style_modal_lines(hard, width, self.is_diff));
        }
        rows
    }

    /// COPY-008: the plain-text of each wrapped visual row at `width`
    /// (span contents concatenated). Shares the exact `styled_rows`
    /// windowing so the copied text agrees with what is painted.
    pub fn plain_rows(&self, width: usize) -> Vec<String> {
        self.styled_rows(width)
            .into_iter()
            .map(|spans| spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect()
    }
}

/// Map a turn's [`ChunkKind`] to its modal title + accent color,
/// mirroring the scrollback role coloring (`chunk_wrap::wrap_source`).
fn title_and_accent(kind: Option<&ChunkKind>) -> (&'static str, Accent) {
    match kind {
        Some(ChunkKind::UserInput) => ("You", Accent::Cyan),
        Some(ChunkKind::AssistantText) => ("Agent", Accent::Cyan),
        Some(ChunkKind::Thinking) => ("Thinking", Accent::Yellow),
        Some(ChunkKind::ToolCall { is_error: true, .. }) => ("Tool (error)", Accent::Red),
        Some(ChunkKind::ToolCall { .. }) => ("Tool", Accent::Cyan),
        Some(ChunkKind::Error) => ("Error", Accent::Red),
        Some(ChunkKind::Interrupted) => ("Interrupted", Accent::Yellow),
        Some(ChunkKind::Notification) => ("Notification", Accent::Cyan),
        Some(ChunkKind::Incoming) => ("Incoming", Accent::Cyan),
        None => ("Turn", Accent::Cyan),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_rows(modal: &TurnContentModal, w: u16, h: u16) -> Vec<String> {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).expect("Terminal::new");
        term.draw(|frame| modal.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        let buf = term.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| {
                let mut row = String::new();
                for x in 0..buf.area.width {
                    row.push_str(buf[(x, y)].symbol());
                }
                row
            })
            .collect()
    }

    #[test]
    fn renders_full_body_text_and_role_title() {
        // RPC-383: full-screen scrollable overlay; use a tall terminal so
        // both short body lines fit at once (RPC-382 intent preserved).
        let modal = TurnContentModal::new("FIRSTLINE\nSECONDLINE", Some(ChunkKind::AssistantText));
        let joined = render_rows(&modal, 40, 20).join("\n");
        assert!(joined.contains("Agent"), "title must show role: {joined}");
        assert!(joined.contains("FIRSTLINE"), "line 1 missing: {joined}");
        assert!(joined.contains("SECONDLINE"), "line 2 missing: {joined}");
    }

    #[test]
    fn full_screen_fixed_rect_and_footer() {
        // RPC-383: area.width-4 × area.height-6, centered, + dim footer.
        let modal = TurnContentModal::new("body", Some(ChunkKind::AssistantText));
        let top: Vec<char> = render_rows(&modal, 40, 12)[3].chars().collect();
        assert_eq!(top.iter().position(|c| *c == '\u{256D}'), Some(2));
        assert_eq!(top.iter().position(|c| *c == '\u{256E}'), Some(37));
        let tall = render_rows(&modal, 40, 20).join("\n");
        assert!(tall.contains(MODAL_FOOTER), "footer missing: {tall}");
    }

    #[test]
    fn long_body_is_scrollable_not_clipped() {
        // RPC-383: every row reachable by scrolling (no silent clip).
        let mut body = String::from("TOPLINE\n");
        for i in 0..80 {
            body.push_str(&format!("L{i:02}\n"));
        }
        body.push_str("LASTLINE");
        let total = body.split('\n').count();
        let top = render_rows(
            &TurnContentModal::new(body.clone(), Some(ChunkKind::AssistantText)),
            40,
            16,
        )
        .join("\n");
        assert!(
            top.contains("TOPLINE") && !top.contains("LASTLINE"),
            "{top}"
        );
        let bot = render_rows(
            &TurnContentModal::new(body, Some(ChunkKind::AssistantText)).with_offset(total),
            40,
            16,
        )
        .join("\n");
        assert!(bot.contains("LASTLINE"), "bottom not reached: {bot}");
    }

    #[test]
    fn user_turn_titles_you() {
        let modal = TurnContentModal::new("hi", Some(ChunkKind::UserInput));
        assert_eq!(modal.title(), "You");
    }
}
