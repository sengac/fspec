// Feature: spec/features/rust-mux-mode.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::sync::Arc;

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

fn render(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
) -> Result<(), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
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
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the established tui093 pattern). Returns the guard
/// (held for the test's duration) + the TempDir.
async fn root_data_dir() -> Result<
    (tokio::sync::MutexGuard<'static, ()>, tempfile::TempDir),
    codelet_test_helpers::TestError,
> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok((guard, tmp))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing j on the focused board pane moves the selection
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing j on the focused board pane moves the selection
#[test]
fn pressing_j_on_the_focused_board_pane_moves_the_selection() -> TestResult {
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing k on the focused board pane leaves the agent
// untouched
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing k on the focused board pane leaves the agent untouched
#[test]
fn pressing_k_on_the_focused_board_pane_leaves_the_agent_untouched() -> TestResult {
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: clicking inside the Agent pane focuses it and routes the
// click
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: clicking inside the Agent pane focuses it and routes the click
#[test]
fn clicking_inside_the_agent_pane_focuses_it_and_routes_the_click() -> TestResult {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent)?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: clicking in a gap outside every pane rect leaves focus
// unchanged
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: clicking in a gap outside every pane rect leaves focus unchanged
#[test]
fn clicking_in_a_gap_outside_every_pane_rect_leaves_focus_unchanged() -> TestResult {
    // @step Given mux mode is active with Board and Agent panes
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    render(&mut nav, &board, &mut agent)?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left cycles focus from ChangedFiles to Agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left cycles focus from ChangedFiles to Agent
#[test]
fn shift_left_cycles_focus_from_changed_files_to_agent() -> TestResult {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux
        .set_layout(MuxLayout::parse("board, agent, files").or_fail("layout")?);
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left cycles focus from Agent to Board
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left cycles focus from Agent to Board
#[test]
fn shift_left_cycles_focus_from_agent_to_board() -> TestResult {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux
        .set_layout(MuxLayout::parse("board, agent, files").or_fail("layout")?);
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left at the first pane stops without wrapping
// (MUX-002 supersedes MUX-001's wrap-around)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left at the first pane stops without wrapping
#[test]
fn shift_left_at_the_first_pane_stops_without_wrapping() -> TestResult {
    // @step Given mux mode is active with three panes Board, Agent and ChangedFiles
    let (mut nav, _rx) = fresh();
    nav.mux
        .set_layout(MuxLayout::parse("board, agent, files").or_fail("layout")?);
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the divider left resizes the split live
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the divider left resizes the split live
#[test]
fn dragging_the_divider_left_resizes_the_split_live() -> TestResult {
    // @step Given mux mode is active with the layout "board, agent" on a 120-column terminal
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent)?;
    let divider = nav.mux.dividers()[0].rect;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: releasing the divider stores the released width as weights
// without snapping back
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: releasing the divider stores the released width as weights without snapping back
#[test]
fn releasing_the_divider_stores_the_released_width_as_weights_without_snapping_back() -> TestResult
{
    // @step Given mux mode is active with the layout "board, agent" on a 120-column terminal
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    // MUX-002: the agent pane only renders when a session is open.
    seed_agent_session(&mut agent);
    render(&mut nav, &board, &mut agent)?;
    let divider = nav.mux.dividers()[0].rect;
    let _ = nav.handle_event(&click(divider.x, divider.y + 5), &board);
    // @step When I release the mouse at a position that would make the Board pane 40 columns
    let _ = nav.handle_event(&drag(40, divider.y + 5), &board);
    let _ = nav.handle_event(&up(40, divider.y + 5), &board);
    // @step Then the Board pane keeps its released 40 columns and the stored layout reads "board:17, agent:33" (34 percent of the pair, reduced — no 64-column minimum, no equal-split reset)
    let rects = nav.mux.pane_rects().to_vec();
    assert_eq!(
        rects[0].width, 40,
        "board pane keeps its released 40 columns, no 64-col minimum"
    );
    assert_eq!(rects[1].width, 119 - 40, "agent pane takes the rest");
    assert_eq!(
        nav.mux.config().layout.to_string(),
        "board:17, agent:33",
        "the released width must be stored as reduced weights"
    );
    // @step And the drag state is cleared after the release
    assert!(!nav.mux.is_dragging());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a board work unit in mux mode focuses the agent
// pane (Navigator half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on a board work unit in mux mode focuses the agent pane
#[test]
fn enter_on_a_board_work_unit_in_mux_mode_focuses_the_agent_pane() -> TestResult {
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
        // BOARD-024: Enter overlays the card chooser (Mux origin); its
        // default `Open in agent` choice emits MuxEnterWorkUnit.
        matches!(
            rx.try_recv().ok(),
            Some(Action::OpenBoardCardChooser { work_unit_id, origin })
                if work_unit_id == "AUTH-001"
                    && origin == codelet_fspec_tui::components::board_card::BoardCardOrigin::Mux
        ),
        "Enter on the board pane in mux mode must open the AUTH-001 card chooser"
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
        let mut term = Terminal::new(TestBackend::new(140, 24)).or_fail("Terminal::new")?;
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .or_fail("draw")?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: existing single-view behavior is unchanged when mux is off
// (Navigator half — R10)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: existing single-view behavior is unchanged when mux is off
#[test]
fn existing_single_view_behavior_is_unchanged_when_mux_is_off() -> TestResult {
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
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
    })
    .or_fail("draw")?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// /mux slash-command grammar parser tests (merged from mux001_parser.rs)
// ─────────────────────────────────────────────────────────────────────────

use codelet_fspec_tui::app::mux_parser::{parse_mux_command, MuxError, MuxSubcommand};
use codelet_fspec_tui::views::multiplex::layout_tree::compute;
use codelet_fspec_tui::views::multiplex::{MuxLayout, MuxPaneKind};
use proptest::{prop_assume, proptest};

/// The layout a `/mux <layout>` line parses to (fails on any other variant).
fn parsed_layout(line: &str) -> Result<MuxLayout, codelet_test_helpers::TestError> {
    match parse_mux_command(line) {
        Ok(MuxSubcommand::Layout(layout)) => Ok(layout),
        other => Err(format!("{line} must parse to a layout, got {other:?}").into()),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux toggles mux mode on with the default preset (parser
// half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enables mux mode with the default preset
#[test]
fn bare_mux_and_on_off_and_help_parse() -> TestResult {
    // @step Given the TUI is showing the single Board view
    // (parser is pure — no TUI needed; the Given is the pre-state the
    // dispatcher applies the parse result to)
    // @step When I submit the slash command "/mux on"
    // MUX-004: bare /mux resolves to Config (opens the MuxConfigDialog),
    // the MUX-001 Toggle behaviour is superseded.
    assert_eq!(parse_mux_command("/mux"), Ok(MuxSubcommand::Config));
    assert_eq!(parse_mux_command("/mux on"), Ok(MuxSubcommand::On));
    // @step Then mux mode is active with the default preset
    // (the dispatcher applies the default preset on /mux on; here we
    // assert the parser hands the dispatcher the right variant)
    assert_eq!(parse_mux_command("/mux off"), Ok(MuxSubcommand::Off));
    assert_eq!(parse_mux_command("/mux help"), Ok(MuxSubcommand::Help));
    // @step And the grid shows the default layout "board, agent": Board on the left and Agent on the right
    // (the default layout string itself parses to two columns)
    let default = parsed_layout("/mux board, agent")?;
    assert_eq!(default.to_string(), "board, agent");
    assert_eq!(
        default.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent]
    );
    // @step And the two columns share the width equally with the Board pane focused on fresh entry (the agent pane is the persisted home focus)
    assert!(
        default.items().iter().all(|item| item.weight() == 1),
        "the default columns carry equal weights"
    );
    // @step And the mux footer row is painted at the bottom of the screen
    // (render-level assertion in the App-level test below)
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
#[test]
fn mux_board_2_agent_3_parses_to_a_weighted_layout() -> TestResult {
    // @step Given mux mode is active
    // (parser-level: assert the pane kinds + weights round-trip)
    // @step When I submit the slash command "/mux board:2, agent:3"
    let layout = parsed_layout("/mux board:2, agent:3")?;
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    assert_eq!(layout.to_string(), "board:2, agent:3");
    assert_eq!(
        layout.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent]
    );
    let weights: Vec<u32> = layout
        .items()
        .iter()
        .map(codelet_fspec_tui::MuxNode::weight)
        .collect();
    assert_eq!(weights, vec![2, 3], "two fifths for Board, three for Agent");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the mux files, checkpoints command sets a two-pane layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the mux files, checkpoints command sets a two-pane layout
#[test]
fn mux_files_checkpoints_parses_to_a_two_pane_layout() -> TestResult {
    // @step Given mux mode is active
    // (parser-level: assert the pane kinds round-trip)
    // @step When I submit the slash command "/mux files, checkpoints"
    let layout = parsed_layout("/mux files, checkpoints")?;
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    assert_eq!(
        layout.leaves(),
        vec![MuxPaneKind::ChangedFiles, MuxPaneKind::Checkpoints]
    );
    assert_eq!(layout.items().len(), 2, "two top-level columns");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux with an invalid pane kind leaves the config unchanged
// and shows an error (parser half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux with an invalid pane kind leaves the config unchanged and shows an error
#[test]
fn invalid_inputs_parse_to_stable_errors() -> TestResult {
    // @step Given mux mode is active with the layout "board, agent"
    // (parser-level: the dispatcher leaves the config untouched on Err —
    // here we assert the error variants and their exact text)
    // @step When I submit the slash command "/mux board, zzz"
    let err = parse_mux_command("/mux board, zzz");
    assert!(matches!(err, Err(MuxError::Layout(_))), "got {err:?}");
    // @step Then a one-line error "/mux: unknown pane 'zzz' at column 8; panes are board, agent, files, checkpoints" is shown in the agent scrollback
    assert_eq!(
        err.map(|_| ()).or_fail_err("an error")?.to_string(),
        "/mux: unknown pane 'zzz' at column 8; panes are board, agent, files, checkpoints"
    );
    // @step And the grid still shows the layout "board, agent"
    // (Err ⇒ dispatcher no-op; asserted in the App-level test)
    // @step And the bad-weight command "/mux board, agent:0" is rejected with a one-line error and leaves the config unchanged
    assert_eq!(
        parse_mux_command("/mux board, agent:0")
            .map(|_| ())
            .or_fail_err("an error")?
            .to_string(),
        "/mux: bad weight '0' at column 14; weights are positive integers"
    );
    // The removed MUX-001 forms name the layout syntax instead.
    for args in ["v", "h", "3", "board agent 40", "board zzz"] {
        assert_eq!(
            parse_mux_command(&format!("/mux {args}")),
            Err(MuxError::RemovedSyntax(args.to_string())),
            "'/mux {args}' is the removed syntax"
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Proptest: the grammar property — every well-formed /mux line parses.
// ─────────────────────────────────────────────────────────────────────────

proptest! {
    /// Property: well-formed weighted layout lines always parse and
    /// round-trip through the canonical string.
    #[test]
    fn well_formed_mux_lines_parse(a in 1u32..=99, b in 1u32..=99) {
        let line = format!("/mux board:{a}, [agent, files]:{b}");
        let parsed = parse_mux_command(&line);
        let Ok(MuxSubcommand::Layout(layout)) = parsed else {
            return Err(proptest::test_runner::TestCaseError::fail(format!(
                "{line} must parse: {parsed:?}"
            )));
        };
        let weight = |w: u32| if w == 1 { String::new() } else { format!(":{w}") };
        assert_eq!(
            layout.to_string(),
            format!("board{}, [agent, files]{}", weight(a), weight(b))
        );
    }
}

proptest! {
    /// Property: an unknown pane kind ALWAYS errors (never silently parses).
    #[test]
    fn unknown_pane_kind_always_errors(kind in "[a-z]{1,12}") {
        prop_assume!(!matches!(
            kind.as_str(),
            "board" | "agent" | "files" | "checkpoints"
            | "on" | "off" | "save" | "default" | "help" | "h" | "v"
        ));
        let line = format!("/mux board, {kind}");
        let expected = format!(
            "/mux: unknown pane '{kind}' at column 8; panes are board, agent, files, checkpoints"
        );
        assert!(matches!(
            parse_mux_command(&line),
            Err(ref e) if e.to_string() == expected
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Mux layout math tests (merged from mux001_layout.rs; MUX-010: the
// layout tree geometry replaces calculate_pane_rects)
// ─────────────────────────────────────────────────────────────────────────

use ratatui::layout::Rect;

fn area(w: u16, h: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

/// The pane rects of `layout` in `area` (every pane shown).
fn rects_of(layout: &str, area: Rect) -> Result<Vec<Rect>, codelet_test_helpers::TestError> {
    let layout = MuxLayout::parse(layout).or_fail("layout")?;
    Ok(compute(area, &layout, &[]).panes)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
// (layout half)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
#[test]
fn explicit_weights_are_respected_below_the_old_minimum() -> TestResult {
    // @step Given mux mode is active
    // @step When I submit the slash command "/mux board:2, agent:3"
    let rects = rects_of("board:2, agent:3", area(120, 24))?;
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    // (MUX-003: the share is honored as-is, no minimum clamp)
    let available = 120u16 - 1; // 1-col divider
    assert_eq!(rects[0].width, available * 2 / 5);
    assert_eq!(rects[1].width, available - rects[0].width);
    // @step When I submit the slash command "/mux files, checkpoints"
    let rects2 = rects_of("files, checkpoints", area(120, 24))?;
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    assert_eq!(rects2.len(), 2);
    assert_eq!(rects2[0].x, 0);
    assert!(rects2[1].x > rects2[0].x);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Boundary cases (design doc §5.4: tiny terminal, all-min splits)
// ─────────────────────────────────────────────────────────────────────────

/// Boundary: a tiny terminal degrades gracefully — no panic, no zero-width panes.
#[test]
fn tiny_terminal_degrades_gracefully() -> TestResult {
    // 66 cols: MUX-003 equal division — 65 available / 2 = 32/33.
    let rects = rects_of("board, agent", area(66, 24))?;
    assert_eq!(rects[0].width, 32);
    assert!(
        rects[1].width >= 1,
        "the second pane must survive with ≥1 col"
    );
    // 10 cols: nothing fits at the board minimum — the layout must
    // still return 2 non-empty rects summing to the available width.
    let tiny = rects_of("board, agent", area(10, 24))?;
    assert_eq!(tiny.len(), 2);
    assert!(tiny[0].width >= 1 && tiny[1].width >= 1);
    Ok(())
}

/// Boundary: a row group divides the rows equally (MUX-003: no minimum
/// pane height).
#[test]
fn a_row_group_divides_the_rows_equally() -> TestResult {
    // 20 rows: 19 available = 9 + 10 (no minimum clamp).
    let rects = rects_of("[board, agent]", area(120, 20))?;
    assert!(rects[0].height >= 1, "board pane must be non-empty");
    assert!(rects[1].height >= 1);
    // 40 rows: 19 + 20 (MUX-003: the last pane absorbs the remainder,
    // no minimum clamp).
    let comfy = rects_of("[board, agent]", area(120, 40))?;
    assert_eq!(comfy[0].height, 19);
    assert_eq!(comfy[1].height, 20);
    assert_eq!(comfy[0].width, 120, "a row spans the full width");
    Ok(())
}

/// Boundary: the non-board panes take their weighted share (MUX-003: no
/// 20-col floor).
#[test]
fn non_board_panes_have_no_20_col_floor() -> TestResult {
    let rects = rects_of("agent:1, files:9", area(40, 24))?;
    // A tenth of 39 available = 3, honored as-is (no floor).
    assert_eq!(rects[0].width, 39 / 10);
    assert_eq!(rects[1].width, 40 - rects[0].width - 1);
    Ok(())
}

/// Boundary: small weights below the old minimums are honored as-is
/// (MUX-003: no minimums); the last pane takes the remainder.
#[test]
fn small_weights_leave_remainder_for_last_pane() -> TestResult {
    // 100 cols: a tenth of 98 = 9, 9, remainder 80.
    let rects = rects_of("board:1, agent:1, checkpoints:8", area(100, 24))?;
    assert_eq!(rects[0].width, 98 / 10);
    assert_eq!(rects[1].width, 98 / 10);
    assert_eq!(rects[2].width, 100 - rects[0].width - rects[1].width - 2);
    Ok(())
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

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

fn submit(app: &mut App, text: &str) {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, text.to_string()));
    }
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
async fn app_with_mux(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(app).await?;
    // MUX-004: bare /mux now opens the config dialog; the explicit
    // "/mux on" keeps the MUX-001 enable-with-default-preset path.
    submit(app, "/mux on");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux on enables mux mode with the default preset
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enables mux mode with the default preset
#[tokio::test]
async fn slash_mux_on_enables_mux_mode_with_the_default_preset() -> TestResult {
    // @step Given the TUI is showing the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux on"
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await?;
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
    // @step And the grid shows the default layout "board, agent": Board on the left and Agent on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "board, agent");
    let rects = app.navigator().mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 2, "the default layout has two panes");
    assert!(rects[0].x < rects[1].x, "Board left of Agent");
    // @step And the two columns share the width equally with the Board pane focused on fresh entry (the agent pane is the persisted home focus)
    assert_eq!(
        (rects[0].width, rects[1].width),
        (59, 60),
        "119 available cells split equally (last column takes the remainder)"
    );
    assert_eq!(app.navigator().mux.focus(), 0, "fresh entry focuses Board");
    assert_eq!(
        cfg.focused_pane, 1,
        "the agent pane is the persisted home focus (fresh entry focuses Board)"
    );
    // @step And the mux footer row is painted at the bottom of the screen
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .or_fail("Terminal::new")?;
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .or_fail("draw")?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux off returns to the pre-mux view
// ─────────────────────────────────────────────────────────────────────────

#[path = "mux001/navigation.rs"]
mod navigation;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board:2, agent:3 sets an explicit layout with weights
#[tokio::test]
async fn slash_mux_board_2_agent_3_sets_an_explicit_layout_with_weights() -> TestResult {
    // @step Given mux mode is active
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    // @step When I submit the slash command "/mux board:2, agent:3"
    submit(&mut app, "/mux board:2, agent:3");
    // @step Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "board:2, agent:3");
    let rects = app.navigator().mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 2);
    assert!(rects[0].x < rects[1].x, "Board left of Agent");
    // 119 available cells: floor(119 * 2 / 5) = 47 for Board, 72 for Agent.
    assert_eq!((rects[0].width, rects[1].width), (47, 72));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the mux files, checkpoints command sets a two-pane layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the mux files, checkpoints command sets a two-pane layout
#[tokio::test]
async fn slash_mux_files_checkpoints_sets_a_two_pane_layout() -> TestResult {
    // @step Given mux mode is active
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    // @step When I submit the slash command "/mux files, checkpoints"
    submit(&mut app, "/mux files, checkpoints");
    // @step Then the grid shows ChangedFiles on the left and Checkpoints on the right
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "files, checkpoints");
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::ChangedFiles, MuxPaneKind::Checkpoints]
    );
    let rects = app.navigator().mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 2);
    assert!(rects[0].x < rects[1].x, "ChangedFiles left of Checkpoints");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux with an invalid pane kind leaves the config unchanged
// and shows an error
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux with an invalid pane kind leaves the config unchanged and shows an error
#[tokio::test]
async fn slash_mux_invalid_pane_kind_leaves_config_unchanged_and_shows_an_error() -> TestResult {
    // @step Given mux mode is active with the layout "board, agent"
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    assert_eq!(app.mux_state().config().layout.to_string(), "board, agent");
    let before = app.mux_state().config().clone();
    // @step When I submit the slash command "/mux board, zzz"
    submit(&mut app, "/mux board, zzz");
    // @step Then a one-line error "/mux: unknown pane 'zzz' at column 8; panes are board, agent, files, checkpoints" is shown in the agent scrollback
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        text.contains(
            "/mux: unknown pane 'zzz' at column 8; panes are board, agent, files, checkpoints"
        ),
        "the one-line error must land in the scrollback; got: {text}"
    );
    // @step And the grid still shows the layout "board, agent"
    assert_eq!(
        app.mux_state().config(),
        &before,
        "a parse error must leave the config untouched"
    );
    assert_eq!(app.mux_state().config().layout.to_string(), "board, agent");
    // @step And the bad-weight command "/mux board, agent:0" is rejected with a one-line error and leaves the config unchanged
    submit(&mut app, "/mux board, agent:0");
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        text.contains("/mux: bad weight '0' at column 14; weights are positive integers"),
        "the bad-weight error must land in the scrollback; got: {text}"
    );
    assert_eq!(app.mux_state().config(), &before);
    Ok(())
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
async fn slash_mux_save_persists_the_config_to_the_shared_fspec_config_json() -> TestResult {
    // @step Given mux mode is active with the layout "[board:2, agent:3]"
    let (_data_guard, _data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    submit(&mut app, "/mux [board:2, agent:3]");
    assert_eq!(
        app.mux_state().config().layout.to_string(),
        "[board:2, agent:3]"
    );
    // @step When I submit the slash command "/mux save"
    submit(&mut app, "/mux save");
    // @step Then the shared fspec-config.json exists and its tui.mux key contains the layout string "[board:2, agent:3]"
    let data_dir = codelet_common::get_data_dir().or_fail("data dir")?;
    let path = data_dir.join("fspec-config.json");
    assert!(
        path.exists(),
        "fspec-config.json must exist after /mux save"
    );
    let raw = fs::read_to_string(&path).or_fail("read fspec-config.json")?;
    let json: serde_json::Value = serde_json::from_str(&raw).or_fail("parse fspec-config.json")?;
    assert_eq!(
        json["tui"]["mux"]["layout"],
        serde_json::json!("[board:2, agent:3]"),
        "the tui.mux key must hold the layout string: {raw}"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a fresh bootstrap restores the saved mux config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a fresh bootstrap restores the saved mux config
#[tokio::test]
#[serial]
async fn a_fresh_bootstrap_restores_the_saved_mux_config() -> TestResult {
    // @step Given a fresh TUI bootstrap with a saved tui.mux layout "[board:2, agent:3]"
    let (_data_guard, _data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    submit(&mut app, "/mux [board:2, agent:3]");
    submit(&mut app, "/mux save");
    let (mut app2, _mock2) = fresh_app();
    app2.load_mux_config();
    app2.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    drain_pending(&mut app2).await?;
    // @step When I submit the slash command "/mux on"
    submit(&mut app2, "/mux on");
    // @step Then the grid restores the saved layout: Board over Agent in one column at 40/60
    let cfg = app2.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "[board:2, agent:3]");
    let rects = app2.navigator().mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].x, rects[1].x, "one column");
    assert!(rects[0].y < rects[1].y, "Board over Agent");
    // 22 available rows (23-row body minus the divider): 8 / 14 (2:3).
    assert_eq!((rects[0].height, rects[1].height), (8, 14));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a fresh bootstrap with no saved config applies the default preset
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a fresh bootstrap with no saved config applies the default preset
#[tokio::test]
#[serial]
async fn a_fresh_bootstrap_with_no_saved_config_applies_the_default_preset() -> TestResult {
    // @step Given a fresh TUI bootstrap with no tui.mux config present
    let (_data_guard, _data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    app.load_mux_config();
    app.dispatch(Action::SessionCreated(SessionId::new("s-3")));
    drain_pending(&mut app).await?;
    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    // @step Then the grid applies the default layout "board, agent" with equal columns
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "board, agent");
    let rects = app.navigator().mux.pane_rects().to_vec();
    assert_eq!(rects.len(), 2);
    assert_eq!((rects[0].width, rects[1].width), (59, 60), "equal columns");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a board work unit in mux mode focuses the agent pane
// ─────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing ? in mux mode shows the HelpDialog over the full screen
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing ? in mux mode shows the HelpDialog over the full screen
#[tokio::test]
async fn pressing_question_in_mux_mode_shows_the_help_dialog() -> TestResult {
    // @step Given mux mode is active with Board and Agent panes
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing the dialog leaves mux mode active with the same
// panes and focus
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the dialog leaves mux mode active with the same panes and focus
#[tokio::test]
async fn closing_the_dialog_leaves_mux_mode_active() -> TestResult {
    // @step Given mux mode is active with Board and Agent panes and the HelpDialog is open
    let (mut app, _mock) = fresh_app();
    app_with_mux(&mut app).await?;
    let _ = app.handle_event(&codelet_fspec_tui::synth_key(KeyCode::Char('?')));
    assert!(app.compositor().contains("help-dialog"));
    let before = app.mux_state().config().clone();
    // @step When I close the dialog
    app.compositor_mut().remove("help-dialog");
    // @step Then mux mode is still active with the same panes and focus
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.mux_state().config(), &before);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: existing single-view behavior is unchanged when mux is off
// ─────────────────────────────────────────────────────────────────────────
