//! MUX-006 — mux focus flash: a 350ms purple background scan (1-row-high
//! full-width strip, bottom-to-top — MUX-008) painted over the ENTIRE
//! newly-selected pane.
//!
//! Feature: spec/features/mux-focus-flash-animation.feature
//!
//! The flash uses the mux footer dark purple (RGB 74, 44, 112 — MUX-005)
//! as cell BACKGROUNDS only (pane glyphs stay readable). It is a SINGLE
//! animation (no burst ring, no rain): a full-width 1-row-high scan
//! strip sweeping from the pane's BOTTOM edge to the TOP over 350ms. It
//! is re-armed on every focus change while mux is enabled, advances
//! through a render-driven clock (+16ms per rendered mux frame — the
//! flash paints at the current clock then advances, so frame 1 shows the
//! bottom edge and frame 22 the top edge), and keeps the run-loop draw
//! gate (`tick_should_draw`, 5th operand) open for its 350ms window.
//! Mux off → no purple cells, gate operand false.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use proptest::prelude::*;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::app::tick_should_draw;
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::store::{AgentViewStore, BoardStore, SessionContext};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::multiplex::flash::{flash_cells, FLASH_MS};
use codelet_fspec_tui::views::{Navigator, ViewMode};

/// MUX-005 footer purple — the flash must reuse this exact colour.
const FLASH_PURPLE: Color = Color::Rgb(74, 44, 112);

const W: u16 = 120;
const H: u16 = 24;

/// Frames rendered until the 350ms window has elapsed (22 * 16ms = 352ms):
/// frame 22 paints at clock 336 (top edge), then the advance to 352 ends
/// the flash.
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

fn fresh() -> (Navigator, tokio::sync::mpsc::UnboundedReceiver<Action>) {
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
/// design — it is not part of the MUX-006 flash).
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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
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
    // @step Then the Agent pane rect shows the dark purple background (RGB 74, 44, 112) as a full-width 1-row strip at the pane's bottom edge
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
    let start_y = painted.iter().map(|&(_, y)| y).min().expect("non-empty");
    assert_eq!(
        start_y,
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
        "the strip must stay exactly 1 row high mid-scan"
    );
    assert!(
        mid_top < start_y,
        "mid-scan row {mid_top} must be ABOVE the start row {start_y}"
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
    // MUX-007/MUX-008: the 350ms scan animation is over, but its FINAL
    // frame — the 1-row bar parked at the top row — settles: it keeps
    // being painted on every subsequent frame of the FOCUSED (agent)
    // pane only.
    assert!(
        !nav.mux.is_flash_active(),
        "the scan animation must end once 350ms of frames have elapsed"
    );
    let after = render_buffer(&mut nav, &board, &mut agent);
    // @step And after 350ms of rendered frames the scan animation has ended (the focused pane keeps the settled 1-row top bar — MUX-007 R1, MUX-008 R2)
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
// Scenario: the flash stays inside the focused pane and keeps pane
// content readable
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the flash stays inside the focused pane and keeps pane content readable
#[test]
fn the_flash_stays_inside_the_focused_pane_and_keeps_content_readable() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused
    nav.mux.set_focus(1);
    // The rects cache is empty until the first render — render once to
    // populate it (frame 1, clock 0).
    let buf1 = render_buffer(&mut nav, &board, &mut agent);
    let agent_rect = nav.mux.pane_rects()[1];
    let board_rect = nav.mux.pane_rects()[0];
    let divider_col = board_rect.x + board_rect.width;
    // @step When the mux frame is rendered to a terminal buffer during the flash
    assert!(nav.mux.is_flash_active(), "flash must be active mid-window");
    let symbols1 = symbols_in(
        &buf1,
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    let buf2 = render_buffer(&mut nav, &board, &mut agent);
    // @step Then every flash-purple cell lies inside the Agent pane rect
    let painted = purple_cells(&buf2);
    assert!(
        !painted.is_empty(),
        "the flash must be painting during the window"
    );
    for (x, y) in &painted {
        assert!(
            agent_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "flash purple cell ({x}, {y}) lies outside the Agent pane rect"
        );
    }
    // @step And no flash-purple cell lies in the divider column, the Board pane rect, or the mux footer row
    for (x, y) in &painted {
        assert_ne!(*x, divider_col, "flash leaked into the divider column {x}");
        assert!(
            !board_rect.contains(ratatui::layout::Position { x: *x, y: *y }),
            "flash leaked into the Board pane at ({x}, {y})"
        );
        assert!(
            *y < H - 1,
            "flash leaked into the mux footer row at ({x}, {y})"
        );
    }
    // @step And the Agent pane header, scrollback text and input placeholder glyphs are unchanged by the flash (background-only tint)
    let symbols2 = symbols_in(
        &buf2,
        agent_rect.x,
        agent_rect.y,
        agent_rect.width,
        agent_rect.height,
    );
    assert_eq!(
        symbols1, symbols2,
        "the flash must tint BACKGROUNDS only — pane glyphs must survive"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right, click-to-focus, Enter on a work unit and
// BackToBoard each re-arm the flash on the newly focused pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right, click-to-focus, Enter on a work unit and BackToBoard each re-arm the flash on the newly focused pane
#[test]
fn every_focus_change_path_re_arms_the_flash_on_the_newly_focused_pane() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, mut rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Board pane is focused with work unit AUTH-001 selected
    assert_eq!(nav.mux.focus(), 0);
    // @step When I press Shift+Right
    let result = nav.handle_event(&key(KeyCode::Right, KeyModifiers::SHIFT), &board);
    // @step Then the Agent pane is focused and its flash is armed at the start of the 350ms window
    assert!(result.is_consumed());
    assert_eq!(
        nav.mux.focus(),
        1,
        "Shift+Right must move focus to the agent pane"
    );
    assert_eq!(
        nav.mux.flash_pane(),
        Some(1),
        "flash must be armed on the agent pane"
    );
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "flash clock must restart at 0 (bottom edge)"
    );
    // @step When I click inside the Board pane rect
    let board_rect = {
        let buf = render_buffer(&mut nav, &board, &mut agent);
        let _ = buf;
        nav.mux.pane_rects()[0]
    };
    let result = nav.handle_event(&click(board_rect.x + 5, board_rect.y + 5), &board);
    // @step Then the Board pane is focused and its flash is armed
    assert!(result.is_consumed());
    assert_eq!(nav.mux.focus(), 0, "clicking the board pane must focus it");
    assert_eq!(
        nav.mux.flash_pane(),
        Some(0),
        "flash must be armed on the board pane"
    );
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "flash clock must restart at 0 (bottom edge)"
    );
    // @step When I press Enter on the selected work unit
    let result = nav.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE), &board);
    assert!(
        result.is_consumed(),
        "Enter on the focused board pane must be consumed"
    );
    // @step Then the Agent pane is focused and its flash is re-armed (restarted at the bottom edge)
    let action = rx.try_recv().ok();
    assert!(
        matches!(
            action,
            Some(Action::MuxEnterWorkUnit(ref id)) if *id == "AUTH-001"
        ),
        "Enter on a board work unit in mux mode must emit MuxEnterWorkUnit(AUTH-001), got {action:?}"
    );
    nav.apply_action(&Action::MuxEnterWorkUnit("AUTH-001".to_string()));
    // @step Then the Agent pane is focused and its flash is re-armed (restarted at the bottom edge)
    assert_eq!(
        nav.mux.focus(),
        1,
        "MuxEnterWorkUnit must focus the agent pane"
    );
    assert_eq!(
        nav.mux.flash_pane(),
        Some(1),
        "flash must re-arm on the agent pane"
    );
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "flash clock must restart at 0 (bottom edge)"
    );
    // @step When BackToBoard lands
    nav.apply_action(&Action::BackToBoard);
    // @step Then the Board pane is focused within the grid and its flash is armed
    assert_eq!(
        nav.mux.focus(),
        0,
        "BackToBoard must focus the board pane in-grid"
    );
    assert_eq!(
        nav.mux.flash_pane(),
        Some(0),
        "flash must re-arm on the board pane"
    );
    assert_eq!(
        nav.mux.flash_clock_ms(),
        0,
        "flash clock must restart at 0 (bottom edge)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the flash keeps the 16ms render tick redrawing while idle
// and stops after the window elapses
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the flash keeps the 16ms render tick redrawing while idle and stops after the window elapses
#[test]
fn the_flash_keeps_the_render_tick_redrawing_while_idle_and_stops_after_the_window() {
    // @step Given mux mode is active with the default two-pane grid and the session is idle
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step And the Agent pane is focused with a fresh flash
    nav.mux.set_focus(1);
    assert!(nav.mux.is_flash_active());
    assert!(
        nav.is_mux_flash_active(),
        "Navigator must report the mux flash as active"
    );
    // @step When the run-loop draw gate is evaluated with no other animation in flight
    // @step Then tick_should_draw reports true because the mux flash is active
    assert!(
        tick_should_draw(false, false, false, false, nav.is_mux_flash_active()),
        "tick_should_draw must report true while the mux flash is active"
    );
    assert!(
        !tick_should_draw(false, false, false, false, false),
        "idle gate must stay false when no operand is set"
    );
    // @step When 350ms of rendered frames have elapsed
    // 22 frames * 16ms = 352ms ≥ 350ms (FLASH_MS) — the window is
    // covered by the elapsed frames (compile-time-constant arithmetic).
    render_frames(&mut nav, &board, &mut agent, ELAPSE_FRAMES);
    // @step Then the mux flash is no longer active and tick_should_draw returns to the idle result (false when nothing else is animating)
    assert!(
        !nav.mux.is_flash_active(),
        "the flash must end after 350ms of rendered frames"
    );
    assert!(!nav.is_mux_flash_active());
    assert!(
        !tick_should_draw(false, false, false, false, nav.is_mux_flash_active()),
        "the gate must return to idle once the flash ends"
    );
    // MUX-007/MUX-008: the settled bar is REPAINT content, not an animation —
    // the gate stays closed but the next (triggered) frame still paints
    // the 1-row top bar on the focused pane.
    let agent_rect = nav.mux.pane_rects()[1];
    let after = render_buffer(&mut nav, &board, &mut agent);
    let settled = purple_cells(&after);
    assert_eq!(
        settled.len(),
        flash_cells(agent_rect, FLASH_MS).len(),
        "with the gate closed the settled 1-row top bar must still be painted"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: no flash occurs and no purple cells are painted when mux is
// off
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: no flash occurs and no purple cells are painted when mux is off
#[test]
fn no_flash_and_no_purple_cells_when_mux_is_off() {
    // @step Given the TUI is in a single-view mode (Board) with mux disabled
    let (mut nav, _rx) = fresh();
    assert_eq!(nav.active_view, ViewMode::Board);
    assert!(!nav.mux.config().enabled, "mux must start disabled");
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // @step When the frame is rendered to a terminal buffer
    let buf = render_buffer(&mut nav, &board, &mut agent);
    // @step Then no cell in the buffer has the dark purple flash background
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            assert_ne!(
                buf[(x, y)].bg,
                FLASH_PURPLE,
                "no purple flash cell may be painted with mux off at ({x}, {y})"
            );
        }
    }
    // @step And the mux-flash operand of the run-loop draw gate is false
    assert!(
        !nav.is_mux_flash_active(),
        "mux-flash gate operand must be false with mux off"
    );
    assert!(
        nav.mux.flash_pane().is_none(),
        "no flash may be armed with mux off"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the flash pattern is deterministic for a given clock and
// never persisted
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the flash pattern is deterministic for a given clock and never persisted
#[test]
fn the_flash_pattern_is_deterministic_for_a_given_clock_and_never_persisted() {
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
    assert!(!set_a.is_empty());
    // @step And the persisted tui.mux config shape is unchanged by the flash (no flash fields written)
    let value = serde_json::to_value(nav.mux.config()).expect("serialize MuxConfig");
    let keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
    assert_eq!(
        keys.len(),
        5,
        "MuxConfig must keep its 5-field serde shape (orientation, splits, panes, focused_pane, enabled): {keys:?}"
    );
    assert!(
        !keys.iter().any(|k| k.to_lowercase().contains("flash")),
        "no flash field may leak into the persisted config: {keys:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Property: flash_cells stays inside the rect, is deterministic and has
// no duplicates for arbitrary (rect, clock)
// ─────────────────────────────────────────────────────────────────────────

proptest! {
    // @step (property) for arbitrary pane rect + clock within the window
    #[test]
    fn flash_cells_stay_inside_the_rect_and_are_deterministic(
        x in 0u16..=8, y in 0u16..=8, w in 1u16..=160, h in 1u16..=60,
        clock in 0u64..=350) {
        use ratatui::layout::Rect;
        let rect = Rect::new(x, y, w, h);
        let a = flash_cells(rect, clock);
        let b = flash_cells(rect, clock);
        prop_assert_eq!(&a, &b, "flash_cells must be deterministic");
        for (cx, cy) in &a {
            prop_assert!(
                *cx >= rect.x && *cx < rect.x + rect.width
                    && *cy >= rect.y && *cy < rect.y + rect.height,
                "flash cell ({cx}, {cy}) escaped the pane rect {rect:?}"
            );
        }
        let mut dedup = a.clone();
        dedup.sort_unstable();
        dedup.dedup();
        prop_assert_eq!(dedup.len(), a.len(), "flash_cells must not repeat a cell");
    }
}
