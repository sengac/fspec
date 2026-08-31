//! BOARD-022 — WorkUnitSearchDialog: board '/' search modal.
//!
//! Feature: spec/features/board-search-dialog-with-tab-toggled-id-title-description-modes.feature
//!
//! Modeled on `attachment_picker_dialog.rs` (RPC-374): a
//! Priority::Foreground modal pushed onto the Compositor by
//! `App::handle_open_work_unit_search`. Renders via the shared
//! `dialog_theme` renderer (cyan accent, like the AgentView @file
//! popup) so the visual contract lands in one place.
//!
//! Filtering is CLIENT-SIDE over a snapshot of the BoardStore's work
//! units — no RPC surface (see the work unit's architecture notes).
//! `Tab` cycles the search mode (Id → Title → Description → Id) and
//! re-runs the filter with the current query. Enter emits
//! `Action::SelectWorkUnit(id)` and pops the dialog; Esc pops.
//!
//! Split across three files (300-LoC budget): this file (struct +
//! keyboard `handle_event` + `render`),
//! `work_unit_search_dialog_accessors` (constructor + accessors) and
//! `work_unit_search_dialog_mouse` (BUG-162 mouse handling + gutter
//! geometry). The mouse module reads the dialog's `pub(super)` state
//! fields directly.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::WorkUnitInfo;

use super::dialog_theme::{render_dialog_at, Accent, DialogRow, FspecDialog};
use super::dialog_theme_rows::{body_content_rows, fixed_dialog_rect};
use super::list_scrollbar::render_list_scrollbar;
use super::scroll_viewport::{ensure_visible, wrap_index, WheelVelocity};
use super::work_unit_search_dialog_mouse::scrollbar_gutter;
use super::work_unit_search_rows::build_rows as build_dialog_rows;
use super::{Action, Callback, Component, EventResult, Priority};
use crate::mouse::scrollbar_drag::ScrollbarDrag;

/// Canonical id used by `Compositor::remove`.
pub const WORK_UNIT_SEARCH_DIALOG_ID: &str = "work-unit-search-dialog";

const ACCENT: Accent = Accent::Cyan;
const FOOTER: &str = "↑↓ Navigate │ Tab Mode │ Enter Select │ Esc Close";
const MIN_WIDTH: u16 = 45;

// BUG-160: `SearchMode` / `filter_work_units` / `SearchMatch` live in
// `work_unit_search_filter` so this file stays under the 300-LoC budget;
// they are re-exported here so the pre-BUG-160 import paths keep working.
pub use super::work_unit_search_filter::{filter_work_units, SearchMatch, SearchMode};

/// Priority::Foreground modal dialog for searching the board's work
/// units by id / title / description.
///
/// The `pub(super)` fields exist so the sibling
/// `work_unit_search_dialog_mouse` module (BUG-162) can read them
/// without widening the public API.
pub struct WorkUnitSearchDialog {
    pub(super) id: String,
    /// Seeded snapshot of the board's work units (board order).
    pub(super) units: Vec<WorkUnitInfo>,
    pub(super) query: String,
    pub(super) mode: SearchMode,
    /// BUG-160: richer matches (id + mode-aware snippet) in board order.
    /// The selection / scroll math operates on `len()` only.
    pub(super) matches: Vec<SearchMatch>,
    pub(super) selected: usize,
    pub(super) scroll_offset: usize,
    pub(super) last_visible_rows: std::cell::Cell<usize>,
    pub(super) action_tx: Option<UnboundedSender<Action>>,
    /// BUG-162: shared wheel-velocity accumulator (1x–5x ramp).
    pub(super) wheel: WheelVelocity,
    /// BUG-162: scrollbar click-and-drag state machine (TUI-101/103).
    pub(super) scrollbar_drag: ScrollbarDrag,
    /// BUG-162: cached fixed dialog rect from the last render for
    /// mouse hit-testing (stable — BUG-159 fixed frame).
    pub(super) last_dialog_rect: Option<Rect>,
    /// BUG-162: cached scrollbar gutter rect from the last render
    /// (`None` when the matches fit in the visible rows).
    pub(super) last_scrollbar_rect: Option<Rect>,
}

impl WorkUnitSearchDialog {
    fn re_filter(&mut self) {
        self.matches = filter_work_units(&self.units, self.mode, &self.query);
        self.selected = 0;
        self.scroll_offset = 0;
        // BUG-162: a stale wheel ramp or in-flight scrollbar drag must
        // not misfire against the new match list.
        self.reset_mouse_state();
    }

    /// `pub(super)` so the BUG-162 mouse module can reuse the exact
    /// same wrap + ensure-visible math as the keyboard navigation.
    pub(super) fn move_by(&mut self, delta: i32) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = wrap_index(self.selected, delta, self.matches.len());
        let vr = self.visible_rows();
        let total = self.matches.len();
        ensure_visible(&mut self.scroll_offset, self.selected, vr, total);
    }

    /// BUG-160: the snippet width budget for the fixed frame (BUG-159) —
    /// the inner body width minus the marker (2), the id width and the
    /// " - " separator, so a long snippet never widens the frame.
    fn snippet_budget(&self, rect: Rect) -> usize {
        let inner = rect.width.saturating_sub(4).max(1) as usize;
        let id_w = self.matches.first().map(|m| m.id.len()).unwrap_or(0).max(1);
        // marker(2) + id + " - "(3)
        inner.saturating_sub(2 + id_w + 3)
    }

    fn emit(&self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action);
        }
    }

    fn remove_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }
}

impl Component for WorkUnitSearchDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // BUG-162: mouse events (wheel + scrollbar drag) are routed to
        // the shared mouse handler; outside the dialog rect they are
        // Ignored so they bubble to the BoardView behind the modal.
        if let Event::Mouse(m) = event {
            return self.handle_mouse(*m);
        }
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        // BUG-161: modifier-chorded keys are CONSUMED (no-op) so chords
        // like Shift+Right / Shift+? cannot leak through the Compositor to
        // the BoardView or the App-level handler behind the modal.
        if key.modifiers.contains(KeyModifiers::SHIFT)
            || key.modifiers.contains(KeyModifiers::CONTROL)
        {
            return EventResult::consumed();
        }
        match key.code {
            KeyCode::Esc => EventResult::Consumed(Some(self.remove_callback())),
            KeyCode::Tab => {
                self.mode = self.mode.next();
                self.re_filter();
                EventResult::consumed()
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.re_filter();
                EventResult::consumed()
            }
            // '/' is the board's open-dialog key; while the dialog is open
            // it is consumed as a no-op (never re-opens, never edits the
            // query) so the board handler below never sees it either.
            KeyCode::Char('/') => EventResult::consumed(),
            KeyCode::Char(c) => {
                self.query.push(c);
                self.re_filter();
                EventResult::consumed()
            }
            KeyCode::Up => {
                self.move_by(-1);
                EventResult::consumed()
            }
            KeyCode::Down => {
                self.move_by(1);
                EventResult::consumed()
            }
            KeyCode::PageUp => {
                self.move_by(-(self.visible_rows() as i32));
                EventResult::consumed()
            }
            KeyCode::PageDown => {
                self.move_by(self.visible_rows() as i32);
                EventResult::consumed()
            }
            KeyCode::Home => {
                self.selected = 0;
                self.scroll_offset = 0;
                EventResult::consumed()
            }
            KeyCode::End => {
                if !self.matches.is_empty() {
                    self.selected = self.matches.len() - 1;
                    let sel = self.selected;
                    let vr = self.visible_rows();
                    let total = self.matches.len();
                    ensure_visible(&mut self.scroll_offset, sel, vr, total);
                }
                EventResult::consumed()
            }
            KeyCode::Enter => {
                // Zero matches → no-op (dialog stays open).
                if let Some(m) = self.matches.get(self.selected) {
                    self.emit(Action::SelectWorkUnit(m.id.clone()));
                    return EventResult::Consumed(Some(self.remove_callback()));
                }
                EventResult::consumed()
            }
            // BUG-161: the dialog is a true modal — any key it does not
            // explicitly handle is CONSUMED as a no-op so the BoardView
            // behind it stays frozen (previously Ignored, which leaked
            // j/k/h/l, [, ], f, c, d, a, ., ?, Left/Right, ... to the
            // board).
            _ => EventResult::consumed(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // BUG-159: FIXED frame rect (area.width-4 x area.height-6, centered)
        // so the dialog does not re-center as the match list grows/shrinks;
        // only the body rows scroll. The visible-rows window is sized by
        // body_content_rows (the same helper render_dialog_at uses) with the
        // pinned query row reserved, so the scroll window and the painted
        // body always agree.
        let rect = fixed_dialog_rect(area);
        let vr = body_content_rows(rect.height, 1, true).max(1);
        self.last_visible_rows.set(vr);
        // BUG-162: cache the FIXED frame rect (stable since BUG-159) so
        // the next mouse event can hit-test against it.
        self.last_dialog_rect = Some(rect);
        // BUG-160: the snippet budget is derived from the FIXED frame
        // (BUG-159) so a long title/description cannot widen it.
        let budget = self.snippet_budget(rect);
        let rows: Vec<DialogRow> = build_dialog_rows(
            &self.matches,
            &self.query,
            self.selected,
            self.scroll_offset,
            vr,
            budget,
        );
        let dialog = FspecDialog {
            accent: ACCENT,
            title: &format!("Search Work Units [{}]", self.mode.label()),
            rows,
            footer: FOOTER,
            min_width: MIN_WIDTH,
            // BUG-159: the live query is painted on a pinned row under the
            // title (visible at all times while typing).
            query_row: Some(&self.query),
        };
        render_dialog_at(rect, buf, &dialog);
        // BUG-162: overflow → paint the shared proportional scrollbar in
        // the rightmost body column and cache the gutter rect for
        // hit-testing (parity with FileSearchPopup's TUI-103 gutter).
        if self.matches.len() > vr {
            if let Some(gutter) = scrollbar_gutter(rect, vr) {
                self.last_scrollbar_rect = Some(gutter);
                render_list_scrollbar(gutter, buf, self.scroll_offset, vr, self.matches.len());
            } else {
                self.last_scrollbar_rect = None;
            }
        } else {
            self.last_scrollbar_rect = None;
        }
    }
}

// BOARD-022 / BUG-160 unit tests for `filter_work_units` (proptest
// invariants, Description-mode no-description rule, mode cycling) live
// in `work_unit_search_filter::tests` alongside the richer-match types.
