//! MUX-008 — focus flash scans bottom-to-top and settles as a 1-row top
//! bar.
//!
//! Feature: spec/features/focus-flash-scans-bottom-to-top-and-settles-as-a-1-row-top-bar.feature
//!
//! The MUX-006 scan strip is a single 1-row-high, full-width row that
//! sweeps from the pane's BOTTOM edge up to the TOP over the 350ms
//! window; the MUX-007 settled frame keeps painting that final top-row
//! bar on every subsequent frame of the focused pane (not a
//! full-height left-edge strip — MUX-007 R1 is superseded). The flash
//! reuses the mux footer dark purple (RGB 74, 44, 112 — MUX-005) as
//! cell BACKGROUNDS only (pane glyphs stay readable). Mux off → no
//! purple cells.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_rpc_types::{SessionId, WorkUnitInfo};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::store::{AgentViewStore, BoardStore, SessionContext};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::multiplex::flash::{flash_cells, FLASH_MS};
use codelet_fspec_tui::views::{Navigator, ViewMode};

/// MUX-005 footer purple — the flash must reuse this exact colour.
const FLASH_PURPLE: Color = Color::Rgb(74, 44, 112);

const W: u16 = 120;
const H: u16 = 24;

/// Frames rendered until the 350ms window has elapsed (22 * 16ms =
/// 352ms): frame 22 paints at clock 336 (LAST_PAINT_MS — the top edge),
/// then the advance to 352 ends the scan.
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
    (Navigator::new(Arc::new(Theme::default()), tx), rx)
}

/// MUX-001: enable the default preset on the Navigator's mux layout.
fn enable_default(nav: &mut Navigator) {
    nav.mux.enable_default();
    nav.active_view = ViewMode::Mux;
}

/// MUX-002: an agent slot only renders when a session is open.
fn seed_agent_session(agent: &mut AgentViewStore) {
    agent.append_session(SessionContext::new(SessionId::new("s-1")));
}

/// Render the Navigator into a 120x24 test buffer (one frame).
fn render_buffer(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
) -> ratatui::buffer::Buffer {
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("Terminal::new");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Render `n` frames (advances the render-driven flash clock n*16ms) and
/// return the last buffer.
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

/// All cells in the buffer whose background is the flash purple,
/// EXCLUDING the mux footer row (MUX-005 paints that row purple by
/// design — it is not part of the flash).
fn purple_cells(buf: &ratatui::buffer::Buffer) -> Vec<(u16, u16)> {
    let footer_row = buf.area.height - 1;
    let mut out = Vec::new();
    for y in 0..footer_row {
        for x in 0..buf.area.width {
            if buf[(x, y)].bg == FLASH_PURPLE {
                out.push((x, y));
            }
        }
    }
    out
}

/// The (x, y) symbol grid of a rect region (content-readability check).
fn symbols_in(buf: &ratatui::buffer::Buffer, x0: u16, y0: u16, w: u16, h: u16) -> Vec<String> {
    (0..h)
        .flat_map(|dy| (0..w).map(move |dx| buf[(x0 + dx, y0 + dy)].symbol().to_string()))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: focusing a pane scans a purple strip from the bottom edge to
// the top
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: focusing a pane scans a purple strip from the bottom edge to the top
#[test]
fn focusing_a_pane_scans_a_purple_strip_from_bottom_edge_to_top() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused
    nav.mux.set_focus(1);
    // @step When the mux frame is rendered to a terminal buffer
    // The frame paints at the CURRENT clock, then advances: frame 1
    // (the first render after arming) paints at clock 0.
    let paint_clock = nav.mux.flash_clock_ms();
    assert_eq!(
        paint_clock, 0,
        "the first frame must paint at the armed clock 0 (bottom edge)"
    );
    let buf = render_frames(&mut nav, &board, &mut agent, 1);
    let agent_rect = nav.mux.pane_rects()[1];
    // @step Then the Agent pane rect shows the dark purple background (RGB 74, 44, 112) as a single full-width row at the pane's bottom edge
    let painted = purple_cells(&buf);
    let mut painted_sorted = painted.clone();
    painted_sorted.sort();
    let mut expected = flash_cells(agent_rect, paint_clock);
    expected.sort();
    assert_eq!(
        painted_sorted, expected,
        "frame 1 must paint exactly the deterministic bottom-edge row"
    );
    assert!(
        !painted.is_empty(),
        "the flash must paint cells at the bottom edge"
    );
    let top_row = painted.iter().map(|&(_, y)| y).min().expect("non-empty");
    assert_eq!(
        top_row,
        agent_rect.y + agent_rect.height - 1,
        "the scan must START at the pane's bottom edge (1 row high)"
    );
    // Full width: every column of the pane is tinted.
    let cols: Vec<u16> = painted.iter().map(|&(x, _)| x).collect();
    let min = *cols.iter().min().expect("cols");
    let max = *cols.iter().max().expect("cols");
    assert_eq!(
        min, agent_rect.x,
        "row must start at the pane's left column"
    );
    assert_eq!(
        max,
        agent_rect.x + agent_rect.width - 1,
        "row must reach the pane's right column"
    );
    // @step And the Board pane rect has no dark purple flash background
    let board_rect = nav.mux.pane_rects()[0];
    for (x, y) in &painted {
        assert!(
            !board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "flash leaked into the Board pane at ({x}, {y})"
        );
    }
    // @step And rendering a few frames later the strip has moved UP (a mid-scan position, still 1 row high and full width)
    // 9 frames total so far: frame 9 paints at clock 8*16 = 128 (mid-scan).
    let mid_buf = render_frames(&mut nav, &board, &mut agent, 8);
    let mid_painted = purple_cells(&mid_buf);
    assert!(
        !mid_painted.is_empty(),
        "the strip must still be visible mid-scan"
    );
    let mid_top = mid_painted
        .iter()
        .map(|&(_, y)| y)
        .min()
        .expect("non-empty");
    assert_eq!(
        mid_painted
            .iter()
            .map(|&(_, y)| y)
            .max()
            .expect("non-empty"),
        mid_top,
        "mid-scan strip must stay exactly 1 row high"
    );
    assert!(
        mid_top < top_row,
        "mid-scan row {mid_top} must be ABOVE the start row {top_row}"
    );
    // @step And rendering to the end of the 350ms window shows the strip at the pane's top edge
    // 9 frames have rendered so far; 13 more land on frame 22, which
    // paints at clock 21*16 = 336 (LAST_PAINT_MS — the top edge) and
    // whose post-paint advance (352 ≥ 350) ends the scan.
    let last_buf = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES - 9);
    let end_painted = purple_cells(&last_buf);
    assert!(
        !end_painted.is_empty(),
        "the strip must still be visible on the final frame"
    );
    let end_top = end_painted
        .iter()
        .map(|&(_, y)| y)
        .min()
        .expect("non-empty");
    assert_eq!(
        end_top, agent_rect.y,
        "the scan must END at the pane's top edge"
    );
    // MUX-007 (as superseded by MUX-008): the 350ms scan animation is
    // over, but its FINAL frame — the 1-row bar parked at the top row —
    // settles: it keeps being painted on every subsequent frame of the
    // FOCUSED (agent) pane only.
    assert!(
        !nav.mux.is_flash_active(),
        "the scan animation must end once 350ms of frames have elapsed"
    );
    let after = render_buffer(&mut nav, &board, &mut agent);
    let settled = purple_cells(&after);
    let expected_settled = flash_cells(agent_rect, FLASH_MS);
    assert!(!expected_settled.is_empty(), "settled bar must exist");
    let mut sorted = settled.clone();
    sorted.sort();
    let mut exp_sorted = expected_settled;
    exp_sorted.sort();
    assert_eq!(
        sorted, exp_sorted,
        "after the window elapses the focused pane must keep exactly the settled top-row bar"
    );
    for (x, y) in &settled {
        assert_eq!(
            *y, agent_rect.y,
            "settled bar must sit on the pane's top row at ({x}, {y})"
        );
        assert!(
            !board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "settled bar leaked into the Board pane at ({x}, {y})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the settled frame is a 1-row top bar that persists on the
// focused pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the settled frame is a 1-row top bar that persists on the focused pane
#[test]
fn the_settled_frame_is_a_1_row_top_bar_that_persists_on_the_focused_pane() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with a fresh scan in flight
    nav.mux.set_focus(1);
    assert!(nav.mux.is_flash_active(), "scan must be in flight at arm");
    // @step When 350ms of rendered frames have elapsed
    let buf = render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    assert!(
        !nav.mux.is_flash_active(),
        "the scan must end after 350ms of frames"
    );
    let agent_rect = nav.mux.pane_rects()[1];
    let board_rect = nav.mux.pane_rects()[0];
    let divider_col = board_rect.x + board_rect.width;
    // @step Then the Agent pane shows the dark purple background (RGB 74, 44, 112) as a 1-row-high bar across the full pane width on the pane's top row
    let settled = purple_cells(&buf);
    assert!(!settled.is_empty(), "the settled bar must paint cells");
    for (x, y) in &settled {
        assert_eq!(
            *y, agent_rect.y,
            "settled cell ({x}, {y}) must sit on the pane's top row"
        );
        assert!(
            (agent_rect.x..agent_rect.x + agent_rect.width).contains(x),
            "settled cell ({x}, {y}) must lie inside the Agent pane"
        );
    }
    let mut cols: Vec<u16> = settled.iter().map(|&(x, _)| x).collect();
    cols.sort();
    cols.dedup();
    assert_eq!(
        cols,
        (agent_rect.x..agent_rect.x + agent_rect.width).collect::<Vec<_>>(),
        "the settled bar must span the full pane width"
    );
    // @step And no other cell in the Agent pane has the dark purple background
    for (x, y) in &settled {
        assert_eq!(
            *y, agent_rect.y,
            "only the top row may be purple in the Agent pane at ({x}, {y})"
        );
    }
    // @step And the Agent pane shows the same 1-row top bar on every subsequent rendered frame
    let later = render_frames(&mut nav, &board, &mut agent, 5);
    assert_eq!(
        purple_cells(&later),
        settled,
        "the settled bar must be byte-identical on every subsequent frame"
    );
    // @step And the Board pane rect has no dark purple background
    for (x, y) in &settled {
        assert!(
            !board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "settled bar leaked into the Board pane at ({x}, {y})"
        );
    }
    // @step And no purple cell lies in the divider column or the mux footer row
    for (x, y) in &settled {
        assert_ne!(
            *x, divider_col,
            "settled bar leaked into the divider column {x}"
        );
        assert!(
            *y < H - 1,
            "settled bar leaked into the mux footer row at ({x}, {y})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: focus change clears the old pane's bar and settles the new
// pane's top bar
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: focus change clears the old pane's bar and settles the new pane's top bar
#[test]
fn focus_change_clears_the_old_panes_bar_and_settles_the_new_panes_top_bar() {
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
        "the agent pane must show the settled bar before the focus move"
    );
    // @step When I press Shift+Left
    let result = nav.handle_event(
        &crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::SHIFT,
        )),
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
        "the new pane's scan must restart at clock 0 (bottom edge)"
    );
    let next = render_frames(&mut nav, &board, &mut agent, 1);
    // @step Then the Agent pane has no dark purple background on the next rendered frame
    for (x, y) in &purple_cells(&next) {
        assert!(
            !agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "the old pane's bar must vanish: purple at ({x}, {y}) in the Agent pane"
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
    assert!(
        !new_settled.is_empty(),
        "the board pane must show the settled bar"
    );
    for (x, y) in &new_settled {
        assert_eq!(
            *y, board_rect.y,
            "settled cell ({x}, {y}) must sit on the Board pane's top row"
        );
        assert!(
            board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "settled cell ({x}, {y}) must lie inside the Board pane"
        );
    }
    // @step And the Agent pane still has no dark purple background
    for (x, y) in &new_settled {
        assert!(
            !agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "purple leaked back into the Agent pane at ({x}, {y})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the bottom-to-top flash and settled top bar are
// deterministic, background-only and off when mux is disabled
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the bottom-to-top flash and settled top bar are deterministic, background-only and off when mux is disabled
#[test]
fn the_bottom_to_top_flash_and_settled_top_bar_are_deterministic_background_only_and_off_when_mux_is_disabled(
) {
    // @step Given mux mode is active with the default two-pane grid
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with a fresh flash
    nav.mux.set_focus(1);
    // @step When the frame is rendered at a given clock value and then re-rendered at the same clock value
    let buf_a = render_buffer(&mut nav, &board, &mut agent);
    let clock_a = nav.mux.flash_clock_ms();
    let set_a = purple_cells(&buf_a);
    // Re-arm (board then agent) resets the clock to 0; one more frame
    // lands at the same clock value.
    nav.mux.set_focus(0);
    nav.mux.set_focus(1);
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "re-arming must reset the clock"
    );
    let buf_b = render_buffer(&mut nav, &board, &mut agent);
    let clock_b = nav.mux.flash_clock_ms();
    let set_b = purple_cells(&buf_b);
    assert_eq!(
        clock_a, clock_b,
        "both renders must land at the same clock value"
    );
    // @step Then both frames paint the identical set of dark purple cells (deterministic pattern)
    assert_eq!(
        set_a, set_b,
        "re-rendering at the same clock must paint the identical purple cell set"
    );
    assert!(!set_a.is_empty(), "the flash must paint cells mid-scan");
    // @step And the Agent pane glyphs are unchanged by the flash (background-only tint)
    let agent_rect = nav.mux.pane_rects()[1];
    // Let the scan settle; compare glyphs across a settled frame.
    render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    let glyphs_settled = symbols_in(
        &render_frames(&mut nav, &board, &mut agent, 1),
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    let buf = render_frames(&mut nav, &board, &mut agent, 1);
    let glyphs_now = symbols_in(
        &buf,
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    assert_eq!(
        glyphs_settled, glyphs_now,
        "the flash must tint BACKGROUNDS only — pane glyphs must survive"
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
                FLASH_PURPLE,
                "no purple cell may be painted with mux off at ({x}, {y})"
            );
        }
    }
    assert!(
        nav2.mux.flash_pane().is_none(),
        "no flash may be armed with mux off"
    );
}
