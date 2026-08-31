//! MUX-007 — mux focus flash settled final frame: after the 350ms
//! bottom-to-top scan (MUX-006, MUX-008) elapses, the focused pane KEEPS
//! the scan's final frame painted — a 1-ROW-HIGH dark-purple bar
//! (MUX-005 footer purple, RGB 74, 44, 112) across the pane's TOP row
//! (full width) — until focus moves or mux is disabled. The settled bar
//! is repaint content, not an animation: it never keeps the run-loop
//! draw gate open.
//!
//! Feature: spec/features/mux-focus-flash-settled-final-frame-persists-on-the-active-pane.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::app::tick_should_draw;
use codelet_fspec_tui::store::{AgentViewStore, BoardStore, SessionContext};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::multiplex::flash::{flash_cells, LAST_PAINT_MS};
use codelet_fspec_tui::views::{Navigator, ViewMode};

/// MUX-005 footer purple — the settled strip reuses this exact colour.
const SETTLE_PURPLE: Color = Color::Rgb(74, 44, 112);

const W: u16 = 120;
const H: u16 = 24;

/// Frames that cover the 350ms scan window (22 * 16ms = 352ms).
const ELAPSE_FRAMES: u32 = 22;

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn fresh() -> (
    Navigator,
    tokio::sync::mpsc::UnboundedReceiver<codelet_fspec_tui::components::Action>,
) {
    let (tx, rx) = unbounded_channel();
    (
        Navigator::new(std::sync::Arc::new(Theme::default()), tx),
        rx,
    )
}

/// MUX-001: enable the default preset on the Navigator's mux layout.
fn enable_default(nav: &mut Navigator) {
    nav.mux.enable_default();
    nav.active_view = ViewMode::Mux;
}

fn seed_agent_session(agent: &mut AgentViewStore) {
    agent.append_session(SessionContext::new(SessionId::new("s-1")));
}

/// Render `n` frames and return the last buffer (each frame advances the
/// render-driven flash clock by 16ms).
fn render_frames(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
    n: u32,
) -> ratatui::buffer::Buffer {
    let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, W, H));
    for _ in 0..n {
        let mut term = Terminal::new(TestBackend::new(W, H)).expect("Terminal::new");
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
        })
        .expect("draw");
        buf = term.backend().buffer().clone();
    }
    buf
}

/// All cells whose background is the settle purple, EXCLUDING the mux
/// footer row (MUX-005 paints that row purple by design).
fn purple_cells(buf: &ratatui::buffer::Buffer) -> Vec<(u16, u16)> {
    let footer_row = buf.area.height - 1;
    let mut out = Vec::new();
    for y in 0..footer_row {
        for x in 0..buf.area.width {
            if buf[(x, y)].bg == SETTLE_PURPLE {
                out.push((x, y));
            }
        }
    }
    out
}

fn symbols_in(buf: &ratatui::buffer::Buffer, x0: u16, y0: u16, w: u16, h: u16) -> Vec<String> {
    (0..h)
        .flat_map(|dy| (0..w).map(move |dx| buf[(x0 + dx, y0 + dy)].symbol().to_string()))
        .collect()
}

/// Assert the purple cells form a 1-row-high bar across the pane's top
/// row (full pane width).
fn assert_top_row_bar(painted: &[(u16, u16)], pane_x: u16, pane_w: u16, pane_y: u16) {
    assert!(!painted.is_empty(), "the bar must paint cells");
    for (x, y) in painted {
        assert_eq!(
            *y, pane_y,
            "bar cell ({x}, {y}) must sit on the pane's top row"
        );
        assert!(
            (pane_x..pane_x + pane_w).contains(x),
            "bar cell ({x}, {y}) must lie inside the pane"
        );
    }
    let mut cols: Vec<u16> = painted.iter().map(|&(x, _)| x).collect();
    cols.sort();
    cols.dedup();
    assert_eq!(
        cols,
        (pane_x..pane_x + pane_w).collect::<Vec<_>>(),
        "the bar must span the full pane width"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the active pane keeps the settled 1-row top bar after the
// scan ends
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the active pane keeps the settled 1-row top bar after the scan ends
#[test]
fn the_active_pane_keeps_the_settled_1_row_top_bar_after_the_scan_ends() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Board pane is focused with a fresh scan in flight
    assert_eq!(nav.mux.focus(), 0);
    assert_eq!(nav.mux.flash_clock_ms(), 0, "scan must start at clock 0");
    assert!(nav.mux.is_flash_active(), "scan must be in flight");
    // @step When 350ms of rendered frames have elapsed
    let buf = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    assert!(
        !nav.mux.is_flash_active(),
        "the scan must end after 350ms of frames"
    );
    let board_rect = nav.mux.pane_rects()[0];
    let agent_rect = nav.mux.pane_rects()[1];
    // @step Then the Board pane shows the dark purple background (RGB 74, 44, 112) as a 1-row-high bar across the full pane width on the pane's top row
    let settled = purple_cells(&buf);
    assert_top_row_bar(&settled, board_rect.x, board_rect.width, board_rect.y);
    // @step And the Agent pane rect has no dark purple background
    for (x, y) in &settled {
        assert!(
            !agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "settled strip leaked into the Agent pane at ({x}, {y})"
        );
    }
    // @step And rendering more frames later the Board pane still shows the same 1-row top bar
    let later = render_frames(&mut nav, &board, &mut agent, 5);
    assert_eq!(
        purple_cells(&later),
        settled,
        "the settled strip must be byte-identical on every subsequent frame"
    );
    // @step And no other cell in the body has the dark purple background
    for (x, y) in &settled {
        assert!(
            board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "purple body cell ({x}, {y}) lies outside the focused pane"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: focus change clears the old pane's strip and settles the
// new pane's strip
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: focus change clears the old pane's strip and settles the new pane's strip
#[test]
fn focus_change_clears_the_old_strip_and_settles_the_new_strip() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with the settled 1-row top bar
    nav.mux.set_focus(1);
    let settled_buf = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    let agent_rect = nav.mux.pane_rects()[1];
    let board_rect = nav.mux.pane_rects()[0];
    assert!(
        !nav.mux.is_flash_active(),
        "the agent scan must have settled"
    );
    assert!(
        !purple_cells(&settled_buf).is_empty(),
        "the agent pane must show the settled strip before the focus move"
    );
    // @step When I press Shift+Left
    let result = nav.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)),
        &board,
    );
    assert!(result.is_consumed());
    assert_eq!(
        nav.mux.focus(),
        0,
        "Shift+Left must move focus to the Board pane"
    );
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "the new pane's scan must restart at clock 0"
    );
    let next = render_frames(&mut nav, &board, &mut agent, 1);
    // @step Then the Agent pane has no dark purple background on the next rendered frame
    for (x, y) in &purple_cells(&next) {
        assert!(
            !agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "the old pane's strip must vanish: purple at ({x}, {y}) in the Agent pane"
        );
    }
    // @step And the Board pane is focused and its scan is armed at the start of the 350ms window
    assert_eq!(
        nav.mux.flash_pane(),
        Some(0),
        "scan must be armed on the Board pane"
    );
    // @step When 350ms of rendered frames have elapsed
    let done = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    assert!(
        !nav.mux.is_flash_active(),
        "the board scan must have settled"
    );
    // @step Then the Board pane shows the dark purple 1-row top bar across its top row
    let new_settled = purple_cells(&done);
    assert_top_row_bar(&new_settled, board_rect.x, board_rect.width, board_rect.y);
    // @step And the Agent pane still has no dark purple background
    for (x, y) in &new_settled {
        assert!(
            !agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "purple leaked back into the Agent pane at ({x}, {y})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the settled strip keeps pane content readable and never
// paints with mux off
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the settled strip keeps pane content readable and never paints with mux off
#[test]
fn the_settled_strip_keeps_content_readable_and_never_paints_with_mux_off() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with the settled 1-row top bar
    nav.mux.set_focus(1);
    render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    let agent_rect = nav.mux.pane_rects()[1];
    let glyphs_settled = symbols_in(
        &render_frames(&mut nav, &board, &mut agent, 1),
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    // @step When the frame is rendered to a terminal buffer
    let buf = render_frames(&mut nav, &board, &mut agent, 1);
    let painted = purple_cells(&buf);
    // @step Then every dark purple cell lies inside the Agent pane rect
    assert!(
        !painted.is_empty(),
        "the settled strip must still be painted"
    );
    for (x, y) in &painted {
        assert!(
            agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "settled strip leaked outside the Agent pane at ({x}, {y})"
        );
    }
    // @step And the Agent pane glyphs are unchanged by the strip (background-only tint)
    let glyphs_now = symbols_in(
        &buf,
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    assert_eq!(
        glyphs_settled, glyphs_now,
        "the strip must tint BACKGROUNDS only — pane glyphs must survive"
    );
    // @step Given the TUI is in a single-view mode (Board) with mux disabled
    let (mut nav2, _rx2) = fresh();
    assert_eq!(nav2.active_view, ViewMode::Board);
    assert!(!nav2.mux.config().enabled, "mux must start disabled");
    let board2 = BoardStore::default();
    let mut agent2 = AgentViewStore::default();
    // @step When the frame is rendered to a terminal buffer
    let buf2 = render_frames(&mut nav2, &board2, &mut agent2, 1);
    // @step Then no cell in the buffer has the dark purple background
    for y in 0..buf2.area.height {
        for x in 0..buf2.area.width {
            assert_ne!(
                buf2[(x, y)].bg,
                SETTLE_PURPLE,
                "no purple cell may be painted with mux off at ({x}, {y})"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the settled strip is repaint content that does not force
// continuous redrawing
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the settled strip is repaint content that does not force continuous redrawing
#[test]
fn the_settled_strip_is_repaint_content_that_does_not_force_continuous_redrawing() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with the settled 1-row top bar
    nav.mux.set_focus(1);
    let _ = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    let agent_rect = nav.mux.pane_rects()[1];
    // @step When the run-loop draw gate is evaluated with no other animation in flight
    assert!(!nav.mux.is_flash_active(), "the scan must be settled");
    // @step Then tick_should_draw reports false (idle — the settled strip is not an animation)
    assert!(
        !tick_should_draw(false, false, false, false, nav.is_mux_flash_active()),
        "the settled strip must NOT keep the draw gate open"
    );
    // @step And the next user-triggered frame still paints the settled 1-row top bar on the Agent pane
    let buf = render_frames(&mut nav, &board, &mut agent, 1);
    let mut painted = purple_cells(&buf);
    assert_top_row_bar(&painted, agent_rect.x, agent_rect.width, agent_rect.y);
    // Pure-math companion: the painted settled cells equal the pure
    // pattern at the settle boundary (the final scan frame's geometry).
    painted.sort();
    let mut expected_settled = flash_cells(agent_rect, LAST_PAINT_MS);
    expected_settled.sort();
    assert_eq!(
        painted, expected_settled,
        "the settled frame must paint exactly the pure pattern at the settle boundary"
    );
}
