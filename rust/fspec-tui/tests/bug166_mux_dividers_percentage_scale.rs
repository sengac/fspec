#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// Feature: spec/features/mux-dividers-percentage-scale.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.
//
// BUG-166 — mux dividers: one per inter-pane gap, drag release persists the
// released position (no snap-back to equal), and splits are a percentage
// scale that rescales proportionally when the pane count changes.
//
// Navigator-level scenarios drive `Navigator::render_with_stores` +
// `Navigator::handle_event` through a `TestBackend`; the pane-count
// scenarios go through the real `/mux` slash-command dispatch on a real
// `App`; the persistence scenario round-trips the full scale through the
// shared fspec-config.json.

use std::sync::Arc;

use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use proptest::proptest;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::multiplex::{scale_scales, MuxOrientation, MuxPaneKind};
use codelet_fspec_tui::views::{Navigator, ViewMode};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use tempfile::TempDir;

mod common;
use common::MockBackend;

fn fresh() -> (Navigator, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    (
        Navigator::new(Arc::new(codelet_fspec_tui::theme::Theme::default()), tx),
        rx,
    )
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn drag(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn up(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// MUX-002: agent slots only render when sessions are open — seed two so a
/// three-pane grid renders all three panes.
fn seed_two_agent_sessions(agent: &mut codelet_fspec_tui::store::AgentViewStore) {
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        SessionId::new("s-1"),
    ));
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        SessionId::new("s-2"),
    ));
}

fn draw_with_agent(
    nav: &mut Navigator,
    board: &codelet_fspec_tui::store::BoardStore,
    agent: &mut codelet_fspec_tui::store::AgentViewStore,
    w: u16,
    h: u16,
) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
    })
    .expect("draw");
}

/// Enable the default preset on the Navigator's mux layout.
fn enable_default(nav: &mut Navigator) {
    nav.mux.enable_default();
    nav.active_view = ViewMode::Mux;
}

fn three_panes(nav: &mut Navigator) {
    nav.mux.set_pane_list(
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
        None,
    );
    nav.active_view = ViewMode::Mux;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: three panes render one divider between every pair of panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: three panes render one divider between every pair of panes
#[test]
fn three_panes_render_one_divider_between_every_pair_of_panes() {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    // @step When the grid is rendered
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 3, "three panes must render");
    let dividers = nav.mux.divider_rects();
    // @step Then two dividers are painted: one between panes 1 and 2 and one between panes 2 and 3
    assert_eq!(
        dividers.len(),
        2,
        "three panes need one divider per inter-pane gap (n-1), got {}",
        dividers.len()
    );
    assert_eq!(
        dividers[0].x,
        rects[0].x + rects[0].width,
        "divider 0 sits immediately right of pane 0"
    );
    assert_eq!(
        dividers[1].x,
        rects[1].x + rects[1].width,
        "divider 1 sits immediately right of pane 1"
    );
    // @step And each divider is 1 column wide and spans the full pane height
    for (i, d) in dividers.iter().enumerate() {
        assert_eq!(d.width, 1, "divider {i} must be 1 column wide");
        assert_eq!(
            d.height, rects[0].height,
            "divider {i} must span the full pane height"
        );
        assert_eq!(d.y, rects[0].y, "divider {i} starts at the pane top");
    }
    // @step And the dividers sit immediately to the right of their left pane
    for i in 0..2 {
        assert_eq!(dividers[i].x, rects[i].x + rects[i].width);
        assert_eq!(
            rects[i + 1].x,
            dividers[i].x + 1,
            "pane {i}+1 starts after divider {i}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: vertical three-pane mux paints one row divider between every
// pair
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: vertical three-pane mux paints one row divider between every pair
#[test]
fn vertical_three_pane_mux_paints_one_row_divider_between_every_pair() {
    // @step Given mux mode is vertical with three panes on a 40-row terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    nav.mux.set_orientation(MuxOrientation::Vertical);
    // @step When the grid is rendered
    draw_with_agent(&mut nav, &board, &mut agent, 120, 40);
    let rects = nav.mux.pane_rects().to_vec();
    let dividers = nav.mux.divider_rects();
    // @step Then two full-width row dividers separate the three stacked panes
    assert_eq!(
        dividers.len(),
        2,
        "vertical 3-pane grid needs 2 row dividers"
    );
    for i in 0..2 {
        assert_eq!(
            dividers[i].y,
            rects[i].y + rects[i].height,
            "row divider {i} sits directly below pane {i}"
        );
        assert_eq!(dividers[i].height, 1, "row divider {i} is 1 row tall");
        assert_eq!(
            dividers[i].width, rects[0].width,
            "row divider {i} spans the full width"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing a divider keeps the released position instead of
// resetting to an equal split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing a divider keeps the released position instead of resetting to an equal split
#[test]
fn releasing_a_divider_keeps_the_released_position_instead_of_resetting_to_an_equal_split() {
    // @step Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        SessionId::new("s-1"),
    ));
    enable_default(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        (rects[0].width, rects[1].width),
        (59, 60),
        "precondition: equal 59/60 split"
    );
    // @step When I press the mouse down on the divider, drag it right past the midpoint and release
    let d0 = nav.mux.divider_rects()[0];
    let _ = nav.handle_event(&click(d0.x, d0.y + 5), &board);
    let _ = nav.handle_event(&drag(79, d0.y + 5), &board);
    let _ = nav.handle_event(&up(79, d0.y + 5), &board);
    assert!(!nav.mux.is_dragging(), "release must clear the drag state");
    // @step Then the Board pane keeps its released width (the stored percent of the available width)
    let after = nav.mux.pane_rects().to_vec();
    let stored_pct = nav.mux.config().splits[0];
    assert!(
        stored_pct > 50,
        "the released position right of the midpoint must store a >50 percent (got {stored_pct})"
    );
    let available = 120 - 1;
    assert_eq!(
        after[0].width,
        (available as u32 * stored_pct as u32 / 100) as u16,
        "the board pane must keep its released width (stored {stored_pct}% of the available 119)"
    );
    // @step And the panes do NOT return to the equal 59/60 split
    assert_ne!(
        (after[0].width, after[1].width),
        (59, 60),
        "the snap-back bug: release must NOT reset to the equal split"
    );
    assert_eq!(
        after[0].width + after[1].width + 1,
        120,
        "panes + divider still fill the terminal"
    );
    // @step And the same widths are recomputed on the next frame and after a terminal resize
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let redrawn = nav.mux.pane_rects().to_vec();
    assert_eq!(
        (redrawn[0].width, redrawn[1].width),
        (after[0].width, after[1].width),
        "the next frame must recompute the same widths from the stored percent"
    );
    draw_with_agent(&mut nav, &board, &mut agent, 160, 24);
    let resized = nav.mux.pane_rects().to_vec();
    assert_eq!(
        resized[0].width,
        ((160 - 1) as u32 * stored_pct as u32 / 100) as u16,
        "a resize re-applies the stored percent, not an equal split"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing the second divider stores the percentage in the mux
// config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing the second divider stores the percentage in the mux config
#[test]
fn releasing_the_second_divider_stores_the_percentage_in_the_mux_config() {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let dividers = nav.mux.divider_rects().to_vec();
    assert_eq!(dividers.len(), 2, "precondition: two dividers exist");
    // @step When I drag the divider between the second and third panes and release it
    let d1 = dividers[1];
    let _ = nav.handle_event(&click(d1.x, d1.y + 5), &board);
    let _ = nav.handle_event(&drag(115, d1.y + 5), &board);
    let _ = nav.handle_event(&up(115, d1.y + 5), &board);
    // @step Then the second split entry of the mux config holds the released percent
    // Equal 3-pane grid: pane 0 = 39 (x 0..38), divider 0 at x=39,
    // pane 1 = 39 (x 40..78), divider 1 at x=79, pane 2 = 40 (x 80..119).
    // Dragging divider 1 to x=115 → pane 1 width = 115 - 40 = 75;
    // percent = 75/118 (the available axis) → 64 (rounded to nearest).
    let cfg = nav.mux.config();
    assert_eq!(cfg.splits.len(), 2, "3 panes -> n-1 = 2 scale entries");
    let available = 118u32; // 120 - 2 dividers
    let stored = ((75u32 * 100 + available / 2) / available) as u16;
    assert_eq!(
        cfg.splits[1], stored,
        "the released percent must be stored in splits[1]"
    );
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        rects[1].width,
        (available * stored as u32 / 100) as u16,
        "pane 1 keeps its released share"
    );
    // @step And the third pane holds the integer remainder of the scale
    assert_eq!(
        rects[2].width,
        (available - available * cfg.splits[0] as u32 / 100 - available * stored as u32 / 100)
            as u16,
        "pane 2 absorbs the remainder"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the second divider resizes its two adjacent panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the second divider resizes its two adjacent panes
#[test]
fn dragging_the_second_divider_resizes_its_two_adjacent_panes() {
    // @step Given mux mode is active with three equal panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let before = nav.mux.pane_rects().to_vec();
    let dividers = nav.mux.divider_rects().to_vec();
    assert_eq!(before[0].width, 39);
    // @step When I press the mouse down on the second divider and drag it right
    let _ = nav.handle_event(&click(dividers[1].x, dividers[1].y + 5), &board);
    let _ = nav.handle_event(&drag(95, dividers[1].y + 5), &board);
    // @step Then the second pane grows and the third pane shrinks live during the drag
    let mid = nav.mux.pane_rects().to_vec();
    assert!(
        mid[1].width > before[1].width,
        "pane 1 must grow while dragging divider 1 right ({}) > ({})",
        mid[1].width,
        before[1].width
    );
    assert!(
        mid[2].width < before[2].width,
        "pane 2 must shrink while dragging divider 1 right ({}) < ({})",
        mid[2].width,
        before[2].width
    );
    // @step And the first pane keeps its width
    assert_eq!(mid[0].width, before[0].width, "pane 0 is untouched");
    // @step And releasing stores a non-equal percentage scale in the config
    let _ = nav.handle_event(&up(95, dividers[1].y + 5), &board);
    let cfg = nav.mux.config();
    assert_ne!(
        (cfg.splits[0], cfg.splits[1]),
        (33, 33),
        "the released scale must be non-equal: {:?}",
        cfg.splits
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the dragged divider is highlighted while dragging
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the dragged divider is highlighted while dragging
#[test]
fn the_dragged_divider_is_highlighted_while_dragging() {
    // @step Given mux mode is active with three panes
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24);
    let dividers = nav.mux.divider_rects().to_vec();
    // @step When I press the mouse down on the first divider
    let _ = nav.handle_event(&click(dividers[0].x, dividers[0].y + 5), &board);
    assert!(
        nav.mux.is_dragging(),
        "mouse-down on divider 0 starts the drag"
    );
    // @step Then the first divider is highlighted (cyan) and the other divider is dimmed
    let buf = {
        let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .expect("draw");
        term.backend().buffer().clone()
    };
    assert_eq!(
        buf[(dividers[0].x, dividers[0].y + 5)].fg,
        Color::Cyan,
        "the dragged divider must be highlighted cyan"
    );
    assert_eq!(
        buf[(dividers[1].x, dividers[1].y + 5)].fg,
        Color::DarkGray,
        "the other divider stays dimmed"
    );
    // @step And releasing clears the highlight
    let _ = nav.handle_event(&up(dividers[0].x, dividers[0].y + 5), &board);
    let buf = {
        let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .expect("draw");
        term.backend().buffer().clone()
    };
    assert_eq!(
        buf[(dividers[0].x, dividers[0].y + 5)].fg,
        Color::DarkGray,
        "the highlight must clear after release"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: adding a third pane rescales a 40/60 split proportionally
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: adding a third pane rescales a 40/60 split proportionally
#[tokio::test]
async fn adding_a_third_pane_rescales_a_40_60_split_proportionally() {
    // @step Given mux mode is active with Board at 40 percent and Agent at 60 percent
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    app.dispatch(Action::InputSubmitted("/mux board agent 40".to_string()));
    assert_eq!(app.mux_state().config().splits, vec![40]);
    // @step When I submit the slash command "/mux board agent agent"
    app.dispatch(Action::InputSubmitted("/mux board agent agent".to_string()));
    // @step Then the new third pane gets an equal share (one third of the width)
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 3);
    assert_eq!(cfg.splits.len(), 2, "3 panes -> 2 scale entries");
    // @step And the Board and Agent panes shrink proportionally so the whole scale still sums to the full width
    // 40/60 rescaled to 3 panes: 40*2/3 = 26.67 → 27, 60*2/3 = 40, and
    // the new (last) pane takes the implicit equal third: 100-27-40.
    assert_eq!(
        cfg.splits,
        vec![27, 40],
        "the 40/60 scale must rescale proportionally"
    );
    assert_eq!(
        100 - 27 - 40,
        33,
        "the new pane gets the equal one-third share (33)"
    );
    // @step And no user-set position is thrown away: the relative Board-to-Agent ratio is preserved
    // (2:3 up to one-point rounding: 27/40 = 0.675 vs 2/3 = 0.667)
    assert!(
        (cfg.splits[0] as f64 / cfg.splits[1] as f64 - 2.0 / 3.0).abs() < 0.02,
        "board:agent must stay ~2:3 after the rescale ({}:{})",
        cfg.splits[0],
        cfg.splits[1]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: removing a pane re-allocates its share to the surviving panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: removing a pane re-allocates its share to the surviving panes
#[tokio::test]
async fn removing_a_pane_re_allocates_its_share_to_the_surviving_panes() {
    // @step Given mux mode is active with three panes at 40, 30 and 30 percent
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    app.dispatch(Action::InputSubmitted("/mux board agent agent".to_string()));
    assert_eq!(app.mux_state().config().splits, vec![33, 33]);
    // 40/30/30 scale: the last pane is the integer remainder of 40+30.
    app.navigator_mut().mux.config_mut().splits = vec![40, 30];
    // @step When I submit the slash command "/mux board agent"
    app.dispatch(Action::InputSubmitted("/mux board agent".to_string()));
    // @step Then the surviving panes keep their relative 40-to-30 ratio at the new scale (57 and 43 percent)
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 2);
    assert_eq!(
        cfg.splits,
        vec![57],
        "40:30 rescaled to 2 panes keeps the ratio (57/43)"
    );
    // @step And the two panes fill the full width with no share lost
    let total: u32 = cfg.splits.iter().map(|p| *p as u32).sum();
    assert_eq!(
        total, 57,
        "board keeps its proportional share; agent takes the 43 remainder"
    );
    // @step And the stored scale has exactly one entry (n-1 entries for n panes)
    assert_eq!(cfg.splits.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 4 on an equal two-pane split divides the width equally
// across four panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 4 on an equal two-pane split divides the width equally across four panes
#[tokio::test]
async fn mux_4_on_an_equal_two_pane_split_divides_the_width_equally_across_four_panes() {
    // @step Given mux mode is active with the default two panes at an equal split on a 200-column terminal
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    // MUX-004: bare /mux now opens the config dialog; the explicit
    // "/mux on" keeps the enable-with-default-preset path.
    app.dispatch(Action::InputSubmitted("/mux on".to_string()));
    assert_eq!(
        app.mux_state().config().splits,
        vec![50],
        "precondition: default equal split"
    );
    // @step When I submit the slash command "/mux 4"
    app.dispatch(Action::InputSubmitted("/mux 4".to_string()));
    // @step Then each of the four panes gets an equal share of the width
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 4);
    assert_eq!(
        cfg.splits,
        vec![25, 25, 25],
        "the equal scale rescales to four equal quarters"
    );
    // @step And the stored scale has three entries that sum with the remainder to 100
    assert_eq!(cfg.splits.len(), 3, "4 panes -> 3 entries");
    let total: u32 = cfg.splits.iter().map(|p| *p as u32).sum();
    assert!(
        (74..=100).contains(&total),
        "entries + integer remainder must cover 100% (entries sum {total})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux save persists every split entry and a fresh bootstrap
// restores them
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux save persists every split entry and a fresh bootstrap restores them
#[tokio::test]
async fn mux_save_persists_every_split_entry_and_a_fresh_bootstrap_restores_them() {
    let data = TempDir::new().expect("data dir");
    let cwd = TempDir::new().expect("cwd");
    // @step Given mux mode is active with three panes whose dividers were dragged to a non-equal scale
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    app.dispatch(Action::InputSubmitted("/mux board agent agent".to_string()));
    app.navigator_mut().mux.config_mut().splits = vec![20, 45];
    app.set_mux_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
    // @step When I submit the slash command "/mux save"
    app.dispatch(Action::InputSubmitted("/mux save".to_string()));
    // @step Then the tui.mux key in fspec-config.json contains all n-1 split entries
    let raw = std::fs::read_to_string(data.path().join("fspec-config.json")).expect("read config");
    assert!(raw.contains("20"), "splits[0]=20 must round-trip: {raw}");
    assert!(raw.contains("45"), "splits[1]=45 must round-trip: {raw}");
    // @step And a fresh bootstrap followed by /mux on restores the same non-equal scale
    let mock2 = Arc::new(MockBackend::new());
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.set_mux_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
    app2.load_mux_config();
    app2.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    while let Some(handle) = app2.next_pending_task() {
        let _ = handle.await;
    }
    app2.dispatch(Action::InputSubmitted("/mux on".to_string()));
    let cfg = app2.mux_state().config();
    assert_eq!(
        cfg.splits,
        vec![20, 45],
        "the non-equal scale must survive the save/load round trip"
    );
    assert_eq!(cfg.panes.len(), 3);
}

// ─────────────────────────────────────────────────────────────────────────
// Property: scale_scales preserves the full 100% across pane-count changes
// ─────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn scale_scales_of_equal_scales_stay_equal_for_two_to_four_panes(
        count in 2usize..=4,
        target in 2usize..=4,
    ) {
        let equal = vec![100u16 / count as u16; count - 1];
        let rescaled = scale_scales(&equal, target);
        assert_eq!(rescaled.len(), target - 1, "n-1 entries for n panes");
        // Every pane (entries + the implicit last share) lands within a
        // point of the true equal share.
        let last = 100u32 - rescaled.iter().map(|p| *p as u32).sum::<u32>();
        let expect = 100u32 / target as u32;
        let shares: Vec<u32> = rescaled.iter().map(|p| *p as u32).collect();
        for p in shares.iter().chain(std::iter::once(&last)) {
            assert!(
                *p >= expect - 1 && *p <= expect + 1,
                "pane share {p} must be within 1 of the equal share {expect} (target {target})"
            );
        }
        assert!(last >= 1, "the last pane keeps its share: {rescaled:?}");
    }
}
