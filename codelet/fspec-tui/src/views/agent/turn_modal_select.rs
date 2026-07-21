//! COPY-008 — text selection + copy inside the turn-content modal.
//!
//! Feature: spec/features/turn-content-modal-text-selection-copy.feature
//!
//! Sibling of `mouse_dispatch.rs` (kept in its own module so both stay
//! under the 300-LoC source-shape ceiling). Holds the AgentView methods
//! that: cache the open modal's body layout each frame
//! (`cache_turn_modal_layout`), feed left press/drag/release to the
//! modal's own [`SelectionRecognizer`] in body-local coordinates
//! (`feed_turn_modal_selection`), and — on Commit — reconstruct the
//! gutter-free selected text and emit [`Action::CopyToClipboard`],
//! retaining the highlight (rule [4]).
//!
//! Reuses COPY-002 (`Selection`/`RowSpan`), COPY-003 (recognizer),
//! COPY-004-style char-slicing, and COPY-001 (OSC 52 via the App
//! clipboard) unchanged.

use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::dialog_theme_rows::{fixed_dialog_rect, turn_modal_geometry};
use crate::components::Action;
use crate::mouse::gesture::SelectionGesture;
use crate::mouse::selection::{RowSpan, Selection};
use crate::store::AgentViewStore;

use super::turn_modal::TurnContentModal;
use super::AgentView;

impl AgentView {
    /// COPY-008: cache the modal body layout then paint the modal + its
    /// live selection highlight. Single entry point called from
    /// `render_with_store` so that orchestrator stays under 300 LoC.
    pub(crate) fn paint_turn_modal(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        store: &AgentViewStore,
    ) {
        self.cache_turn_modal_layout(area, store);
        let sel = self.turn_modal_selection.as_ref();
        render_turn_modal(
            area,
            buf,
            self.turn_modal_seq,
            self.turn_modal_offset,
            store,
            sel,
        );
    }

    /// COPY-008: refresh the cached modal body layout (plain-text visual
    /// rows + body-rect origin) from the focused turn's full text. Called
    /// each frame. Clears the cache when the modal is closed.
    fn cache_turn_modal_layout(&mut self, area: Rect, store: &AgentViewStore) {
        let Some(seq) = self.turn_modal_seq else {
            self.turn_modal_rows.clear();
            self.turn_modal_body_origin = None;
            // TUI-103: reset modal scrollbar state when modal closes
            self.turn_modal_scrollbar_drag.reset();
            self.turn_modal_scrollbar_rect = None;
            return;
        };
        let Some(ctx) = store.current_session_context() else {
            return;
        };
        let Some(text) = ctx.scrollback.full_text_for_seq(seq) else {
            return;
        };
        let kind = ctx.scrollback.kind_for_seq(seq);
        let geom = turn_modal_geometry(area, &text);
        let rect = fixed_dialog_rect(area);
        let modal = TurnContentModal::new(text, kind);
        self.turn_modal_rows = modal.plain_rows(geom.content_width);
        // TUI-103: cache scrollbar geometry
        self.turn_modal_total_rows = geom.total_rows;
        self.turn_modal_viewport_rows = geom.viewport_rows;
        // Body content begins at rect.x + 2 (border + padding) and
        // rect.y + 4 (border + padding + title + gap) — the SAME origin
        // `TurnContentModal::render` uses for its scrollbar `bar_area`.
        self.turn_modal_body_origin = Some(Rect {
            x: rect.x + 2,
            y: rect.y + 4,
            width: geom.content_width as u16,
            height: geom.viewport_rows as u16,
        });
        // TUI-103: cache scrollbar gutter rect (rightmost column of body area)
        let show_scrollbar = geom.total_rows > geom.viewport_rows;
        self.turn_modal_scrollbar_rect = if show_scrollbar {
            let body = self.turn_modal_body_origin.unwrap();
            Some(Rect {
                x: body.x + body.width - 1,
                y: body.y,
                width: 1,
                height: body.height,
            })
        } else {
            None
        };
    }

    /// COPY-008: feed a left press/drag/release to the modal selection
    /// recognizer using BODY-local coordinates (subtract the cached body
    /// origin), then apply the resulting gestures (rule [1], [2]).
    pub(super) fn feed_turn_modal_selection(&mut self, ev: MouseEvent) {
        let Some(body) = self.turn_modal_body_origin else {
            return;
        };
        let local = MouseEvent {
            column: ev.column.saturating_sub(body.x),
            row: ev.row.saturating_sub(body.y),
            ..ev
        };
        let gestures = self.recognizer.on_mouse(local, std::time::Instant::now());
        self.apply_turn_modal_gestures(&gestures);
    }

    /// COPY-008/010: translate recognizer gestures into modal-selection
    /// state. Begin anchors PRECISELY at the press cell (so a drag copies
    /// from the press column); BeginLine (long-press) selects the WHOLE
    /// line; Extend overrides the cursor to the drag cell; Commit
    /// reconstructs + copies (rule [4]).
    fn apply_turn_modal_gestures(&mut self, gestures: &[SelectionGesture]) {
        let cw = self.turn_modal_content_width();
        for gesture in gestures {
            match gesture {
                SelectionGesture::Begin(cell) => {
                    // COPY-010: anchor precisely at the pressed cell so a
                    // drag copies from the press column, not line start.
                    self.turn_modal_selection = Some(Selection::collapsed(*cell));
                }
                SelectionGesture::BeginLine(cell) => {
                    // COPY-010: long-press selects the WHOLE line.
                    self.turn_modal_selection = Some(Selection::whole_line(cell.row, cw));
                }
                SelectionGesture::Extend(cell) => {
                    if let Some(sel) = self.turn_modal_selection.as_mut() {
                        sel.cursor = *cell;
                    }
                }
                SelectionGesture::Commit => self.commit_turn_modal_selection(),
                SelectionGesture::Cancel => self.turn_modal_selection = None,
            }
        }
    }

    /// COPY-008 rule [4]: reconstruct the gutter-free selected text and
    /// write it via `Action::CopyToClipboard` (routed to the App's OSC 52
    /// writer). The selection is NOT cleared so the highlight persists.
    fn commit_turn_modal_selection(&mut self) {
        let Some(sel) = self.turn_modal_selection else {
            return;
        };
        let spans = sel.spans(self.turn_modal_content_width());
        let text = self.reconstruct_turn_modal_text(&spans);
        if !text.is_empty() {
            self.emit(Action::CopyToClipboard(text));
        }
    }

    /// COPY-008: gutter-free content width of the cached modal body.
    fn turn_modal_content_width(&self) -> u16 {
        self.turn_modal_body_origin.map(|r| r.width).unwrap_or(0)
    }

    /// COPY-008: reconstruct plain text from viewport-space `spans`,
    /// mapping each viewport row to the cached body row via
    /// `turn_modal_offset` (rule [2] — same windowing the render uses)
    /// and char-slicing to exclude any padding past real content.
    fn reconstruct_turn_modal_text(&self, spans: &[RowSpan]) -> String {
        if spans.is_empty() {
            return String::new();
        }
        let offset = self.turn_modal_offset;
        let rows: Vec<String> = spans
            .iter()
            .map(|span| {
                let idx = offset + span.row as usize;
                let flat = self.turn_modal_rows.get(idx).cloned().unwrap_or_default();
                slice_chars(&flat, span.start_col, span.end_col)
            })
            .collect();
        rows.join("\n")
    }
}

/// Char-slice `text[start_col..min(end_col, char_len)]` on unicode scalar
/// boundaries, with no padding past real content (COPY-004 semantics).
fn slice_chars(text: &str, start_col: u16, end_col: u16) -> String {
    let char_len = text.chars().count();
    let end = (end_col as usize).min(char_len);
    let start = (start_col as usize).min(end);
    text.chars().skip(start).take(end - start).collect()
}

/// RPC-382/383 + COPY-008: paint the turn content modal overlay (if open)
/// for the focused session, then overlay the live selection highlight
/// (REVERSED cells) clamped to the gutter-free content width so the
/// scrollbar column is never painted over.
fn render_turn_modal(
    area: Rect,
    buf: &mut Buffer,
    seq: Option<u64>,
    offset: usize,
    store: &AgentViewStore,
    selection: Option<&Selection>,
) {
    let Some(seq) = seq else { return };
    let Some(ctx) = store.current_session_context() else {
        return;
    };
    let Some(text) = ctx.scrollback.full_text_for_seq(seq) else {
        return;
    };
    let kind = ctx.scrollback.kind_for_seq(seq);
    TurnContentModal::new(text.clone(), kind)
        .with_offset(offset)
        .render(area, buf);
    if let Some(sel) = selection {
        let geom = turn_modal_geometry(area, &text);
        let rect = fixed_dialog_rect(area);
        let body_area = Rect {
            x: rect.x + 2,
            y: rect.y + 4,
            width: geom.content_width as u16,
            height: geom.viewport_rows as u16,
        };
        let spans = sel.spans(geom.content_width as u16);
        crate::views::agent::scrollback_paint::paint_selection_highlight(
            body_area,
            buf,
            &spans,
            geom.content_width as u16,
        );
    }
}
