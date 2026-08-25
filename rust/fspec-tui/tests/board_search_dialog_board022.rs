//! BOARD-022 — Board '/' search dialog with Tab-toggled id/title/description modes.
//!
//! Feature: spec/features/board-search-dialog-with-tab-toggled-id-title-description-modes.feature
//!
//! ACDD TESTING phase: these tests assert the BOARD-022 behaviour — a
//! `WorkUnitSearchDialog` component (Priority::Foreground modal on the
//! Compositor, dialog_theme cyan accent) opened by the modifier-free `/`
//! key on the board, filtering the BoardStore snapshot client-side by
//! id / title / description (Tab cycles the mode), and selecting a match
//! with Enter which focuses the board's column + row for that unit and
//! closes the dialog. Esc closes without changing the selection.
//!
//! They compile against the yet-to-be-implemented
//! `WorkUnitSearchDialog` / `WORK_UNIT_SEARCH_DIALOG_ID` /
//! `Action::OpenWorkUnitSearch` / `Action::SelectWorkUnit(String)` /
//! `BoardStore::work_units` / `BoardStore::find` /
//! `BoardStore::select_work_unit` surface, so this file is RED (fails to
//! compile) until the BOARD-022 implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    Action, App, BoardStore, BoardView, Component, EventResult, Theme, WorkUnitSearchDialog,
    WORK_UNIT_SEARCH_DIALOG_ID,
};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::MockBackend;

fn wu(id: &str, status: &str, title: &str, description: Option<&str>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: description.map(str::to_string),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn char_key(c: char) -> Event {
    key(KeyCode::Char(c))
}

fn store_with(units: Vec<WorkUnitInfo>) -> BoardStore {
    let mut store = BoardStore::default();
    store.replace_work_units(units);
    store
}

/// Render the BoardView into a 120x24 TestBackend and return the joined text.
fn render_board(store: &BoardStore) -> String {
    let (view, _rx) = fresh();
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    join_buffer(&buf)
}

fn join_buffer(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: Pressing '/' on the board opens the work-unit search dialog
#[test]
fn pressing_slash_on_the_board_opens_the_work_unit_search_dialog() {
    // @step Given a board with work units "AUTH-001" in backlog and "RPC-100" in implementing
    let store = store_with(vec![
        wu("AUTH-001", "backlog", "User login", None),
        wu("RPC-100", "implementing", "Board grid", None),
    ]);
    let (view, mut rx) = fresh();

    // @step When I press the '/' key on the board
    let result = view.handle_event(&char_key('/'), &store);

    // @step Then the work-unit search dialog is open
    let action = rx.try_recv().expect("Action::OpenWorkUnitSearch on bus");
    assert!(
        matches!(action, Action::OpenWorkUnitSearch),
        "expected OpenWorkUnitSearch, got {action:?}"
    );

    // @step And the dialog shows the Id search mode
    let dialog = WorkUnitSearchDialog::new(Vec::new());
    assert_eq!(
        dialog.mode_label(),
        "id",
        "fresh dialog must default to Id mode"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

/// Scenario: Searching by id filters the result list live
#[test]
fn searching_by_id_filters_the_result_list_live() {
    // @step Given a board with work units "AUTH-001" in backlog, "RPC-100" in implementing and "TUI-110" in done
    let units = vec![
        wu("AUTH-001", "backlog", "User login", None),
        wu("RPC-100", "implementing", "Board grid", None),
        wu("TUI-110", "done", "Key input", None),
    ];
    let store = store_with(units);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I type "auth" into the dialog
    for c in "auth".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists exactly one match "AUTH-001"
    assert_eq!(dialog.matches(), vec!["AUTH-001".to_string()]);
}

/// Scenario: Selecting a match focuses its card on the board and closes the dialog
#[test]
fn selecting_a_match_focuses_its_card_on_the_board_and_closes_the_dialog() {
    // @step Given a board with work units "AUTH-001" in backlog, "RPC-100" in implementing and "TUI-110" in done
    let units = vec![
        wu("AUTH-001", "backlog", "User login", None),
        wu("RPC-100", "implementing", "Board grid", None),
        wu("TUI-110", "done", "Key input", None),
    ];
    let store = store_with(units);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I type "auth" into the dialog
    for c in "auth".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step And I press Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then the board focuses the backlog column
    // @step And the board selects work unit "AUTH-001"
    // The dialog emits Action::SelectWorkUnit(id); App::dispatch applies it
    // to the BoardStore via set_focused_column + select_work_unit.
    let mut app = App::new(Arc::new(MockBackend::new()));
    app.board_store_mut()
        .replace_work_units(vec![
            wu("AUTH-001", "backlog", "User login", None),
            wu("RPC-100", "implementing", "Board grid", None),
            wu("TUI-110", "done", "Key input", None),
        ]);
    app.dispatch(Action::SelectWorkUnit("AUTH-001".to_string()));
    assert_eq!(app.board_store().focused_column(), "backlog");
    assert_eq!(
        app.board_store().selected_work_unit().map(|u| u.id.as_str()),
        Some("AUTH-001")
    );

    // @step And the work-unit search dialog is closed
    // Enter carries the remove callback; running it against the compositor
    // (the way App::handle_event does) must drop the dialog layer.
    let EventResult::Consumed(Some(callback)) = result else {
        panic!("Enter must carry the remove callback");
    };
    let mut compositor = codelet_fspec_tui::Compositor::new();
    compositor.push(Box::new(
        WorkUnitSearchDialog::new(vec![wu("AUTH-001", "backlog", "User login", None)]),
    ));
    assert!(compositor.contains(WORK_UNIT_SEARCH_DIALOG_ID));
    callback(&mut compositor);
    assert!(
        !compositor.contains(WORK_UNIT_SEARCH_DIALOG_ID),
        "remove callback must drop the dialog layer"
    );
}

/// Scenario: Pressing Tab toggles the search mode and re-filters with the same query
#[test]
fn pressing_tab_toggles_the_search_mode_and_refilters_with_the_same_query() {
    // @step Given a board with a work unit "BOARD-022" in backlog whose title contains "search"
    let units = vec![wu(
        "BOARD-022",
        "backlog",
        "Board '/' search dialog",
        Some("find work units"),
    )];
    let store = store_with(units);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I type "search" into the dialog
    for c in "search".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }
    // In Id mode "search" does not match "BOARD-022"
    assert_eq!(dialog.matches(), Vec::<String>::new());

    // @step And I press Tab
    let _ = dialog.handle_event(&key(KeyCode::Tab));

    // @step Then the dialog shows the Title search mode
    assert_eq!(dialog.mode_label(), "title");

    // @step And the dialog lists work unit "BOARD-022" as a match
    assert_eq!(dialog.matches(), vec!["BOARD-022".to_string()]);
}

/// Scenario: Pressing Tab cycles through all three modes and returns to Id
#[test]
fn pressing_tab_cycles_through_all_three_modes_and_returns_to_id() {
    // @step Given the work-unit search dialog is open in Id mode
    let mut dialog = WorkUnitSearchDialog::new(Vec::new());
    assert_eq!(dialog.mode_label(), "id");

    // @step When I press Tab twice
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    let _ = dialog.handle_event(&key(KeyCode::Tab));

    // @step Then the dialog shows the Description search mode
    assert_eq!(dialog.mode_label(), "description");

    // @step When I press Tab once more
    let _ = dialog.handle_event(&key(KeyCode::Tab));

    // @step Then the dialog shows the Id search mode
    assert_eq!(dialog.mode_label(), "id");
}

/// Scenario: Description mode never matches a unit without a description
#[test]
fn description_mode_never_matches_a_unit_without_a_description() {
    // @step Given a board with a work unit "NO-DESC-1" in backlog that has no description
    let units = vec![wu("NO-DESC-1", "backlog", "No description unit", None)];
    let store = store_with(units);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I switch the search mode to Description
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    assert_eq!(dialog.mode_label(), "description");

    // @step And I type "anything" into the dialog
    for c in "anything".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists no matches
    assert_eq!(dialog.matches(), Vec::<String>::new());
}

/// Scenario: A unit with a matching description appears in Description mode
#[test]
fn a_unit_with_a_matching_description_appears_in_description_mode() {
    // @step Given a board with a work unit "DOC-001" in backlog whose description contains "viewer"
    let units = vec![wu(
        "DOC-001",
        "backlog",
        "Docs",
        Some("Open the attachment viewer"),
    )];
    let store = store_with(units);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I switch the search mode to Description
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    let _ = dialog.handle_event(&key(KeyCode::Tab));

    // @step And I type "viewer" into the dialog
    for c in "viewer".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists exactly one match "DOC-001"
    assert_eq!(dialog.matches(), vec!["DOC-001".to_string()]);
}

/// Scenario: Pressing Esc closes the dialog without changing the board selection
#[test]
fn pressing_esc_closes_the_dialog_without_changing_the_board_selection() {
    // @step Given a board with work unit "AUTH-001" in backlog selected
    let store = store_with(vec![wu("AUTH-001", "backlog", "User login", None)]);
    let mut store = store;
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I press Esc
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then the work-unit search dialog is closed
    assert!(
        matches!(result, EventResult::Consumed(Some(_))),
        "Esc must carry the remove callback"
    );

    // @step And the board still selects work unit "AUTH-001"
    assert_eq!(
        store.selected_work_unit().map(|u| u.id.as_str()),
        Some("AUTH-001")
    );
}

/// Scenario: Pressing '/' while the dialog is open does not re-open it
#[test]
fn pressing_slash_while_the_dialog_is_open_does_not_reopen_it() {
    // @step Given the work-unit search dialog is open
    let mut app = App::new(Arc::new(MockBackend::new()));
    app.board_store_mut()
        .replace_work_units(vec![wu("AUTH-001", "backlog", "User login", None)]);
    app.dispatch(Action::OpenWorkUnitSearch);
    assert!(
        app.compositor().contains(WORK_UNIT_SEARCH_DIALOG_ID),
        "dialog must be open after the first OpenWorkUnitSearch"
    );

    // @step When I press the '/' key
    // (The compositor layer consumes '/' at Stage 2; dispatching the
    // action a second time must be idempotent.)
    app.dispatch(Action::OpenWorkUnitSearch);

    // @step Then the work-unit search dialog is open
    assert!(app.compositor().contains(WORK_UNIT_SEARCH_DIALOG_ID));

    // @step And the dialog is not stacked twice
    let count = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == WORK_UNIT_SEARCH_DIALOG_ID)
        .count();
    assert_eq!(count, 1, "dialog must not be stacked twice");
}

/// Scenario: A query with no matches shows the empty-state row and Enter is a no-op
#[test]
fn a_query_with_no_matches_shows_the_empty_state_row_and_enter_is_a_no_op() {
    // @step Given a board with a single work unit "AUTH-001" in backlog
    let store = store_with(vec![wu("AUTH-001", "backlog", "User login", None)]);

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step And I type "zzz-no-such-unit" into the dialog
    for c in "zzz-no-such-unit".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog shows the empty state for the query "zzz-no-such-unit"
    let rendered = render_dialog(&mut dialog);
    assert!(
        rendered.contains("zzz-no-such-unit"),
        "empty-state row must quote the query:\n{rendered}"
    );

    // @step When I press Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then the work-unit search dialog is still open
    assert!(
        !matches!(result, EventResult::Consumed(Some(_))),
        "Enter with zero matches must not close the dialog"
    );
}

/// Scenario: Opening the dialog on an empty board shows the board-is-empty state
#[test]
fn opening_the_dialog_on_an_empty_board_shows_the_board_is_empty_state() {
    // @step Given a board with no work units
    let store = BoardStore::default();

    // @step When I open the search dialog with '/'
    let (view, _rx) = fresh();
    let _ = view.handle_event(&char_key('/'), &store);
    let mut dialog = WorkUnitSearchDialog::new(store.work_units().to_vec());

    // @step Then the dialog shows the board is empty
    let rendered = render_dialog(&mut dialog);
    assert!(
        rendered.contains("board is empty"),
        "empty-board state must be shown:\n{rendered}"
    );

    // @step When I press Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then the work-unit search dialog is still open
    assert!(
        !matches!(result, EventResult::Consumed(Some(_))),
        "Enter with an empty board must not close the dialog"
    );
}

/// Scenario: The board header chord shows the '/' search shortcut
#[test]
fn the_board_header_chord_shows_the_slash_search_shortcut() {
    // @step Given a board with any selection state
    let store = BoardStore::default();

    // @step When the board is rendered
    let rendered = render_board(&store);

    // @step Then the header chord row contains the segment "/ Search"
    assert!(
        rendered.contains("/ Search"),
        "header chord must contain '/ Search':\n{rendered}"
    );
}

/// Scenario: The board help dialog lists the '/' search shortcut
#[test]
fn the_board_help_dialog_lists_the_slash_search_shortcut() {
    use codelet_fspec_tui::components::help_dialog::HelpDialog;

    // @step Given the board help dialog is open
    let mut dialog = HelpDialog::for_board();

    // @step When the help content is inspected
    let buf = render_200x60(&mut dialog);
    let text = join_buffer(&buf);

    // @step Then it lists the '/' key with the description "Search work units"
    assert!(
        text.contains("Search work units"),
        "board help must list the '/' search shortcut:\n{text}"
    );
}

/// Render any Component into a 200x60 TestBackend and return the buffer.
fn render_200x60<C: Component>(component: &mut C) -> Buffer {
    let backend = TestBackend::new(200, 60);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            component.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

/// Render the dialog into a 80x24 TestBackend and return the joined text.
fn render_dialog(dialog: &mut WorkUnitSearchDialog) -> String {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| {
        dialog.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    join_buffer(&buf)
}
