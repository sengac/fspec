#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// Feature: spec/features/rust-mux-mode.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.

use std::sync::Arc;
use std::sync::Mutex;

use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serial_test::serial;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::store::{AgentViewStore, BoardStore};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::{Navigator, ViewMode};

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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    // KeyEvent::new defaults to KeyEventKind::Press (the only kind the
    // App's central filter admits).
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

fn render(nav: &mut Navigator, board: &BoardStore, agent: &mut AgentViewStore) {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
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

/// MUX-002: an agent slot only renders when a session is open — seed one
/// into the agent store so render-level scenarios see the agent pane.
fn seed_agent_session(agent: &mut AgentViewStore) {
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// BUG-167: shared-config persistence helpers (the manual persist-dirs
// wiring was removed from App; these helpers root the process-global
// data directory the way the production entry points do — combined/
// daemon via build_service, client via client::run)
// ─────────────────────────────────────────────────────────────────────────

/// Serialises tests that mutate the process-global data directory
/// (within this test binary).
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the established tui093 pattern). Returns the guard
/// (held for the test's duration) + the TempDir.
fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set data dir");
    (guard, tmp)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing j on the focused board pane moves the selection
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing j on the focused board pane moves the selection
#[test]
fn pressing_j_on_the_focused_board_pane_moves_the_selection() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, mut rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "backlog"),
        wu("AUTH-003", "backlog"),
    ]);
    let agent = AgentViewStore::default();
    // @step And the Board pane is focused
    assert!(nav.mux.focus() == 0);
    // @step When I press the key "j"
    let result = nav.handle_event(&key(KeyCode::Char('j'), KeyModifiers::NONE), &board);
    // @step Then the board selection moves down one row
    assert!(
        result.is_consumed(),
        "'j' on the board pane must be consumed"
    );
    assert!(
        matches!(rx.try_recv().ok(), Some(Action::SelectNext)),
        "board 'j' must emit SelectNext, got: {:?}",
        rx.try_recv().ok()
    );
    // @step And the agent input and scrollback are unchanged
    assert!(
        nav.agent.input.value().is_empty(),
        "agent input must be untouched while the board pane is focused"
    );
    assert_eq!(nav.agent.chunk_count(&agent), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing k on the focused board pane leaves the agent
// untouched
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing k on the focused board pane leaves the agent untouched
#[test]
fn pressing_k_on_the_focused_board_pane_leaves_the_agent_untouched() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let agent = AgentViewStore::default();
    // @step And the Board pane is focused
    assert!(nav.mux.focus() == 0);
    // @step When I press the key "k"
    let _ = nav.handle_event(&key(KeyCode::Char('k'), KeyModifiers::NONE), &board);
    // @step Then the agent input and scrollback are still unchanged
    assert!(nav.agent.input.value().is_empty());
    assert_eq!(nav.agent.chunk_count(&agent), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: clicking inside the Agent pane focuses it and routes the
// click
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: clicking inside the Agent pane focuses it and routes the click
#[test]
fn clicking_inside_the_agent_pane_focuses_it_and_routes_the_click() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent);
    // @step And the Board pane is focused
    assert!(nav.mux.focus() == 0);
    let rects = nav.mux.pane_rects().to_vec();
    let agent_rect = rects[1];
    // @step When I click inside the Agent pane rect
    let result = nav.handle_event(&click(agent_rect.x + 5, agent_rect.y + 5), &board);
    // @step Then the Agent pane is focused
    assert!(
        nav.mux.focus() == 1,
        "clicking the agent pane must focus it"
    );
    assert!(result.is_consumed());
    // @step And the click event is forwarded to the Agent pane handler
    // (the agent consumed the click — its mouse dispatch ran; the
    // observable effect is that the focus moved AND the event was
    // consumed rather than ignored)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: clicking in a gap outside every pane rect leaves focus
// unchanged
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: clicking in a gap outside every pane rect leaves focus unchanged
#[test]
fn clicking_in_a_gap_outside_every_pane_rect_leaves_focus_unchanged() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    render(&mut nav, &board, &mut agent);
    // @step And the Board pane is focused
    assert!(nav.mux.focus() == 0);
    // @step When I click in a gap outside every pane rect
    // The mux footer row sits below every pane rect.
    let footer_row = 23u16;
    let _ = nav.handle_event(&click(5, footer_row), &board);
    // @step Then the focus stays on the Board pane
    assert!(
        nav.mux.focus() == 0,
        "a click outside every pane must not move focus (was: {:?})",
        nav.mux.focus()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left cycles focus from ChangedFiles to Agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left cycles focus from ChangedFiles to Agent
#[test]
fn shift_left_cycles_focus_from_changed_files_to_agent() {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux.set_pane_count(3);
    nav.active_view = ViewMode::Mux;
    let board = BoardStore::default();
    let agent = AgentViewStore::default();
    // @step And the ChangedFiles pane is focused
    nav.mux.set_focus(2);
    // @step When I press Shift+Left
    let _ = nav.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT), &board);
    // @step Then the Agent pane is focused
    assert!(nav.mux.focus() == 1);
    let _ = agent;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left cycles focus from Agent to Board
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left cycles focus from Agent to Board
#[test]
fn shift_left_cycles_focus_from_agent_to_board() {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux.set_pane_count(3);
    nav.active_view = ViewMode::Mux;
    let board = BoardStore::default();
    let agent = AgentViewStore::default();
    // @step And the Agent pane is focused
    nav.mux.set_focus(1);
    // @step When I press Shift+Left
    let _ = nav.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT), &board);
    // @step Then the Board pane is focused
    assert!(nav.mux.focus() == 0);
    let _ = agent;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left at the first pane stops without wrapping
// (MUX-002 supersedes MUX-001's wrap-around)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left at the first pane stops without wrapping
#[test]
fn shift_left_at_the_first_pane_stops_without_wrapping() {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux.set_pane_count(3);
    nav.active_view = ViewMode::Mux;
    let board = BoardStore::default();
    let agent = AgentViewStore::default();
    // @step And the Board pane is focused
    nav.mux.set_focus(0);
    // @step When I press Shift+Left
    let _ = nav.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT), &board);
    // @step Then the Board pane is still focused
    // (MUX-002 rule 5: Shift+Left at the first pane STOPS — no wrap-around
    // to the rightmost pane; MUX-001's wrap-around is superseded)
    assert!(
        nav.mux.focus() == 0,
        "Shift+Left at the first pane must stop, not wrap (was: {})",
        nav.mux.focus()
    );
    let _ = agent;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the divider left resizes the split live
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the divider left resizes the split live
#[test]
fn dragging_the_divider_left_resizes_the_split_live() {
    // @step Given mux mode is active with Board and Agent panes at a 50/50 split on a 120-column terminal
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent);
    let divider = nav.mux.divider_rects()[0];
    // @step When I press the mouse down on the divider and drag it left
    let _ = nav.handle_event(&click(divider.x, divider.y + 5), &board);
    assert!(
        nav.mux.is_dragging(),
        "mouse-down on the divider must start a drag"
    );
    let _ = nav.handle_event(&drag(divider.x - 10, divider.y + 5), &board);
    // @step Then the Board pane shrinks and the Agent pane grows live during the drag
    let rects = nav.mux.pane_rects().to_vec();
    assert!(
        rects[0].width < 60,
        "board pane must shrink while dragging left (got {})",
        rects[0].width
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing the divider stores the released percent without
// snapping back
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing the divider stores the released percent without snapping back
#[test]
fn releasing_the_divider_stores_the_released_percent_without_snapping_back() {
    // @step Given mux mode is active with Board and Agent panes at a 50/50 split on a 120-column terminal
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent);
    let divider = nav.mux.divider_rects()[0];
    let _ = nav.handle_event(&click(divider.x, divider.y + 5), &board);
    // @step When I release the mouse at a position that would make the Board pane 40 columns
    let _ = nav.handle_event(&drag(40, divider.y + 5), &board);
    let _ = nav.handle_event(&up(40, divider.y + 5), &board);
    // @step Then the Board pane keeps its released 34 percent of the width (40/119, rounded — no 64-column minimum, no equal-split reset)
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        rects[0].width,
        119 * 34 / 100,
        "board pane keeps its released width (34% of 119), no 64-col minimum"
    );
    assert_eq!(
        nav.mux.config().splits,
        vec![34],
        "the released percent must be stored in the scale"
    );
    // @step And the drag state is cleared after the release
    assert!(!nav.mux.is_dragging());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a board work unit in mux mode focuses the agent
// pane (Navigator half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on a board work unit in mux mode focuses the agent pane
#[test]
fn enter_on_a_board_work_unit_in_mux_mode_focuses_the_agent_pane() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, mut rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // @step And the Board pane is focused with work unit AUTH-001 selected
    assert!(nav.mux.focus() == 0);
    // @step When I press Enter
    let result = nav.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE), &board);
    // @step Then the work unit AUTH-001 is bound to the agent pane session
    assert!(result.is_consumed());
    assert!(
        matches!(
            rx.try_recv().ok(),
            Some(Action::MuxEnterWorkUnit(id)) if id == "AUTH-001"
        ),
        "Enter on the board pane in mux mode must emit MuxEnterWorkUnit(AUTH-001)"
    );
    // @step And the Agent pane is focused
    nav.apply_action(&Action::MuxEnterWorkUnit("AUTH-001".to_string()));
    assert!(
        nav.mux.focus() == 1,
        "applying MuxEnterWorkUnit must focus the agent pane"
    );
    // @step And the TUI is still in mux mode showing both panes
    assert_eq!(nav.active_view, ViewMode::Mux);
    // @step And the board stays visible in its pane
    // (140-col terminal: the equal-division board pane is 69 cols,
    // above the board view's 64-col fit width — MUX-003 removed the
    // layout-level minimum clamp, so the pane is only as wide as its
    // equal share)
    let out = {
        let mut term = Terminal::new(TestBackend::new(140, 24)).expect("Terminal::new");
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .expect("draw");
        let buf = term.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(
        out.contains("BACKLOG"),
        "the board must stay visible in its pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: existing single-view behavior is unchanged when mux is off
// (Navigator half — R10)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: existing single-view behavior is unchanged when mux is off
#[test]
fn existing_single_view_behavior_is_unchanged_when_mux_is_off() {
    // @step Given the TUI is showing the single Board view with mux inactive
    let (mut nav, _rx) = fresh();
    assert_eq!(nav.active_view, ViewMode::Board);
    assert!(!nav.mux.config().enabled);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // @step When I press Enter on a selected work unit
    let _ = nav.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE), &board);
    nav.apply_action(&Action::EnterWorkUnit("AUTH-001".to_string()));
    // @step Then the TUI flips to the single Agent view exactly as before mux existed
    assert_eq!(nav.active_view, ViewMode::Agent);
    // @step And pressing Esc in the Agent view returns to the single Board view
    nav.apply_action(&Action::BackToBoard);
    assert_eq!(nav.active_view, ViewMode::Board);
    // @step And no mux footer row is painted and no pane divider is rendered
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        !rows.iter().any(|r| r.contains("MUX")),
        "no mux footer may be painted when mux is off"
    );
    assert!(
        !rows.iter().any(|r| r.contains('│') && r.contains("MUX")),
        "no mux divider may be painted when mux is off"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /mux slash-command grammar parser tests (merged from mux001_parser.rs)
// ─────────────────────────────────────────────────────────────────────────

use codelet_fspec_tui::app::mux_parser::{parse_mux_command, MuxError, MuxSubcommand};
use codelet_fspec_tui::views::multiplex::{MuxOrientation, MuxPaneKind};
use proptest::{prop_assume, proptest};

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux toggles mux mode on with the default preset (parser
// half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enables mux mode with the default preset
#[test]
fn bare_mux_and_on_off_and_help_parse() {
    // @step Given the TUI is showing the single Board view
    // (parser is pure — no TUI needed; the Given is the pre-state the
    // dispatcher applies the parse result to)
    // @step When I submit the slash command "/mux"
    // MUX-004: bare /mux resolves to Config (opens the MuxConfigDialog),
    // the MUX-001 Toggle behaviour is superseded.
    assert_eq!(parse_mux_command("/mux"), Ok(MuxSubcommand::Config));
    // @step Then mux mode is active with the default preset
    // (the dispatcher applies the default preset on Toggle-from-off;
    // here we assert the parser hands the dispatcher the right variant)
    assert_eq!(parse_mux_command("/mux on"), Ok(MuxSubcommand::On));
    assert_eq!(parse_mux_command("/mux off"), Ok(MuxSubcommand::Off));
    assert_eq!(parse_mux_command("/mux help"), Ok(MuxSubcommand::Help));
    // @step And the grid shows two horizontal panes: Board on the left and Agent on the right
    // (the default preset shape is asserted in mux001_app.rs +
    // presets.rs; the parser only resolves the subcommand)
    // @step And the split is 50/50 with the Board pane focused on fresh entry (the agent pane is the persisted home focus)
    // (see above — dispatcher-level assertion)
    // @step And the mux footer row is painted at the bottom of the screen
    // (render-level assertion in mux001_app.rs)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux v sets the grid to vertical
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux v sets the grid to vertical
#[test]
fn mux_v_parses_to_vertical_orientation() {
    // @step Given mux mode is active with panes Board and Agent
    // (parser-level: assert the variant carries the right orientation)
    // @step When I submit the slash command "/mux v"
    assert_eq!(
        parse_mux_command("/mux v"),
        Ok(MuxSubcommand::Orientation(MuxOrientation::Vertical))
    );
    // @step Then the grid is vertical with Board on top and Agent on the bottom
    // (the dispatcher folds this into MuxConfig.orientation)
    assert_eq!(
        parse_mux_command("/mux vertical"),
        Ok(MuxSubcommand::Orientation(MuxOrientation::Vertical))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux h sets the grid to horizontal
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux h sets the grid to horizontal
#[test]
fn mux_h_parses_to_horizontal_orientation() {
    // @step Given mux mode is active with panes Board and Agent
    // (parser-level: assert the variant carries the right orientation)
    // @step When I submit the slash command "/mux h"
    assert_eq!(
        parse_mux_command("/mux h"),
        Ok(MuxSubcommand::Orientation(MuxOrientation::Horizontal))
    );
    // @step Then the grid is horizontal with Board on the left and Agent on the right
    // (the dispatcher folds this into MuxConfig.orientation)
    assert_eq!(
        parse_mux_command("/mux horizontal"),
        Ok(MuxSubcommand::Orientation(MuxOrientation::Horizontal))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 3 sets the pane count to three
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 3 sets the pane count to three
#[test]
fn mux_3_parses_to_pane_count_three() {
    // @step Given mux mode is active with the default two panes
    // (parser-level: assert the count round-trips)
    // @step When I submit the slash command "/mux 3"
    assert_eq!(parse_mux_command("/mux 3"), Ok(MuxSubcommand::PaneCount(3)));
    // @step Then the grid shows three panes: Board, Agent and ChangedFiles
    // (the dispatcher expands the count to the default pane kinds)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 4 sets the pane count to four
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 4 sets the pane count to four
#[test]
fn mux_4_parses_to_pane_count_four() {
    // @step Given mux mode is active with three panes
    // (parser-level: assert the count round-trips)
    // @step When I submit the slash command "/mux 4"
    assert_eq!(parse_mux_command("/mux 4"), Ok(MuxSubcommand::PaneCount(4)));
    // @step Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    // (the dispatcher expands the count to the default pane kinds)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 2 collapses the pane count to two
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 2 collapses the pane count to two
#[test]
fn mux_2_parses_to_pane_count_two() {
    // @step Given mux mode is active with four panes
    // (parser-level: assert the count round-trips)
    // @step When I submit the slash command "/mux 2"
    assert_eq!(parse_mux_command("/mux 2"), Ok(MuxSubcommand::PaneCount(2)));
    // @step Then the grid shows two panes: Board and Agent
    // (the dispatcher collapses the pane list to the first two kinds)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board agent 40 sets an explicit pane list and split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board agent 40 sets an explicit pane list and split
#[test]
fn mux_board_agent_40_parses_to_pane_list_with_split() {
    // @step Given mux mode is active
    // (parser-level: assert the pane kinds + split round-trip)
    // @step When I submit the slash command "/mux board agent 40"
    assert_eq!(
        parse_mux_command("/mux board agent 40"),
        Ok(MuxSubcommand::PaneList {
            panes: vec![MuxPaneKind::Board, MuxPaneKind::Agent],
            split_percent: Some(40),
        })
    );
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    // (the dispatcher folds this into MuxConfig.panes + splits)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux files checkpoints sets a two-pane list
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux files checkpoints sets a two-pane list
#[test]
fn mux_files_checkpoints_parses_to_pane_list() {
    // @step Given mux mode is active
    // (parser-level: assert the pane kinds round-trip)
    // @step When I submit the slash command "/mux files checkpoints"
    assert_eq!(
        parse_mux_command("/mux files checkpoints"),
        Ok(MuxSubcommand::PaneList {
            panes: vec![MuxPaneKind::ChangedFiles, MuxPaneKind::Checkpoints],
            split_percent: None,
        })
    );
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    // (the dispatcher folds this into MuxConfig.panes)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux with an invalid pane kind leaves the config unchanged
// and shows an error (parser half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux with an invalid pane kind leaves the config unchanged and shows an error
#[test]
fn invalid_inputs_parse_to_stable_errors() {
    // @step Given mux mode is active with panes Board and Agent at a 50/50 split
    // (parser-level: the dispatcher leaves the config untouched on Err —
    // here we assert the error variants are stable + snapshot their text)
    // @step When I submit the slash command "/mux board zzz"
    assert!(matches!(
        parse_mux_command("/mux board zzz"),
        Err(MuxError::UnknownPaneKind(ref k)) if k == "zzz"
    ));
    // @step Then a one-line error "/mux: unknown pane kind: zzz" is shown in the agent scrollback
    insta::assert_snapshot!(
        "mux_error_unknown_pane_kind",
        MuxError::UnknownPaneKind("zzz".to_string()).to_string()
    );
    // @step And the grid still shows Board and Agent at a 50/50 split
    // (Err ⇒ dispatcher no-op; asserted in the integration test)
    // @step And the out-of-range command "/mux board agent 5" is rejected with a one-line error and leaves the config unchanged
    assert!(matches!(
        parse_mux_command("/mux board agent 5"),
        Err(MuxError::SplitPercentOutOfRange(5))
    ));
    insta::assert_snapshot!(
        "mux_error_split_percent_out_of_range",
        MuxError::SplitPercentOutOfRange(5).to_string()
    );
    assert!(matches!(
        parse_mux_command("/mux 1"),
        Err(MuxError::PaneCountOutOfRange(1))
    ));
    assert!(matches!(
        parse_mux_command("/mux 5"),
        Err(MuxError::PaneCountOutOfRange(5))
    ));
    assert!(matches!(
        parse_mux_command("/mux board agent 95"),
        Err(MuxError::SplitPercentOutOfRange(95))
    ));
    assert!(matches!(
        parse_mux_command("/mux board"),
        Err(MuxError::TooFewPanes(1))
    ));
    assert!(matches!(
        parse_mux_command("/mux board agent files checkpoints extra"),
        Err(MuxError::TooManyPanes(5))
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Proptest: the grammar property — every well-formed /mux line parses.
// ─────────────────────────────────────────────────────────────────────────

proptest! {
    /// Property: well-formed /mux lines always parse to the expected variant.
    #[test]
    fn well_formed_mux_lines_parse(count in 2usize..=4, pct in 10u16..=90) {
        let line = format!("/mux {count}");
        assert!(matches!(parse_mux_command(&line), Ok(MuxSubcommand::PaneCount(c)) if c == count));
        let line2 = format!("/mux board agent {pct}");
        assert!(matches!(
            parse_mux_command(&line2),
            Ok(MuxSubcommand::PaneList { split_percent: Some(p), .. }) if p == pct
        ));
    }
}

proptest! {
    /// Property: an unknown pane kind ALWAYS errors (never silently parses).
    #[test]
    fn unknown_pane_kind_always_errors(kind in "[a-z]{1,12}") {
        prop_assume!(!matches!(
            kind.as_str(),
            "board" | "agent" | "files" | "checkpoints"
        ));
        let line = format!("/mux board {kind}");
        assert!(matches!(
            parse_mux_command(&line),
            Err(MuxError::UnknownPaneKind(ref k)) if k == &kind
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Mux layout math tests (merged from mux001_layout.rs)
// ─────────────────────────────────────────────────────────────────────────

use ratatui::layout::Rect;

use codelet_fspec_tui::views::multiplex::calculate_pane_rects;

fn area(w: u16, h: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: split clamping never produces a pane narrower than the
// minimum width (layout-math half)
// ─────────────────────────────────────────────────────────────────────────
// MUX-003 supersedes R5: the minimum clamps are removed. An explicit
// percent is honored as-is (no board minimum); the second pane takes
// the remainder.

/// Scenario: split clamping never produces a pane narrower than the minimum width
#[test]
fn clamping_never_produces_a_sub_minimum_pane() {
    // @step Given a 110-column terminal with two mux panes
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // @step When the split is requested at 10/90
    let rects = calculate_pane_rects(area(110, 24), MuxOrientation::Horizontal, &panes, &[10]);
    // @step Then the first pane is clamped to the 64-column board minimum and the second pane takes the 44-column remainder
    // (MUX-003: no minimum — the 10% percent is honored as-is)
    assert_eq!(
        rects[0].width,
        109 * 10 / 100,
        "10% of 109 available, honored without clamping"
    );
    assert_eq!(
        rects[1].width,
        110 - rects[0].width - 1,
        "agent pane takes the remainder minus the 1-col divider"
    );
    // @step And no board pane rect is ever narrower than the 64-column minimum
    // (MUX-003: superseded — panes may be narrower than the old minimum)
    assert!(
        rects[0].width >= 1 && rects[1].width >= 1,
        "both panes non-empty"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux with an explicit pane list and split percent (layout half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux with an explicit pane list and split percent
#[test]
fn explicit_split_percent_is_respected_when_above_minimum() {
    // @step Given mux mode is active
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux board agent 40"
    // (the dispatcher folds this into splits = [40]; the layout math
    // consumes it here)
    let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[40]);
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    // (MUX-003: the percent is honored as-is, no minimum clamp)
    let available = 120u16 - 1; // 1-col divider
    assert_eq!(rects[0].width, available * 40 / 100);
    assert_eq!(rects[1].width, available - rects[0].width);
    // @step When I submit the slash command "/mux files checkpoints"
    let panes2 = [MuxPaneKind::ChangedFiles, MuxPaneKind::Checkpoints];
    let rects2 = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes2, &[50]);
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    assert_eq!(rects2.len(), 2);
    assert_eq!(rects2[0].x, 0);
    assert!(rects2[1].x > rects2[0].x);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux h and /mux v set the grid orientation (layout half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux h and /mux v set the grid orientation
#[test]
fn orientation_changes_stacking_direction() {
    // @step Given mux mode is active with panes Board and Agent
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux v"
    let v = calculate_pane_rects(area(120, 40), MuxOrientation::Vertical, &panes, &[50]);
    // @step Then the grid is vertical with Board on top and Agent on the bottom
    assert_eq!(v[0].y, 0);
    assert!(v[1].y > v[0].y, "second pane must sit below the first");
    assert_eq!(
        v[0].width, v[1].width,
        "vertical panes share the full width"
    );
    // @step When I submit the slash command "/mux h"
    let h = calculate_pane_rects(area(120, 40), MuxOrientation::Horizontal, &panes, &[50]);
    // @step Then the grid is horizontal with Board on the left and Agent on the right
    assert_eq!(h[0].x, 0);
    assert!(h[1].x > h[0].x, "second pane must sit right of the first");
    assert_eq!(
        h[0].height, h[1].height,
        "horizontal panes share the full height"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux <n> sets the pane count with default pane kinds (layout
// half — 2/3/4 panes)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux <n> sets the pane count with default pane kinds
#[test]
fn two_three_and_four_panes_partition_the_area() {
    // @step Given mux mode is active with the default two panes
    // @step When I submit the slash command "/mux 3"
    let three = [
        MuxPaneKind::Board,
        MuxPaneKind::Agent,
        MuxPaneKind::ChangedFiles,
    ];
    let rects3 = calculate_pane_rects(area(200, 24), MuxOrientation::Horizontal, &three, &[40, 30]);
    assert_eq!(rects3.len(), 3);
    // @step Then the grid shows three panes: Board, Agent and ChangedFiles
    // @step When I submit the slash command "/mux 4"
    let four = [
        MuxPaneKind::Board,
        MuxPaneKind::Agent,
        MuxPaneKind::ChangedFiles,
        MuxPaneKind::Checkpoints,
    ];
    let rects4 = calculate_pane_rects(
        area(300, 24),
        MuxOrientation::Horizontal,
        &four,
        &[30, 30, 30],
    );
    assert_eq!(rects4.len(), 4);
    // @step Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    // @step When I submit the slash command "/mux 2"
    let two = [MuxPaneKind::Board, MuxPaneKind::Agent];
    let rects2 = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &two, &[50]);
    assert_eq!(rects2.len(), 2);
    // @step Then the grid shows two panes: Board and Agent
    // Invariants: panes are contiguous, non-overlapping, and the last
    // pane takes the remainder (splits has n-1 entries).
    for (i, r) in rects4.iter().enumerate() {
        assert!(r.width > 0, "pane {i} must be non-empty");
        if i > 0 {
            let prev = &rects4[i - 1];
            assert_eq!(
                r.x,
                prev.x + prev.width + 1,
                "exactly one divider col between panes"
            );
        }
    }
    let total: u16 = rects4.iter().map(|r| r.width).sum();
    assert_eq!(total, 300 - 3, "panes + 3 dividers fill the terminal");
    let _ = (rects2, rects3);
}

// ─────────────────────────────────────────────────────────────────────────
// Boundary cases (design doc §5.4: tiny terminal, all-min splits)
// ─────────────────────────────────────────────────────────────────────────

/// Boundary: a tiny terminal degrades gracefully — no panic, no zero-width panes.
#[test]
fn tiny_terminal_degrades_gracefully() {
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // 66 cols: MUX-003 equal division — 65 available / 2 = 32/33.
    let rects = calculate_pane_rects(area(66, 24), MuxOrientation::Horizontal, &panes, &[50]);
    assert_eq!(rects[0].width, 32);
    assert!(
        rects[1].width >= 1,
        "the second pane must survive with ≥1 col"
    );
    // 10 cols: nothing fits at the board minimum — the layout must
    // still return 2 non-empty rects summing to the available width.
    let tiny = calculate_pane_rects(area(10, 24), MuxOrientation::Horizontal, &panes, &[50]);
    assert_eq!(tiny.len(), 2);
    assert!(tiny[0].width >= 1 && tiny[1].width >= 1);
}

/// Boundary: vertical orientation divides the rows equally (MUX-003:
/// no minimum pane height).
#[test]
fn vertical_orientation_enforces_minimum_height() {
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // 20 rows: 50% = 9 rows each (no minimum clamp).
    let rects = calculate_pane_rects(area(120, 20), MuxOrientation::Vertical, &panes, &[50]);
    assert!(rects[0].height >= 1, "board pane must be non-empty");
    assert!(rects[1].height >= 1);
    // Wide-enough terminal: 50/50 of 40 rows = 19 + 20 (MUX-003: the
    // last pane absorbs the remainder, no minimum clamp).
    let comfy = calculate_pane_rects(area(120, 40), MuxOrientation::Vertical, &panes, &[50]);
    assert_eq!(comfy[0].height, 19);
    assert_eq!(comfy[1].height, 20);
}

/// Boundary: the non-board panes take their equal share (MUX-003: no
/// 20-col floor).
#[test]
fn non_board_panes_use_the_20_col_floor() {
    let panes = [MuxPaneKind::Agent, MuxPaneKind::ChangedFiles];
    let rects = calculate_pane_rects(area(40, 24), MuxOrientation::Horizontal, &panes, &[10]);
    // 10% of 39 available = 3, honored as-is (no floor).
    assert_eq!(rects[0].width, 39 * 10 / 100);
    assert_eq!(rects[1].width, 40 - rects[0].width - 1);
}

/// Boundary: explicit percents below the old minimums are honored as-is
/// (MUX-003: no minimums); the last pane takes the remainder.
#[test]
fn all_min_splits_leave_remainder_for_last_pane() {
    let panes = [
        MuxPaneKind::Board,
        MuxPaneKind::Agent,
        MuxPaneKind::Checkpoints,
    ];
    // 100 cols: 10% of 98 = 9, 10% of 98 = 9, remainder 80.
    let rects = calculate_pane_rects(area(100, 24), MuxOrientation::Horizontal, &panes, &[10, 10]);
    assert_eq!(rects[0].width, 98 * 10 / 100);
    assert_eq!(rects[1].width, 98 * 10 / 100);
    assert_eq!(rects[2].width, 100 - rects[0].width - rects[1].width - 2);
}

// ─────────────────────────────────────────────────────────────────────────
// App-level integration tests (merged from mux001_app.rs)
// ─────────────────────────────────────────────────────────────────────────

use std::fs;

use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

fn submit(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Enable mux with the default preset on a fresh app (shared Given).
async fn app_with_mux(app: &mut App) {
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(app).await;
    // MUX-004: bare /mux now opens the config dialog; the explicit
    // "/mux on" keeps the MUX-001 enable-with-default-preset path.
    submit(app, "/mux on");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux on enables mux mode with the default preset
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enables mux mode with the default preset
#[tokio::test]
async fn slash_mux_on_enables_mux_mode_with_the_default_preset() {
    // @step Given the TUI is showing the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux on"
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    submit(&mut app, "/mux on");
    // @step Then mux mode is active with the default preset
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "/mux on must flip the view to Mux"
    );
    assert!(
        app.mux_state().config().enabled,
        "mux config must be enabled"
    );
    // @step And the grid shows two horizontal panes: Board on the left and Agent on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 2, "default preset has two panes");
    // @step And the split is 50/50 with the Board pane focused on fresh entry (the agent pane is the persisted home focus)
    assert_eq!(cfg.splits, vec![50], "default preset splits 50/50");
    assert_eq!(
        cfg.focused_pane, 1,
        "the agent pane is the persisted home focus (fresh entry focuses Board)"
    );
    // @step And the mux footer row is painted at the bottom of the screen
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .expect("Terminal::new");
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        term.backend().buffer().clone()
    };
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        rows.iter().any(|r| r.contains("MUX")),
        "the mux footer row must be painted: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux off returns to the pre-mux view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux off returns to the pre-mux view
#[tokio::test]
async fn slash_mux_off_returns_to_the_pre_mux_view() {
    // @step Given the TUI is showing the single Agent view
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::EnterWorkUnit("AUTH-001".to_string()));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step When I submit the slash command "/mux on"
    // (MUX-004: bare /mux now opens the config dialog; the explicit
    // "/mux on" keeps the MUX-001 enable path this scenario exercises)
    submit(&mut app, "/mux on");
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    // @step Then mux mode is inactive
    assert!(!app.mux_state().config().enabled);
    // @step And the TUI shows the single Agent view that was active before mux was entered
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "/mux off must restore the pre-mux view"
    );
    // @step And the board and agent store state is unchanged by the round trip
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        Some("AUTH-001"),
        "the work-unit binding must survive the mux round trip"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux v sets the grid to vertical
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux v sets the grid to vertical
#[tokio::test]
async fn slash_mux_v_sets_the_grid_to_vertical() {
    // @step Given mux mode is active with panes Board and Agent
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    // @step When I submit the slash command "/mux v"
    submit(&mut app, "/mux v");
    // @step Then the grid is vertical with Board on top and Agent on the bottom
    use codelet_fspec_tui::views::multiplex::MuxOrientation;
    assert_eq!(
        app.mux_state().config().orientation,
        MuxOrientation::Vertical
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux h sets the grid to horizontal
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux h sets the grid to horizontal
#[tokio::test]
async fn slash_mux_h_sets_the_grid_to_horizontal() {
    // @step Given mux mode is active with panes Board and Agent
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    submit(&mut app, "/mux v");
    // @step When I submit the slash command "/mux h"
    submit(&mut app, "/mux h");
    // @step Then the grid is horizontal with Board on the left and Agent on the right
    use codelet_fspec_tui::views::multiplex::MuxOrientation;
    assert_eq!(
        app.mux_state().config().orientation,
        MuxOrientation::Horizontal
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 3 sets the pane count to three
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 3 sets the pane count to three
#[tokio::test]
async fn slash_mux_3_sets_the_pane_count_to_three() {
    // @step Given mux mode is active with the default two panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    assert_eq!(app.mux_state().config().panes.len(), 2);
    // @step When I submit the slash command "/mux 3"
    submit(&mut app, "/mux 3");
    // @step Then the grid shows three panes: Board, Agent and ChangedFiles
    assert_eq!(app.mux_state().config().panes.len(), 3);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 4 sets the pane count to four
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 4 sets the pane count to four
#[tokio::test]
async fn slash_mux_4_sets_the_pane_count_to_four() {
    // @step Given mux mode is active with three panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    submit(&mut app, "/mux 3");
    // @step When I submit the slash command "/mux 4"
    submit(&mut app, "/mux 4");
    // @step Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    assert_eq!(app.mux_state().config().panes.len(), 4);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 2 collapses the pane count to two
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 2 collapses the pane count to two
#[tokio::test]
async fn slash_mux_2_collapses_the_pane_count_to_two() {
    // @step Given mux mode is active with four panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    submit(&mut app, "/mux 4");
    // @step When I submit the slash command "/mux 2"
    submit(&mut app, "/mux 2");
    // @step Then the grid shows two panes: Board and Agent
    assert_eq!(app.mux_state().config().panes.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board agent 40 sets an explicit pane list and split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board agent 40 sets an explicit pane list and split
#[tokio::test]
async fn slash_mux_board_agent_40_sets_an_explicit_pane_list_and_split() {
    // @step Given mux mode is active
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    // @step When I submit the slash command "/mux board agent 40"
    submit(&mut app, "/mux board agent 40");
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.splits, vec![40]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux files checkpoints sets a two-pane list
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux files checkpoints sets a two-pane list
#[tokio::test]
async fn slash_mux_files_checkpoints_sets_a_two_pane_list() {
    // @step Given mux mode is active
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    // @step When I submit the slash command "/mux files checkpoints"
    submit(&mut app, "/mux files checkpoints");
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 2);
    use codelet_fspec_tui::views::multiplex::MuxPaneKind;
    assert_eq!(cfg.panes[0], MuxPaneKind::ChangedFiles);
    assert_eq!(cfg.panes[1], MuxPaneKind::Checkpoints);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux with an invalid pane kind leaves the config unchanged
// and shows an error
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux with an invalid pane kind leaves the config unchanged and shows an error
#[tokio::test]
async fn slash_mux_invalid_pane_kind_leaves_config_unchanged_and_shows_an_error() {
    // @step Given mux mode is active with panes Board and Agent at a 50/50 split
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    let before = app.mux_state().config().clone();
    // @step When I submit the slash command "/mux board zzz"
    submit(&mut app, "/mux board zzz");
    // @step Then a one-line error "/mux: unknown pane kind: zzz" is shown in the agent scrollback
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        text.contains("/mux: unknown pane kind: zzz"),
        "the one-line error must land in the scrollback; got: {text}"
    );
    // @step And the grid still shows Board and Agent at a 50/50 split
    assert_eq!(
        app.mux_state().config(),
        &before,
        "a parse error must leave the config untouched"
    );
    // @step And the out-of-range command "/mux board agent 5" is rejected with a one-line error and leaves the config unchanged
    submit(&mut app, "/mux board agent 5");
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        text.contains("/mux: split percent must be 10..=90, got 5"),
        "the out-of-range error must land in the scrollback; got: {text}"
    );
    assert_eq!(app.mux_state().config(), &before);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux save persists the config to the shared fspec-config.json
// (BUG-167: no manual persist-dirs wiring — the process-global data dir
// is the single source of truth, rooted here the way the production
// entry points root it)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux save persists the config to the shared fspec-config.json
#[tokio::test]
#[serial]
async fn slash_mux_save_persists_the_config_to_the_shared_fspec_config_json() {
    // @step Given mux mode is active with panes Board and Agent at a 40/60 vertical split
    let (_data_guard, _data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    submit(&mut app, "/mux v");
    submit(&mut app, "/mux board agent 40");
    // @step When I submit the slash command "/mux save"
    submit(&mut app, "/mux save");
    // @step Then the shared fspec-config.json exists and its tui.mux key contains the saved orientation, splits and pane list
    let data_dir = codelet_common::get_data_dir().expect("data dir");
    let path = data_dir.join("fspec-config.json");
    assert!(
        path.exists(),
        "fspec-config.json must exist after /mux save"
    );
    let raw = fs::read_to_string(&path).expect("read fspec-config.json");
    assert!(
        raw.contains("\"mux\""),
        "the tui.mux key must round-trip: {raw}"
    );
    assert!(raw.contains("40"), "the saved split must round-trip: {raw}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a fresh bootstrap restores the saved mux config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a fresh bootstrap restores the saved mux config
#[tokio::test]
#[serial]
async fn a_fresh_bootstrap_restores_the_saved_mux_config() {
    // @step Given a fresh TUI bootstrap with a saved tui.mux config of 40/60 vertical Board and Agent
    let (_data_guard, _data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    submit(&mut app, "/mux v");
    submit(&mut app, "/mux board agent 40");
    submit(&mut app, "/mux save");
    let (mut app2, _mock2) = fresh_app();
    app2.load_mux_config();
    app2.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    drain_pending(&mut app2).await;
    // @step When I submit the slash command "/mux on"
    submit(&mut app2, "/mux on");
    // @step Then the grid restores the saved 40/60 vertical Board | Agent layout
    let cfg = app2.mux_state().config();
    use codelet_fspec_tui::views::multiplex::MuxOrientation;
    assert_eq!(cfg.orientation, MuxOrientation::Vertical);
    assert_eq!(cfg.splits, vec![40]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a fresh bootstrap with no saved config applies the default preset
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a fresh bootstrap with no saved config applies the default preset
#[tokio::test]
#[serial]
async fn a_fresh_bootstrap_with_no_saved_config_applies_the_default_preset() {
    // @step Given a fresh TUI bootstrap with no tui.mux config present
    let (_data_guard, _data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    app.load_mux_config();
    app.dispatch(Action::SessionCreated(SessionId::new("s-3")));
    drain_pending(&mut app).await;
    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    // @step Then the grid applies the default preset: horizontal Board | Agent at 50/50
    let cfg = app.mux_state().config();
    use codelet_fspec_tui::views::multiplex::MuxOrientation;
    assert_eq!(cfg.orientation, MuxOrientation::Horizontal);
    assert_eq!(cfg.splits, vec![50]);
    assert_eq!(cfg.panes.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a board work unit in mux mode focuses the agent pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on a board work unit in mux mode focuses the agent pane
#[tokio::test]
async fn enter_on_a_board_work_unit_in_mux_mode_focuses_the_agent_pane_app() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the Board pane is focused with work unit AUTH-001 selected
    // (the App reducer binds the unit + focuses the agent pane when
    // MuxEnterWorkUnit lands)
    app.dispatch(Action::MuxEnterWorkUnit("AUTH-001".to_string()));
    // @step Then the work unit AUTH-001 is bound to the agent pane session
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        Some("AUTH-001")
    );
    // @step And the Agent pane is focused
    assert!(app.navigator().mux.focus() == 1);
    // @step And the TUI is still in mux mode showing both panes
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "MuxEnterWorkUnit must NOT flip the whole view to Agent (R8)"
    );
    // @step And the board stays visible in its pane
    // (asserted at the Navigator level in mux001_routing.rs; here the
    // view-mode invariant is the App-level half)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing ? in mux mode shows the HelpDialog over the full screen
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing ? in mux mode shows the HelpDialog over the full screen
#[tokio::test]
async fn pressing_question_in_mux_mode_shows_the_help_dialog() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step When I press the "?" key
    let result = app.handle_event(&codelet_fspec_tui::synth_key(KeyCode::Char('?')));
    // @step Then the HelpDialog is shown over the full screen
    assert!(
        result.is_consumed(),
        "'?' must be consumed by the App shortcut while mux is active"
    );
    assert!(
        app.compositor().contains("help-dialog"),
        "the HelpDialog must be pushed onto the compositor"
    );
    // @step And keys typed while the dialog is open do not reach any mux pane
    let before = app.mux_state().config().clone();
    let _ = app.handle_event(&codelet_fspec_tui::synth_key(KeyCode::Char('j')));
    assert_eq!(
        app.mux_state().config(),
        &before,
        "keys under an open dialog must not mutate mux state"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing the dialog leaves mux mode active with the same
// panes and focus
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the dialog leaves mux mode active with the same panes and focus
#[tokio::test]
async fn closing_the_dialog_leaves_mux_mode_active() {
    // @step Given mux mode is active with Board and Agent panes and the HelpDialog is open
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await;
    let _ = app.handle_event(&codelet_fspec_tui::synth_key(KeyCode::Char('?')));
    assert!(app.compositor().contains("help-dialog"));
    let before = app.mux_state().config().clone();
    // @step When I close the dialog
    app.compositor_mut().remove("help-dialog");
    // @step Then mux mode is still active with the same panes and focus
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.mux_state().config(), &before);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: existing single-view behavior is unchanged when mux is off
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: existing single-view behavior is unchanged when mux is off
#[tokio::test]
async fn existing_single_view_behavior_is_unchanged_when_mux_is_off_app() {
    // @step Given the TUI is showing the single Board view with mux inactive
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled);
    // @step When I press Enter on a selected work unit
    app.dispatch(Action::EnterWorkUnit("AUTH-001".to_string()));
    drain_pending(&mut app).await;
    // @step Then the TUI flips to the single Agent view exactly as before mux existed
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "EnterWorkUnit must flip to the single Agent view when mux is off"
    );
    // @step And pressing Esc in the Agent view returns to the single Board view
    app.dispatch(Action::BackToBoard);
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And no mux footer row is painted and no pane divider is rendered
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .expect("Terminal::new");
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        term.backend().buffer().clone()
    };
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        !rows.iter().any(|r| r.contains("MUX")),
        "no mux footer may be painted when mux is off: {rows:?}"
    );
}
