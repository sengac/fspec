//! MENU-005 — 'u' → menu-bar help re-purpose + help content + snapshot/shape
//! test migration.
//!
//! Feature: spec/features/u-menu-bar-help-re-purpose-help-content-snapshot-shape-test-migration-tag.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences, except the
//! two When/Then pairs the snapshot scenarios define).
//!
//! Harnesses: App + MockBackend (the `menu004_mux_surface.rs` pattern) for
//! the board/agent/mux surface scenarios; direct `HelpDialog` rendering
//! (the `help_dialog_content_rpc397.rs` pattern at 200x60) for the help
//! content scenarios; raw file reads for the source-shape scenarios.
//!
//! NOTE (red phase): these tests compile against the YET-TO-BE-LANDED
//! surface — the dialog titled 'Menu bar' with 9 registry rows, the
//! renamed `components/menu_bar_help.rs` module, the deleted
//! `views/board/board_shortcuts.rs` + `keybinding_shortcuts.rs` files,
//! and the updated help content. They fail until MENU-005 lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

mod common;
use common::{buffer_to_rows, MockBackend};

/// The stable compositor id for the menu-bar help dialog (BOARD-023 id,
/// kept stable this release — MENU-005 R2).
const MENU_BAR_HELP_DIALOG_ID: &str = "board-actions-dialog";

/// The 10 dialog body rows in registry order (MENU-005 R1, MENU-008 R1:
/// Kanban 4 + Tools 2 + Settings 2 + Help 2).
const DIALOG_ROWS: [&str; 10] = [
    ". New Agent",
    "/ Search",
    "D FOUNDATION.md",
    "A Attachments",
    "F Changed Files",
    "C Checkpoints",
    "M Mux",
    "P Providers",
    "? Help",
    "Esc Exit",
];

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn render_app_at(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
}

/// The y of the first buffer row containing `needle` (None when absent).
fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row_text(buf, y).contains(needle))
}

/// True iff any cell on row `y` carries the given background colour —
/// the dialog theme's selected-row inverse highlight uses
/// `bg = accent` (Yellow for this dialog) across the full inner row.
fn row_has_bg(buf: &Buffer, y: u16, bg: Color) -> bool {
    (0..buf.area.width).any(|x| buf[(x, y)].style().bg == Some(bg))
}

/// True iff any cell in the buffer carries the given foreground colour
/// (the dialog border paints in the accent colour).
fn buffer_has_fg(buf: &Buffer, fg: Color) -> bool {
    (0..buf.area.width).any(|x| (0..buf.area.height).any(|y| buf[(x, y)].style().fg == Some(fg)))
}

/// The y-range `(title row, footer row)` of the menu-bar help dialog in
/// a rendered buffer. The title reads "Menu bar" (MENU-005 R2) — located
/// as the LAST row containing it before the footer (the board help lines
/// behind the dialog carry "Menu bar help" only when the help dialog is
/// itself open, which these scenarios never do).
fn dialog_title_footer(buf: &Buffer) -> (u16, u16) {
    let footer = find_row(buf, "↑↓ Navigate").expect("dialog footer row");
    let title = (0..footer)
        .rev()
        .find(|&y| row_text(buf, y).contains("Menu bar"))
        .expect("dialog title row");
    assert!(
        title < footer,
        "title row {title} must precede footer row {footer}"
    );
    (title, footer)
}

/// First buffer row in the dialog's own body rows (`title+2..footer-1`)
/// that contains `needle` (panics when absent).
fn find_dialog_row(buf: &Buffer, needle: &str, title: u16, footer: u16) -> u16 {
    ((title + 2)..(footer - 1))
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("row '{needle}' not found inside the dialog body"))
}

/// Seed `n` open sessions (s-1..s-n, Idle by default) on the App.
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
}

fn make_unit(id: &str, status: &str, attachments: Vec<&str>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: attachments.into_iter().map(String::from).collect(),
        last_state_change_at: None,
    }
}

/// Seed a small board so the board surface has content.
async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog", vec![]),
        make_unit("AUTH-002", "testing", vec![]),
        make_unit("AUTH-003", "blocked", vec![]),
    ]));
    drain_pending(app).await;
}

/// Enter mux mode with the given pane kinds (the `menu004_mux_surface.rs`
/// `enable_mux` helper — MuxConfig + `enable_with_config`, lazy-pane
/// initial loads for Files/Checkpoints only when rendered).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    // BUG-182 R7 parity: initial loads for rendered lazy panes (none for
    // Board | Agent grids).
}

/// Let the MUX-006 focus flash settle into its MUX-007 final frame
/// (the 350ms bottom-to-top scan elapses → the focused pane keeps a
/// static 1-row bar on its top row) so the mux snapshots are
/// deterministic. The flash clock advances 16ms PER RENDERED mux frame
/// (render.rs `advance_flash_clock`), so ~24 renders cover the 350ms
/// window + margin.
fn settle_focus_flash(app: &mut App) {
    for _ in 0..24 {
        let _ = render_app_at(app, 120, 24);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: u opens the Menu bar dialog listing all ten registry rows in order
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_u_opens_the_menu_bar_dialog_listing_all_ten_registry_rows_in_order() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I press the 'u' key
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the 'Menu bar' dialog is open on the compositor with a Yellow accent border
    assert!(
        app.compositor().contains(MENU_BAR_HELP_DIALOG_ID),
        "the menu-bar help dialog must be open; layers={:?}",
        app.compositor().layer_ids()
    );
    let buf = render_app_at(&mut app, 120, 24);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_text(&buf, title).contains("Menu bar"),
        "the dialog title must read 'Menu bar'; got: `{}`",
        row_text(&buf, title)
    );
    assert!(
        buffer_has_fg(&buf, Color::Yellow),
        "the dialog border must paint in the Yellow accent colour"
    );

    // @step And the body lists 10 rows in registry order: . New Agent, / Search, D FOUNDATION.md, A Attachments, F Changed Files, C Checkpoints, M Mux, P Providers, ? Help, Esc Exit — each with its one-line description
    let mut prev = 0u16;
    for label in &DIALOG_ROWS {
        let pos = find_dialog_row(&buf, label, title, footer);
        assert!(
            pos > prev,
            "row order must be top-to-bottom: '{label}' at {pos} not after {prev}"
        );
        prev = pos;
    }
    let dialog_rows: String = (title..=footer)
        .map(|y| row_text(&buf, y))
        .collect::<Vec<_>>()
        .join("\n");
    for desc in [
        "Open/start an agent",
        "Search work units",
        "checkpoints view",
        "changed-files view",
        "browser",
        "mux layout",
        "attachment picker",
        "provider settings view",
        "menu bar help",
        "Confirm exiting fspec",
    ] {
        assert!(
            dialog_rows.contains(desc),
            "rows must carry one-line descriptions (missing '{desc}'); got:\n{dialog_rows}"
        );
    }

    // @step And the first row ('. New Agent') is highlighted
    let y0 = find_dialog_row(&buf, ". New Agent", title, footer);
    assert!(
        row_has_bg(&buf, y0, Color::Yellow),
        "the first row ('. New Agent') must carry the inverse (Yellow bg) highlight"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on the New Agent row substitutes the session snapshot
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_on_the_new_agent_row_substitutes_the_session_snapshot() {
    // @step Given a board with a focused work unit that has an open agent session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    app.board_store_mut()
        .attach_session("AUTH-001", SessionId::new("s-1"));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When I open the 'Menu bar' dialog with 'u' and press Enter on the '. New Agent' row
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dialog is closed
    assert!(
        !app.compositor().contains(MENU_BAR_HELP_DIALOG_ID),
        "the dialog must close on Enter"
    );

    // @step And the Agent view opens for that session — the same target the bare '.' key would open
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Enter on '. New Agent' must flip to the Agent view"
    );
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&SessionId::new("s-1")),
        "the dialog must carry the session snapshot taken at open time"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on the A Attachments row fires the attachment picker
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_on_the_a_attachments_row_fires_the_attachment_picker() {
    // @step Given I am in the single Board view with a selected work unit that has attachments
    let (mut app, _mock) = fresh_app();
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![make_unit(
        "AUTH-001",
        "backlog",
        vec!["spec/attachments/MENU-005/research-help-and-migration.md"],
    )]));
    drain_pending(&mut app).await;
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When I open the 'Menu bar' dialog and press Enter on the 'A Attachments' row
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    for _ in 0..3 {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE)); // cursor 0 → 3 (A Attachments)
    }
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dialog is closed
    assert!(
        !app.compositor().contains(MENU_BAR_HELP_DIALOG_ID),
        "the dialog must close on Enter"
    );

    // @step And the attachment picker opens for the selected work unit — the same outcome as pressing 'a' directly on the board
    assert!(
        app.compositor().contains("attachment-picker-dialog"),
        "Enter on 'A Attachments' must open the attachment picker; layers={:?}",
        app.compositor().layer_ids()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Up from the first row wraps to the last row and Down wraps back
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_up_from_the_first_row_wraps_to_the_last_row_and_down_wraps_back() {
    // @step Given the 'Menu bar' dialog is open with '. New Agent' (the first row) highlighted
    let (mut app, _mock) = fresh_app();
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert!(app.compositor().contains(MENU_BAR_HELP_DIALOG_ID));

    // @step When I press the Up arrow
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));

    // @step Then 'Esc Exit' (the last of the 10 rows) is highlighted
    let buf = render_app_at(&mut app, 120, 24);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_has_bg(
            &buf,
            find_dialog_row(&buf, "Esc Exit", title, footer),
            Color::Yellow
        ),
        "Up from the first row must wrap to the last row ('Esc Exit')"
    );

    // @step When I press the Down arrow
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));

    // @step Then '. New Agent' is highlighted again
    let buf = render_app_at(&mut app, 120, 24);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_has_bg(
            &buf,
            find_dialog_row(&buf, ". New Agent", title, footer),
            Color::Yellow
        ),
        "Down from the last row must wrap to the first row ('. New Agent')"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The board help dialog lists the updated keybinding rows
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_board_help_dialog_lists_the_updated_keybinding_rows() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I open the board help dialog with '?'
    let _ = app.handle_event(&key(KeyCode::Char('?'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert!(app.compositor().contains("help-dialog"));
    // 200x60 fits the full board help list without scrolling (the
    // rpc397 full-content size).
    let buf = render_app_at(&mut app, 200, 60);
    let text = buf_text(&buf);

    // @step Then the help lists the '←/h, →/l' row as 'Cycle columns → menu items → chips'
    assert!(
        text.contains("Cycle columns → menu items → chips"),
        "the board help must list the ring wording; got:\n{text}"
    );
    assert!(
        !text.contains("Switch column"),
        "the old 'Switch column' row must be gone; got:\n{text}"
    );

    // @step And a row 'Enter (item)' described as 'Opens its dropdown menu'
    assert!(
        text.contains("Enter (item)") && text.contains("Opens its dropdown menu"),
        "the board help must list the Enter-on-item row; got:\n{text}"
    );

    // @step And a row 'Enter (chip)' described as 'Opens that session'
    assert!(
        text.contains("Enter (chip)") && text.contains("Opens that session"),
        "the board help must list the Enter-on-chip row; got:\n{text}"
    );

    // @step And the 'u' row reads 'Menu bar help' instead of 'Actions'
    assert!(
        text.contains("u             Menu bar help"),
        "the board help must list the 'u Menu bar help' row; got:\n{text}"
    );
    assert!(
        !text.contains("u             Actions"),
        "the old 'u Actions' help row must be gone; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The agent help lists the empty-input left-arrow menu bar entry
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_agent_help_lists_the_empty_input_left_arrow_menu_bar_entry() {
    // @step Given I am in the Agent view
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new("s-1"),
    )));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When I open the agent help with '/help'
    app.dispatch(codelet_fspec_tui::Action::InputSubmitted(
        "/help".to_string(),
    ));
    drain_pending(&mut app).await;
    assert!(app.compositor().contains("help-dialog"));
    // 200x60 fits the full agent help list without scrolling.
    let buf = render_app_at(&mut app, 200, 60);
    let text = buf_text(&buf);

    // @step Then the help lists a row '← empty input' described as 'Enter the menu bar'
    assert!(
        text.contains("← empty input") && text.contains("Enter the menu bar"),
        "the agent help must list the empty-input Left menu-bar entry; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The board footer hint keeps the ring wording
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_board_footer_hint_keeps_the_ring_wording() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I look at the board footer row
    let buf = render_app_at(&mut app, 120, 24);
    let text = buf_text(&buf);

    // @step Then it reads '← → Cycle Columns → menu items → chips' unchanged (MENU-002 wording)
    assert!(
        text.contains("← → Cycle Columns → menu items → chips"),
        "the board footer must keep the MENU-002 ring wording; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dead chord shim files are deleted and the dialog module renamed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_the_dead_chord_shim_files_are_deleted_and_the_dialog_module_renamed() {
    // @step Given the rust/fspec-tui crate after MENU-005 lands
    let src = common::workspace_root().join("fspec-tui").join("src");

    // @step When a developer scans the views/board/ and components/ directories
    let board_shortcuts = src.join("views").join("board").join("board_shortcuts.rs");
    let keybinding_shortcuts = src
        .join("views")
        .join("board")
        .join("keybinding_shortcuts.rs");
    let board_rs = src.join("views").join("board.rs");
    let menu_bar_help = src.join("components").join("menu_bar_help.rs");

    // @step Then the file rust/fspec-tui/src/views/board/board_shortcuts.rs does NOT exist
    assert!(
        !board_shortcuts.exists(),
        "views/board/board_shortcuts.rs (the SHORTCUTS + CHORD_HINT shim) must be deleted"
    );
    // @step And the file rust/fspec-tui/src/views/board/keybinding_shortcuts.rs does NOT exist
    assert!(
        !keybinding_shortcuts.exists(),
        "views/board/keybinding_shortcuts.rs (the dead chord painter) must be deleted"
    );
    // @step And views/board.rs no longer declares the board_shortcuts module
    let board_body = std::fs::read_to_string(&board_rs).expect("read views/board.rs");
    let board_stripped = common::strip_rust_comments(&board_body);
    assert!(
        !board_stripped.contains("board_shortcuts"),
        "views/board.rs must no longer declare the board_shortcuts module"
    );
    // @step And rust/fspec-tui/src/components/menu_bar_help.rs exists and no longer references the old SHORTCUTS table
    assert!(
        menu_bar_help.exists(),
        "components/menu_bar_help.rs must exist (the renamed dialog module)"
    );
    let help_body = std::fs::read_to_string(&menu_bar_help).expect("read menu_bar_help.rs");
    let help_stripped = common::strip_rust_comments(&help_body);
    assert!(
        !help_stripped.contains("board_shortcuts"),
        "the menu-bar help dialog must not read the old SHORTCUTS shim"
    );
    assert!(
        help_stripped.contains("CATEGORIES"),
        "the dialog must build its rows from the MenuCategories registry"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The agent pane flag-on layout pins the 6-constraint list
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_the_agent_pane_flag_on_layout_pins_the_6_constraint_list() {
    // @step Given the AgentView pane layout in rust/fspec-tui/src/views/agent/pane_render.rs
    let src = common::workspace_root().join("fspec-tui").join("src");
    let pane_render = src.join("views").join("agent").join("pane_render.rs");
    let pane_body = std::fs::read_to_string(&pane_render).expect("read pane_render.rs");
    let agent_rs = src.join("views").join("agent.rs");
    let agent_body = std::fs::read_to_string(&agent_rs).expect("read agent.rs");

    // @step When a developer reads pane_layout_constraints_menu
    let Some(start) = pane_body.find("pub fn pane_layout_constraints_menu") else {
        panic!("pane_render.rs must define pane_layout_constraints_menu")
    };
    // The function's array literal — up to the closing `]` of the
    // constraint list.
    let end = pane_body[start..]
        .find("\n        ]")
        .map(|i| start + i)
        .expect("the menu constraint list must close");
    let body = &pane_body[start..end];

    // @step Then it returns [Length(1), Length(1), Length(role_height), Min(0), Length(1), Length(input_height)] in that order
    let mut pos = 0usize;
    for needle in [
        "Constraint::Length(1),", // header
        "Constraint::Length(1),", // menu bar
        "Constraint::Length(role_height),",
        "Constraint::Min(0),",
        "Constraint::Length(1),",
        "Constraint::Length(input_height),",
    ] {
        let next = body[pos..].find(needle).expect("constraint missing");
        pos += next + needle.len();
    }

    // @step And the flag-off pane_layout_constraints stays the pinned 5-list (rpc013)
    assert!(
        agent_body.contains("pub fn pane_layout_constraints(role_height: u16, input_height: u16) -> [Constraint; 5]")
            || agent_body.contains("pub fn pane_layout_constraints(role_height: u16, input_height: u16) -> [ratatui::layout::Constraint; 5]"),
        "the rpc013-pinned flag-off 5-list must stay byte-identical"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The board surface renders the bar (wide and narrow snapshots)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_board_surface_renders_the_bar_wide_and_narrow_snapshots() {
    // @step Given I am in the single Board view with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await; // both Idle → static chip dots (deterministic frames)
    seed_units(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I render the App at 120x24
    let buf120 = render_app_at(&mut app, 120, 24);

    // @step Then the frame matches the insta snapshot 'menu005_board_120x24'
    insta::assert_yaml_snapshot!("menu005_board_120x24", buffer_to_rows(&buf120));

    // @step When I render the App at 80x24
    let buf80 = render_app_at(&mut app, 80, 24);

    // @step Then the frame matches the insta snapshot 'menu005_board_80x24'
    insta::assert_yaml_snapshot!("menu005_board_80x24", buffer_to_rows(&buf80));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The agent surface renders the bar row under the header (wide and narrow snapshots)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_agent_surface_renders_the_bar_row_under_the_header_wide_and_narrow_snapshots()
{
    // @step Given I am in the single Agent view with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await; // both Idle → static chip dots (deterministic frames)
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new("s-1"),
    )));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When I render the App at 120x24
    let buf120 = render_app_at(&mut app, 120, 24);

    // @step Then the frame matches the insta snapshot 'menu005_agent_120x24' with the bar row directly below the session header
    // Pane geometry (MENU-003): the single-view agent pane paints its
    // SessionHeader on row 0 and the 2-zone menu bar on row 1.
    assert!(
        row_text(&buf120, 1).contains("Board View"),
        "the bar row (directly below the session header, row 1) must paint the 'Board View' item (MENU-007): `{}`",
        row_text(&buf120, 1)
    );
    insta::assert_yaml_snapshot!("menu005_agent_120x24", buffer_to_rows(&buf120));

    // @step When I render the App at 80x24
    let buf80 = render_app_at(&mut app, 80, 24);

    // @step Then the frame matches the insta snapshot 'menu005_agent_80x24'
    insta::assert_yaml_snapshot!("menu005_agent_80x24", buffer_to_rows(&buf80));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The mux surface renders one top-row bar with per-pane suppression (wide and narrow)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_mux_surface_renders_one_top_row_bar_with_per_pane_suppression_wide_and_narrow(
) {
    // @step Given I am in mux mode with the Board | Agent grid and 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await; // both Idle → static chip dots (deterministic frames)
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    assert_eq!(app.active_view(), ViewMode::Mux);
    settle_focus_flash(&mut app);

    // @step When I render the App at 120x24
    let buf120 = render_app_at(&mut app, 120, 24);
    let bar = row_text(&buf120, 0);
    assert!(
        bar.contains("Kanban") && bar.contains("Help"),
        "the mux top row must carry the menu bar: `{bar}`"
    );

    // @step Then the frame matches the insta snapshot 'menu005_mux_120x24' with the bar on the mux top row and no per-pane bars (the board pane's header row 3 is blank)
    insta::assert_yaml_snapshot!("menu005_mux_120x24", buffer_to_rows(&buf120));

    // @step When I render the App at 80x24
    let buf80 = render_app_at(&mut app, 80, 24);

    // @step Then the frame matches the insta snapshot 'menu005_mux_80x24'
    insta::assert_yaml_snapshot!("menu005_mux_80x24", buffer_to_rows(&buf80));
}
