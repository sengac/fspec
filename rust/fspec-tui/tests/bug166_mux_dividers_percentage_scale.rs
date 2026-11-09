// Feature: spec/features/mux-dividers-percentage-scale.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.
//
// BUG-166 — mux dividers: one per pair of neighbours, drag release persists
// the released position (no snap-back to equal) as weights written into the
// stored layout string (MUX-010).
//
// Navigator-level scenarios drive `Navigator::render_with_stores` +
// `Navigator::handle_event` through a `TestBackend`; the persistence
// scenario round-trips the weighted layout through the shared
// fspec-config.json.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use proptest::proptest;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use serial_test::serial;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::multiplex::layout_tree::{compute, drag::reweight};
use codelet_fspec_tui::views::multiplex::{MuxLayout, MuxNode, MuxPaneKind};
use codelet_fspec_tui::views::{Navigator, ViewMode};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use ratatui::layout::Rect;
use tempfile::TempDir;

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory
/// (within this test binary).
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// BUG-167: root the process-global data directory at a fresh throwaway
/// dir (the established tui093 pattern). Returns the guard (held for the
/// test's duration) + the TempDir.
async fn root_data_dir(
) -> Result<(tokio::sync::MutexGuard<'static, ()>, TempDir), codelet_test_helpers::TestError> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = TempDir::new().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok((guard, tmp))
}

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
) -> Result<(), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(w, h)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
    })
    .or_fail("draw")?;
    Ok(())
}

/// Enable the default preset on the Navigator's mux layout.
fn enable_default(nav: &mut Navigator) {
    nav.mux.enable_default();
    nav.active_view = ViewMode::Mux;
}

/// The divider rects of the live mux geometry.
fn divider_rects(nav: &Navigator) -> Vec<Rect> {
    nav.mux.dividers().iter().map(|d| d.rect).collect()
}

/// The weights of the stored layout's top-level items.
fn top_weights(nav: &Navigator) -> Vec<u32> {
    nav.mux
        .config()
        .layout
        .items()
        .iter()
        .map(MuxNode::weight)
        .collect()
}

fn three_panes(nav: &mut Navigator) {
    if let Some(mux_layout) =
        MuxLayout::columns(&[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent])
    {
        nav.mux.set_layout(mux_layout);
    }
    nav.active_view = ViewMode::Mux;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: three panes render one divider between every pair of panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: three panes render one divider between every pair of panes
#[test]
fn three_panes_render_one_divider_between_every_pair_of_panes() -> TestResult {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    // @step When the grid is rendered
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 3, "three panes must render");
    let dividers = divider_rects(&nav);
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a three-pane row group paints one row divider between every
// pair
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a three-pane row group paints one row divider between every pair
#[test]
fn a_three_pane_row_group_paints_one_row_divider_between_every_pair() -> TestResult {
    // @step Given mux mode uses the layout "[board, agent, files]" on a 40-row terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    nav.mux
        .set_layout(MuxLayout::parse("[board, agent, files]").or_fail("layout")?);
    // @step When the grid is rendered
    draw_with_agent(&mut nav, &board, &mut agent, 120, 40)?;
    let rects = nav.mux.pane_rects().to_vec();
    let dividers = divider_rects(&nav);
    // @step Then two full-width row dividers separate the three stacked panes
    assert_eq!(rects.len(), 3, "three stacked panes render");
    assert_eq!(dividers.len(), 2, "a 3-pane row group needs 2 row dividers");
    assert!(
        nav.mux.dividers().iter().all(|d| !d.between_columns),
        "the dividers separate rows"
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
        assert_eq!(dividers[i].width, 120, "row divider {i} spans the terminal");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing a divider keeps the released position instead of
// resetting to an equal split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing a divider keeps the released position instead of resetting to an equal split
#[test]
fn releasing_a_divider_keeps_the_released_position_instead_of_resetting_to_an_equal_split(
) -> TestResult {
    // @step Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        SessionId::new("s-1"),
    ));
    enable_default(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        (rects[0].width, rects[1].width),
        (59, 60),
        "precondition: equal 59/60 split"
    );
    // @step When I press the mouse down on the divider, drag it right past the midpoint and release
    let d0 = divider_rects(&nav)[0];
    let _ = nav.handle_event(&click(d0.x, d0.y + 5), &board);
    let _ = nav.handle_event(&drag(79, d0.y + 5), &board);
    let _ = nav.handle_event(&up(79, d0.y + 5), &board);
    assert!(!nav.mux.is_dragging(), "release must clear the drag state");
    // @step Then the Board pane keeps its released width (the weights written into the stored layout)
    // 79 of the pair's 119 cells = 66 percent: 2 x 66 : 2 x 34, reduced
    // by their gcd 4 -> board:33, agent:17.
    let after = nav.mux.pane_rects().to_vec();
    assert_eq!(
        nav.mux.config().layout.to_string(),
        "board:33, agent:17",
        "the release must write the weights into the stored layout"
    );
    let weights = top_weights(&nav);
    let share = |span: u32| (span * weights[0] / (weights[0] + weights[1])) as u16;
    assert_eq!(
        after[0].width,
        share(119),
        "the board pane must keep its released width (floor(119 x 33/50) = 78)"
    );
    assert!(
        after[0].width.abs_diff(79) <= 1,
        "the released width stays at the release column (got {})",
        after[0].width
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
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let redrawn = nav.mux.pane_rects().to_vec();
    assert_eq!(
        (redrawn[0].width, redrawn[1].width),
        (after[0].width, after[1].width),
        "the next frame must recompute the same widths from the stored weights"
    );
    draw_with_agent(&mut nav, &board, &mut agent, 160, 24)?;
    let resized = nav.mux.pane_rects().to_vec();
    assert_eq!(
        resized[0].width,
        share(160 - 1),
        "a resize re-applies the stored weights, not an equal split"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing the second divider stores the weights in the mux
// layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing the second divider stores the weights in the mux layout
#[test]
fn releasing_the_second_divider_stores_the_weights_in_the_mux_layout() -> TestResult {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let dividers = divider_rects(&nav);
    assert_eq!(dividers.len(), 2, "precondition: two dividers exist");
    // @step When I drag the divider between the second and third panes and release it
    let d1 = dividers[1];
    let _ = nav.handle_event(&click(d1.x, d1.y + 5), &board);
    let _ = nav.handle_event(&drag(115, d1.y + 5), &board);
    let _ = nav.handle_event(&up(115, d1.y + 5), &board);
    // @step Then the second and third panes' weights in the stored layout hold the released ratio
    // Equal 3-pane grid: pane 0 = 39 (x 0..38), divider 0 at x=39,
    // pane 1 = 39 (x 40..78), divider 1 at x=79, pane 2 = 40 (x 80..119).
    // Dragging divider 1 to x=115 → pane 1 takes 75 of the pair's 79
    // cells = 95 percent: the pair's joint weight 2 becomes 190 : 10, the
    // first pane's 1 becomes 100, reduced by the gcd 10 → 10 : 19 : 1.
    assert_eq!(
        nav.mux.config().layout.to_string(),
        "board:10, agent:19, agent",
        "the release must write the pair's weights into the stored layout"
    );
    let weights = top_weights(&nav);
    assert_eq!(
        (weights[1] * 100 + (weights[1] + weights[2]) / 2) / (weights[1] + weights[2]),
        95,
        "the pair's weights hold the released 95 percent"
    );
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        (rects[1].width, rects[2].width),
        (74, 5),
        "pane 1 keeps its released share (floor(118 x 19/30) = 74), pane 2 the rest"
    );
    // @step And the first pane's weight keeps its share of the group
    assert_eq!(
        weights[0] * 3,
        weights.iter().sum::<u32>(),
        "the first pane keeps its one-third share of the group"
    );
    assert_eq!(rects[0].width, 39, "the first pane keeps its width");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the second divider resizes its two adjacent panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the second divider resizes its two adjacent panes
#[test]
fn dragging_the_second_divider_resizes_its_two_adjacent_panes() -> TestResult {
    // @step Given mux mode is active with three equal panes on a 120-column terminal
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let before = nav.mux.pane_rects().to_vec();
    let dividers = divider_rects(&nav);
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
    // @step And releasing stores non-equal weights in the stored layout
    let _ = nav.handle_event(&up(95, dividers[1].y + 5), &board);
    let weights = top_weights(&nav);
    assert!(
        weights.iter().any(|w| *w != weights[0]),
        "the released weights must be non-equal: {}",
        nav.mux.config().layout
    );
    assert_eq!(
        nav.mux.config().layout.to_string(),
        "board:5, agent:7, agent:3",
        "55 of 79 cells = 70 percent: 100 : 140 : 60, reduced by 20"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the dragged divider is highlighted while dragging
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the dragged divider is highlighted while dragging
#[test]
fn the_dragged_divider_is_highlighted_while_dragging() -> TestResult {
    // @step Given mux mode is active with three panes
    let (mut nav, _rx) = fresh();
    let board = codelet_fspec_tui::store::BoardStore::default();
    let mut agent = codelet_fspec_tui::store::AgentViewStore::default();
    seed_two_agent_sessions(&mut agent);
    enable_default(&mut nav);
    three_panes(&mut nav);
    draw_with_agent(&mut nav, &board, &mut agent, 120, 24)?;
    let dividers = divider_rects(&nav);
    // @step When I press the mouse down on the first divider
    let _ = nav.handle_event(&click(dividers[0].x, dividers[0].y + 5), &board);
    assert!(
        nav.mux.is_dragging(),
        "mouse-down on divider 0 starts the drag"
    );
    // @step Then the first divider is highlighted (cyan) and the other divider is dimmed
    let buf = {
        let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .or_fail("draw")?;
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
        let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .or_fail("draw")?;
        term.backend().buffer().clone()
    };
    assert_eq!(
        buf[(dividers[0].x, dividers[0].y + 5)].fg,
        Color::DarkGray,
        "the highlight must clear after release"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux save persists the dragged weights and a fresh bootstrap
// restores them
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux save persists the dragged weights and a fresh bootstrap restores them
#[tokio::test]
#[serial]
async fn mux_save_persists_the_dragged_weights_and_a_fresh_bootstrap_restores_them() -> TestResult {
    // BUG-167: no manual persist-dirs wiring — root the process-global
    // data directory the way the production entry points do.
    let (_data_guard, _data) = root_data_dir().await?;
    // @step Given mux mode is active with three panes whose dividers were dragged to a non-equal scale
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(
            session,
            "/mux board, agent, agent".to_string(),
        ));
    }
    // The weights two divider drags leave behind (20 / 45 / 35 percent).
    app.navigator_mut().mux.config_mut().layout =
        MuxLayout::parse("board:4, agent:9, agent:7").or_fail("layout")?;
    // @step When I submit the slash command "/mux save"
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/mux save".to_string()));
    }
    // @step Then the tui.mux key in fspec-config.json contains the layout string with the dragged weights
    let data_dir = codelet_common::get_data_dir().or_fail("data dir")?;
    let raw = std::fs::read_to_string(data_dir.join("fspec-config.json")).or_fail("read config")?;
    let saved: serde_json::Value = serde_json::from_str(&raw).or_fail("config is JSON")?;
    assert_eq!(
        saved["tui"]["mux"]["layout"],
        serde_json::json!("board:4, agent:9, agent:7"),
        "the dragged weights must round-trip in the layout string: {raw}"
    );
    // @step And a fresh bootstrap followed by /mux on restores the same non-equal widths
    let mock2 = Arc::new(MockBackend::new());
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.load_mux_config();
    app2.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app2.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    while let Some(handle) = app2.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    if let Some(session) = app2.current_session() {
        app2.dispatch(Action::InputSubmitted(session, "/mux on".to_string()));
    }
    assert_eq!(
        app2.mux_state().config().layout.to_string(),
        "board:4, agent:9, agent:7",
        "the weighted layout must survive the save/load round trip"
    );
    let widths: Vec<u16> = app2
        .navigator()
        .mux
        .pane_rects()
        .iter()
        .map(|r| r.width)
        .collect();
    assert_eq!(
        widths,
        vec![23, 53, 42],
        "118 available cells split 4 : 9 : 7 (the last pane takes the remainder)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Property: a released drag keeps the pane at the release position
// ─────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn a_released_drag_keeps_the_board_pane_at_the_release_position(cells in 1u16..=118) {
        let area = Rect::new(0, 1, 120, 22);
        let mut layout = MuxLayout::parse("board, agent").or_fail("layout")?;
        let divider = compute(area, &layout, &[]).dividers.first().cloned().or_fail("a divider")?;
        assert!(reweight(&mut layout, &divider, cells), "the drag re-weights the pair");
        let panes = compute(area, &layout, &[]).panes;
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].width + panes[1].width, 119, "the pair keeps its 119 cells");
        assert!(panes[0].width >= 1 && panes[1].width >= 1, "no pane collapses");
        // Percent rounding (1..=99 of 119 cells) keeps the pane within
        // two cells of the release position.
        let target = cells.clamp(2, 117);
        assert!(
            panes[0].width.abs_diff(target) <= 2,
            "released at {cells}, the board pane is {} wide ({layout})",
            panes[0].width
        );
    }
}
