//! COPY-006 — scrollback selection recognizer wiring for `AgentView`.
//!
//! Feature: spec/features/agentview-text-selection-copy.feature
//!
//! Extracted from `mouse_dispatch.rs` so that file stays under the
//! 300-LoC source-shape ceiling.

use crossterm::event::MouseEvent;
use ratatui::layout::Rect;

use crate::components::Action;
use crate::mouse::gesture::SelectionGesture;

use super::AgentView;

impl AgentView {
    /// COPY-006: feed a left press/drag/release to the selection
    /// recognizer with scrollback-relative coords (subtract the rect
    /// origin) and fan the resulting gestures onto the action bus.
    pub(super) fn feed_selection_recognizer(&mut self, ev: MouseEvent, rect: Rect) {
        let local = MouseEvent {
            column: ev.column.saturating_sub(rect.x),
            row: ev.row.saturating_sub(rect.y),
            ..ev
        };
        let gestures = self.recognizer.on_mouse(local, std::time::Instant::now());
        self.apply_selection_gestures(&gestures);
    }

    /// COPY-006: poll the recognizer from the run loop's render tick so a
    /// stationary long-press fires its `Begin` gesture (~0.5s).
    pub(crate) fn poll_selection_tick(&mut self) {
        let gestures = self.recognizer.tick(std::time::Instant::now());
        self.apply_selection_gestures(&gestures);
    }

    /// COPY-006: translate recognizer gestures into `Action`s and track
    /// the view-local `text_selection_active` flag (rule [10]). Commit
    /// keeps the flag set so the highlight persists (rule [2]).
    pub(super) fn apply_selection_gestures(&mut self, gestures: &[SelectionGesture]) {
        for gesture in gestures {
            match gesture {
                SelectionGesture::Begin(cell) => {
                    self.text_selection_active = true;
                    self.emit(Action::SelectionBegin(*cell));
                }
                SelectionGesture::BeginLine(cell) => {
                    self.text_selection_active = true;
                    self.emit(Action::SelectionBeginLine(*cell));
                }
                SelectionGesture::Extend(cell) => self.emit(Action::SelectionExtend(*cell)),
                SelectionGesture::Commit => self.emit(Action::SelectionCommit),
                SelectionGesture::Cancel => {
                    self.text_selection_active = false;
                    self.emit(Action::SelectionClear);
                }
            }
        }
        }
}
