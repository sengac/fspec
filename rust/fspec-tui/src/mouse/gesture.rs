//! Mouse gesture recognizer for text selection (COPY-003).
//!
//! Feature: spec/features/selection-mouse-gesture-recognizer.feature
//!
//! Translates raw crossterm [`MouseEvent`]s plus injected timestamps into
//! high-level [`SelectionGesture`]s (`Begin`/`Extend`/`Commit`/`Cancel`) —
//! never raw mouse events. Supports two entry gestures:
//!   * drag: a left-button Down followed by a Drag begins + extends;
//!   * long-press: a stationary left-button Down held past [`HOLD`] begins
//!     a selection anchored at the press cell (fired from [`tick`]).
//!
//! Both `on_mouse` and `tick` take an injected `now: Instant` so behaviour
//! is deterministic and testable without real time. Mirrors the
//! debounce-timer pattern in [`crate::mouse::toggle`] but for press-hold
//! detection. Consumer is COPY-006.
//!
//! [`tick`]: SelectionRecognizer::tick

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::mouse::selection::Cell;

/// Long-press threshold: a stationary press held at least this long begins
/// a selection at the press cell. Tunable.
const HOLD: Duration = Duration::from_millis(400);

/// A high-level selection gesture emitted by [`SelectionRecognizer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionGesture {
    /// Begin a selection anchored precisely at the given press cell
    /// (drag start). The anchor keeps the real row AND column.
    Begin(Cell),
    /// Begin a whole-line selection at the given cell's row (long-press).
    /// The consumer expands this to the full line under the press.
    BeginLine(Cell),
    /// Extend the active selection to the given cell.
    Extend(Cell),
    /// Commit (finish) the active selection.
    Commit,
    /// Cancel the active selection.
    Cancel,
}

/// Internal recognizer state.
enum State {
    Idle,
    Pressed { cell: Cell, at: Instant },
    Selecting,
}

/// Recognizes drag and long-press selection gestures from mouse events.
pub struct SelectionRecognizer {
    state: State,
}

impl Default for SelectionRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionRecognizer {
    /// Construct a fresh recognizer in the idle state.
    pub fn new() -> Self {
        Self { state: State::Idle }
    }

    /// Feed a raw mouse event (with an injected `now`) and return the
    /// resulting high-level gestures (possibly empty).
    pub fn on_mouse(&mut self, ev: MouseEvent, now: Instant) -> Vec<SelectionGesture> {
        let cell = Cell {
            row: ev.row,
            col: ev.column,
        };
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.state = State::Pressed { cell, at: now };
                Vec::new()
            }
            MouseEventKind::Drag(MouseButton::Left) => match self.state {
                State::Pressed {
                    cell: press_cell, ..
                } => {
                    self.state = State::Selecting;
                    vec![
                        SelectionGesture::Begin(press_cell),
                        SelectionGesture::Extend(cell),
                    ]
                }
                State::Selecting => vec![SelectionGesture::Extend(cell)],
                State::Idle => Vec::new(),
            },
            MouseEventKind::Up(MouseButton::Left) => {
                let prev = std::mem::replace(&mut self.state, State::Idle);
                match prev {
                    // Drag or long-press produced an active selection: commit it.
                    State::Selecting => vec![SelectionGesture::Commit],
                    // A quick click (press then release, no drag/long-press):
                    // cancel any selection that was active before the click.
                    State::Pressed { .. } => vec![SelectionGesture::Cancel],
                    State::Idle => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Poll the recognizer with an injected `now`. Fires `BeginLine`
    /// exactly once when a stationary press has been held for at least
    /// [`HOLD`] — a long-press selects the WHOLE line under the press.
    pub fn tick(&mut self, now: Instant) -> Vec<SelectionGesture> {
        if let State::Pressed { cell, at } = self.state {
            if now.duration_since(at) >= HOLD {
                self.state = State::Selecting;
                return vec![SelectionGesture::BeginLine(cell)];
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/selection-mouse-gesture-recognizer.feature
    use super::*;
    use crossterm::event::KeyModifiers;

    fn ev(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    #[test]
    fn drag_begins_and_extends_a_selection() {
        // @step Given a fresh selection recognizer
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();

        // @step When the left button is pressed at row 5 column 3 at time 0ms
        let down = rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);
        assert_eq!(down, vec![]);

        // @step And the mouse is dragged to row 5 column 8 at time 30ms
        let drag = rec.on_mouse(
            ev(MouseEventKind::Drag(MouseButton::Left), 8, 5),
            base + Duration::from_millis(30),
        );

        // @step Then the recognizer emits Begin at row 5 column 3 then Extend at row 5 column 8
        assert_eq!(
            drag,
            vec![
                SelectionGesture::Begin(Cell { row: 5, col: 3 }),
                SelectionGesture::Extend(Cell { row: 5, col: 8 }),
            ]
        );
    }

    #[test]
    fn a_stationary_long_press_begins_a_selection_at_the_press_cell() {
        // @step Given a fresh selection recognizer
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();

        // @step When the left button is pressed at row 5 column 3 at time 0ms
        rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);

        // @step And the recognizer is ticked at time 500ms
        let ticked = rec.tick(base + Duration::from_millis(500));

        // @step Then the recognizer emits Begin at row 5 column 3
        assert_eq!(
            ticked,
            vec![SelectionGesture::BeginLine(Cell { row: 5, col: 3 })]
        );
    }

    #[test]
    fn a_quick_click_cancels_any_active_selection() {
        // @step Given a fresh selection recognizer
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();

        // @step When the left button is pressed at row 5 column 3 at time 0ms
        rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);

        // @step And the left button is released at row 5 column 3 at time 100ms with no drag in between
        let up = rec.on_mouse(
            ev(MouseEventKind::Up(MouseButton::Left), 3, 5),
            base + Duration::from_millis(100),
        );

        // @step Then the recognizer emits Cancel
        assert_eq!(up, vec![SelectionGesture::Cancel]);
    }

    #[test]
    fn releasing_an_active_selection_commits_it() {
        // @step Given a recognizer with an active drag selection
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();
        rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);
        rec.on_mouse(
            ev(MouseEventKind::Drag(MouseButton::Left), 8, 5),
            base + Duration::from_millis(30),
        );

        // @step When the left button is released
        let up = rec.on_mouse(
            ev(MouseEventKind::Up(MouseButton::Left), 8, 5),
            base + Duration::from_millis(60),
        );

        // @step Then the recognizer emits Commit
        assert_eq!(up, vec![SelectionGesture::Commit]);
    }

    #[test]
    fn wheel_events_are_ignored_during_an_active_selection() {
        // @step Given a recognizer with an active drag selection
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();
        rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);
        rec.on_mouse(
            ev(MouseEventKind::Drag(MouseButton::Left), 8, 5),
            base + Duration::from_millis(30),
        );

        // @step When a mouse wheel scroll-up event arrives
        let wheel = rec.on_mouse(
            ev(MouseEventKind::ScrollUp, 8, 5),
            base + Duration::from_millis(60),
        );

        // @step Then the recognizer emits no selection gesture
        assert_eq!(wheel, vec![]);
    }

    #[test]
    fn long_press_then_drag_begins_extends_and_commits() {
        // @step Given a fresh selection recognizer
        let mut rec = SelectionRecognizer::new();
        let base = Instant::now();

        // @step When the left button is pressed at row 5 column 3 at time 0ms
        rec.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 3, 5), base);

        // @step And the recognizer is ticked at time 500ms
        let ticked = rec.tick(base + Duration::from_millis(500));

        // @step And the mouse is dragged to row 7 column 2
        let drag = rec.on_mouse(
            ev(MouseEventKind::Drag(MouseButton::Left), 2, 7),
            base + Duration::from_millis(530),
        );

        // @step And the left button is released
        let up = rec.on_mouse(
            ev(MouseEventKind::Up(MouseButton::Left), 2, 7),
            base + Duration::from_millis(560),
        );

        // @step Then the recognizer emits Begin at row 5 column 3, then Extend at row 7 column 2, then Commit
        assert_eq!(
            ticked,
            vec![SelectionGesture::BeginLine(Cell { row: 5, col: 3 })]
        );
        assert_eq!(
            drag,
            vec![SelectionGesture::Extend(Cell { row: 7, col: 2 })]
        );
        assert_eq!(up, vec![SelectionGesture::Commit]);
    }
}
