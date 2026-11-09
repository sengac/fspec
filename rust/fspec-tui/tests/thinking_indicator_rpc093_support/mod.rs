//! RPC-093 test support for `thinking_indicator_animation_parity_rpc093.rs`
//! (split out by REFAC-038 to keep the test file under 300 lines).
//!
//! Observation points: the `InputTransitionState` phase machine
//! (`advance`, `transition_on_idle`, `transition_on_running`,
//! `on_printable_key`), the painted input row (`render_input_transition`
//! into a one-row `Buffer`), the cursor predicate
//! (`AgentView::is_cursor_visible_for`) and the run-loop draw guard
//! (`app::tick_should_draw`). TUI-118's busy-composer cursor goes through
//! the TUI-115 App harness (`tui115_support::agent_view_with`).

pub use codelet_fspec_tui::app::tick_should_draw;
pub use codelet_fspec_tui::views::agent::input_transition::{
    render_input_transition, InputTransitionState,
};
pub use codelet_fspec_tui::views::agent::spinner::current_frame_glyph;
pub use codelet_fspec_tui::views::agent::{AgentView, INPUT_PLACEHOLDER_HINT};
pub use codelet_rpc_types::SessionStatus;
use std::error::Error;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::tui115_support::{agent_view_with, row_of};

// Animation timing constants (parity with TS InputTransition.tsx).
pub const INK_FRAME_TIME_MS: u64 = 17;
pub const CHARS_PER_FRAME: usize = 5;
pub const ANIMATION_PHASE_DELAY_MS: u64 = 34;

/// The spinner text the Hiding phase captures at t=240ms.
pub const CAPTURED: &str = "⠸ Thinking... (Esc to stop)";

/// A Hiding phase over [`CAPTURED`].
pub fn hiding(visible_chars: usize, started_at: u64, done: Option<u64>) -> InputTransitionState {
    InputTransitionState::Hiding {
        captured: CAPTURED.to_string(),
        visible_chars,
        started_at,
        hide_completed_at: done,
    }
}

/// A Showing phase revealing `placeholder`.
pub fn showing(placeholder: &str, visible_chars: usize, started_at: u64) -> InputTransitionState {
    InputTransitionState::Showing {
        placeholder: placeholder.to_string(),
        visible_chars,
        started_at,
    }
}

/// Paint `state` into a `width`x1 buffer and return the row, trimmed right.
pub fn render_line(state: &InputTransitionState, width: u16) -> String {
    let area = Rect::new(0, 0, width, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(area, &mut buf, state);
    let mut s = String::new();
    for x in area.x..area.x + area.width {
        s.push_str(buf[(x, area.y)].symbol());
    }
    s.trim_end().to_string()
}

/// The glyph painted at column 0 of the input row.
pub fn first_cell(state: &InputTransitionState) -> String {
    let area = Rect::new(0, 0, 4, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(area, &mut buf, state);
    buf[(0, 0)].symbol().to_string()
}

/// `AgentView::is_cursor_visible_for` for a focused session in `status`.
pub fn cursor_visible(status: SessionStatus, state: &InputTransitionState) -> bool {
    AgentView::is_cursor_visible_for(Some(status), state)
}

/// `visible_chars` of a Hiding state; fails the test on any other phase.
#[track_caller]
pub fn hiding_visible(
    state: &InputTransitionState,
    at: &str,
) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(match state {
        InputTransitionState::Hiding { visible_chars, .. } => *visible_chars,
        other => return Err(format!("expected Hiding at {at}, got {other:?}").into()),
    })
}

/// Phase equality through `Debug` (the state machine has no `PartialEq`).
#[track_caller]
pub fn assert_state(actual: &InputTransitionState, expected: &InputTransitionState) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

/// The RPC-093 source-shape ceiling target.
pub const INPUT_TRANSITION_RS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/views/agent/input_transition.rs"
);

/// The finish-animation predicate the run loop feeds `tick_should_draw`.
pub fn is_animating(state: &InputTransitionState) -> bool {
    matches!(
        state,
        InputTransitionState::Hiding { .. } | InputTransitionState::Showing { .. }
    )
}

/// TUI-118: the cursor state of one 80x24 frame of a busy agent view.
pub struct BusyCursor {
    /// `AgentView::is_cursor_visible`, the gate `App::run` puts on
    /// `frame.set_cursor_position`.
    pub visible: bool,
    /// The row `cursor_position()` hands `frame.set_cursor_position`.
    pub cursor_row: Option<usize>,
    /// The first row containing the activity text.
    pub activity_row: usize,
}

/// Render the agent view of a session in `status` into an 80x24 buffer
/// and read its cursor gate, cursor row and activity row.
pub async fn busy_cursor(
    status: SessionStatus,
    activity: &str,
) -> Result<BusyCursor, Box<dyn Error>> {
    let (mut app, _mock, _session) = agent_view_with(status).await?;
    let mut term = Terminal::new(TestBackend::new(80, 24))?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))?;
    let buffer = term.backend().buffer();
    let rows: Vec<String> = buffer
        .content
        .chunks(usize::from(buffer.area.width))
        .map(|row| row.iter().map(ratatui::buffer::Cell::symbol).collect())
        .collect();
    let agent = &app.navigator().agent;
    Ok(BusyCursor {
        visible: agent.is_cursor_visible(app.agent_view_store(), Some(status)),
        cursor_row: agent.cursor_position().map(|(_, y)| usize::from(y)),
        activity_row: row_of(&rows, activity)?,
    })
}
