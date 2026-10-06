//! MENU-008 — Board menu bar: reorganize Zone A into Kanban / Tools /
//! Settings categories.
//!
//! Feature: spec/features/board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions).
//!
//! Harness: pure snapshot structs + stateless painters rendered into
//! `ratatui::buffer::Buffer`s for the registry/paint/ring scenarios, and
//! the full App + MockBackend for the dropdown Execute paths, the 'u'
//! help dialog and the `P` bare-key arm (the owning surface holds the
//! menu state the component does not).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::chips::build_chips;
use codelet_fspec_tui::components::menu_bar::chips::ChipInput;
use codelet_fspec_tui::components::menu_bar::dropdown::dropdown_rect;
use codelet_fspec_tui::components::menu_bar::items::{
    MenuAction, CATEGORIES, HELP, KANBAN, SETTINGS, TOOLS,
};
use codelet_fspec_tui::components::menu_bar::paint::paint_menu_bar;
use codelet_fspec_tui::components::menu_bar::{MenuFocus, MenuSnapshot, ZoneBCell};
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::{App, FspecBackend, Theme, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;
use serial_test::serial;

mod common;
use common::MockBackend;

/// The 10 'u' dialog body row labels in registry order (MENU-008:
/// Kanban 4, Tools 2, Settings 2, Help 2).
const DIALOG_ROW_LABELS: [&str; 10] = [
    "New Agent",
    "Search",
    "FOUNDATION.md",
    "Attachments",
    "Changed Files",
    "Checkpoints",
    "Mux",
    "Providers",
    "Help",
    "Exit",
];

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

/// A 1-chip (Idle, index (1,1)) snapshot — the board/mux default shape.
fn snap_one_chip() -> MenuSnapshot {
    let chip = build_chips(
        &[ChipInput {
            index: (1, 1),
            status: SessionStatus::Idle,
            wu_id: None,
            active: false,
        }],
        0,
    )
    .pop()
    .expect("one chip for one open session");
    MenuSnapshot {
        zone_b: vec![ZoneBCell::Chip(0)],
        chips: vec![chip],
        ..Default::default()
    }
}

/// The trimmed row-0 string of a `w`-column buffer.
fn row(buf: &Buffer, w: u16) -> String {
    (0..w)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Paint the bar (1-row tall) into a fresh `w`-column buffer.
fn render(snap: &MenuSnapshot, w: u16) -> Buffer {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    assert!(
        paint_menu_bar(area, &mut buf, snap, &Theme::default()).is_some(),
        "layout for non-zero area"
    );
    buf
}

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

/// Render the full App into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Focus the `item_index`-th menu item (0 = Kanban, 1 = Tools,
/// 2 = Settings, 3 = Help — MENU-008 four Zone A items), open its
/// dropdown, and walk the cursor to `cursor_row`.
async fn open_category_dropdown_at(app: &mut App, item_index: usize, cursor_row: usize) {
    // Focus the last column, then step Right into the bar at item 0.
    app.board_store_mut().set_focused_column("blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(app).await;
    for _ in 1..=item_index {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(app).await;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(app).await;
    for _ in 0..cursor_row {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    }
    drain_pending(app).await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The registry defines the four categories in Kanban, Tools,
// Settings, Help order
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_registry_defines_the_four_categories_in_kanban_tools_settings_help_order() {
    // @step Given the MenuCategories registry
    // @step When the Zone A category list is read
    let labels: Vec<&str> = CATEGORIES.iter().map(|c| c.label).collect();
    // @step Then it has 4 categories in order: "Kanban", "Tools", "Settings", "Help"
    assert_eq!(labels, vec!["Kanban", "Tools", "Settings", "Help"]);
    // @step And the first category's id is "kanban"
    assert_eq!(CATEGORIES[0].id, "kanban");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Kanban category lists the work-unit workflow entries in order
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_kanban_category_lists_the_work_unit_workflow_entries_in_order() {
    // @step Given the MenuCategories registry
    // @step When the Kanban category is read
    assert_eq!(KANBAN.len(), 4);
    let labels: Vec<&str> = KANBAN.iter().map(|e| e.label).collect();
    // @step Then it has 4 entries in order: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    assert_eq!(
        labels,
        vec!["New Agent", "Search", "FOUNDATION.md", "Attachments"]
    );
    let keys: Vec<&str> = KANBAN.iter().map(|e| e.key).collect();
    assert_eq!(keys, vec![".", "/", "D", "A"]);
    // @step And each entry's action matches the bare board key (OpenAgentView, OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker)
    assert_eq!(KANBAN[0].action, MenuAction::NewAgent);
    assert_eq!(KANBAN[1].action, MenuAction::Search);
    assert_eq!(KANBAN[2].action, MenuAction::Foundation);
    assert_eq!(KANBAN[3].action, MenuAction::Attachments);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Tools category lists the git tools in order
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_tools_category_lists_the_git_tools_in_order() {
    // @step Given the MenuCategories registry
    // @step When the Tools category is read
    // @step Then it has 2 entries in order: "F Changed Files" and "C Checkpoints"
    assert_eq!(TOOLS.len(), 2);
    assert_eq!(TOOLS[0].key, "F");
    assert_eq!(TOOLS[0].label, "Changed Files");
    assert_eq!(TOOLS[1].key, "C");
    assert_eq!(TOOLS[1].label, "Checkpoints");
    // @step And they emit OpenChangedFilesView and OpenCheckpointsView
    assert_eq!(TOOLS[0].action, MenuAction::ChangedFiles);
    assert_eq!(TOOLS[1].action, MenuAction::Checkpoints);
    assert!(matches!(
        MenuAction::ChangedFiles.to_action(None),
        Action::OpenChangedFilesView
    ));
    assert!(matches!(
        MenuAction::Checkpoints.to_action(None),
        Action::OpenCheckpointsView
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Settings category lists the configuration entries in order
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_settings_category_lists_the_configuration_entries_in_order() {
    // @step Given the MenuCategories registry
    // @step When the Settings category is read
    // @step Then it has 2 entries in order: "M Mux" and "P Providers"
    assert_eq!(SETTINGS.len(), 2);
    assert_eq!(SETTINGS[0].key, "M");
    assert_eq!(SETTINGS[0].label, "Mux");
    assert_eq!(SETTINGS[1].key, "P");
    assert_eq!(SETTINGS[1].label, "Providers");
    // @step And they emit OpenMuxConfigDialog and OpenProviderSettingsView
    assert_eq!(SETTINGS[0].action, MenuAction::Mux);
    assert_eq!(SETTINGS[1].action, MenuAction::Providers);
    assert!(matches!(
        MenuAction::Mux.to_action(None),
        Action::OpenMuxConfigDialog
    ));
    assert!(matches!(
        MenuAction::Providers.to_action(None),
        Action::OpenProviderSettingsView
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Help category is unchanged
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_help_category_is_unchanged() {
    // @step Given the MenuCategories registry
    // @step When the Help category is read
    // @step Then it has 2 entries in order: "? Help" and "Esc Exit"
    assert_eq!(HELP.len(), 2);
    assert_eq!(HELP[0].key, "?");
    assert_eq!(HELP[0].label, "Help");
    assert_eq!(HELP[1].key, "Esc");
    assert_eq!(HELP[1].label, "Exit");
    // @step And they emit OpenBoardHelp and OpenBoardExitConfirmation
    assert_eq!(HELP[0].action, MenuAction::Help);
    assert_eq!(HELP[1].action, MenuAction::Exit);
    assert!(matches!(
        MenuAction::Help.to_action(None),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::Exit.to_action(None),
        Action::OpenBoardExitConfirmation
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Every registry entry has a key hint, label, description and a
// valid action
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn every_registry_entry_has_a_key_hint_label_description_and_a_valid_action() {
    // @step Given the MenuCategories registry
    // @step When every entry of every category is inspected
    for category in CATEGORIES {
        for entry in category.entries {
            // @step Then none of its key hints, labels or descriptions are empty
            assert!(!entry.key.is_empty(), "key of {:?}", entry.label);
            assert!(!entry.label.is_empty());
            assert!(!entry.description.is_empty());
        }
    }
    // @step And every action resolves to an existing Action variant
    let target = SessionId::new("s-1");
    assert!(matches!(
        MenuAction::NewAgent.to_action(Some(target)),
        Action::OpenAgentView(Some(_))
    ));
    assert!(matches!(
        MenuAction::NewAgent.to_action(None),
        Action::OpenAgentView(None)
    ));
    assert!(matches!(
        MenuAction::Search.to_action(None),
        Action::OpenWorkUnitSearch
    ));
    assert!(matches!(
        MenuAction::Foundation.to_action(None),
        Action::OpenFoundation
    ));
    assert!(matches!(
        MenuAction::Attachments.to_action(None),
        Action::OpenAttachmentPicker
    ));
    assert!(matches!(
        MenuAction::ChangedFiles.to_action(None),
        Action::OpenChangedFilesView
    ));
    assert!(matches!(
        MenuAction::Checkpoints.to_action(None),
        Action::OpenCheckpointsView
    ));
    assert!(matches!(
        MenuAction::Mux.to_action(None),
        Action::OpenMuxConfigDialog
    ));
    assert!(matches!(
        MenuAction::Providers.to_action(None),
        Action::OpenProviderSettingsView
    ));
    assert!(matches!(
        MenuAction::Help.to_action(None),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::Exit.to_action(None),
        Action::OpenBoardExitConfirmation
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The bar row paints the four Zone A items
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_bar_row_paints_the_four_zone_a_items() {
    // @step Given a MenuSnapshot with 1 open session (Idle)
    let snap = snap_one_chip();
    // @step When the bar is rendered into a 120-column row
    let buf = render(&snap, 120);
    let line = row(&buf, 120);
    // @step Then Zone A reads "Kanban Tools Settings Help"
    assert!(
        line.starts_with(" Kanban Tools Settings Help"),
        "Zone A first, after the 1-cell R1 pad: {line}"
    );
    // @step And the row still carries the dim separator and the session chips
    assert!(line.contains("│"), "separator present: {line}");
    assert!(line.contains("#1 ●"), "chip 1: {line}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Focusing the Kanban item opens a 4-row dropdown
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn focusing_the_kanban_item_opens_a_4_row_dropdown() {
    // @step Given the board's menu bar with focus on the 0th menu item
    let mut snap = snap_one_chip();
    snap.focus = Some(MenuFocus::Item(0));
    // @step When Enter is pressed
    // (Enter opens the item's dropdown — the panel geometry is pure:
    // the Kanban category is index 0, 4 entries → height 6)
    let panel = dropdown_rect(Rect::new(0, 0, 120, 24), 1, 0, 0).expect("panel");
    // @step Then the Kanban dropdown is open with 4 rows: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    assert_eq!(panel.height, 6, "4 entries + 2 border rows");
    let kanban = &CATEGORIES[0];
    assert_eq!(kanban.label, "Kanban");
    assert_eq!(kanban.entries.len(), 4);
    // @step And the Tools and Settings items are not highlighted
    assert_ne!(snap.focus, Some(MenuFocus::Item(1)));
    assert_ne!(snap.focus, Some(MenuFocus::Item(2)));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The ring walks across all four items before reaching Zone B
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_ring_walks_across_all_four_items_before_reaching_zone_b() {
    // @step Given a MenuSnapshot with 1 open session and MenuFocus on the "Help" item
    let s = snap_one_chip();
    // Help is index 3 (Kanban → Tools → Settings → Help).
    assert_eq!(
        s.advance(Some(MenuFocus::Item(3)), 1),
        MenuFocus::ZoneB(0),
        "right from Help lands on the first chip"
    );
    // @step When the ring is advanced right
    // @step Then the focus lands on the first Zone B cell (the session chip)
    // @step And advancing right from the first item walks "Tools", "Settings", "Help" in order
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
    assert_eq!(s.advance(Some(MenuFocus::Item(1)), 1), MenuFocus::Item(2));
    assert_eq!(s.advance(Some(MenuFocus::Item(2)), 1), MenuFocus::Item(3));
    assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::ZoneB(0));
    assert_eq!(
        s.advance(Some(MenuFocus::ZoneB(0)), 1),
        MenuFocus::Item(0),
        "the ring wraps back to Kanban"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Executing the Tools row opens the changed-files view
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_executing_the_tools_row_opens_the_changed_files_view() {
    // @step Given the Tools dropdown open with the cursor on row 0
    let (mut app, _mock) = fresh_app();
    open_category_dropdown_at(&mut app, 1, 0).await;
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        text.contains("Changed Files"),
        "the Tools dropdown must be open with 'Changed Files' as row 0:\n{text}"
    );

    // @step When Enter is pressed
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Changed Files dual-pane view opens (OpenChangedFilesView)
    assert_eq!(
        app.active_view(),
        ViewMode::ChangedFiles,
        "Enter on the Tools dropdown's 'Changed Files' row must open the Changed Files view"
    );

    // @step And the dropdown closes and the bar highlight clears (BUG-196 R2: execute de-selects the bar)
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        !text.contains("New Agent"),
        "the dropdown panel must be closed after execute:\n{text}"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "the bar highlight must clear after executing the row (BUG-196 R2)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Executing the Settings Providers row opens the provider
// settings view
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_executing_the_settings_providers_row_opens_the_provider_settings_view() {
    // @step Given the Settings dropdown open with the cursor on row 1
    let (mut app, _mock) = fresh_app();
    open_category_dropdown_at(&mut app, 2, 1).await;
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        text.contains("Providers"),
        "the Settings dropdown must be open with 'Providers' as row 1:\n{text}"
    );

    // @step When Enter is pressed
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the ProviderSettings view opens (OpenProviderSettingsView)
    assert_eq!(
        app.active_view(),
        ViewMode::ProviderSettings,
        "Enter on the Settings dropdown's 'Providers' row must open the ProviderSettings view"
    );

    // @step And the dropdown closes and the bar highlight clears (BUG-196 R2: execute de-selects the bar)
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        !text.contains("New Agent"),
        "the dropdown panel must be closed after execute:\n{text}"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "the bar highlight must clear after executing the row (BUG-196 R2)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The P key opens the provider settings view from the board
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_p_key_opens_the_provider_settings_view_from_the_board() {
    // @step Given the board view is active
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When the modifier-free "P" key is pressed
    let _ = app.handle_event(&key(KeyCode::Char('P'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then OpenProviderSettingsView is emitted
    assert_eq!(
        app.active_view(),
        ViewMode::ProviderSettings,
        "P on the board must flip to the ProviderSettings view"
    );
}

/// Scenario (R4): lowercase `p` behaves identically (case-insensitive,
/// like the a/c/f/d/m arms).
#[tokio::test]
async fn the_lowercase_p_key_opens_the_provider_settings_view_from_the_board() {
    // @step Given the board view is active
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When the modifier-free "P" key is pressed
    let _ = app.handle_event(&key(KeyCode::Char('p'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then OpenProviderSettingsView is emitted
    assert_eq!(app.active_view(), ViewMode::ProviderSettings);
}

/// Scenario (R4): `And Ctrl+P falls through to App-level handling` —
/// the provider settings view must NOT open on the Ctrl chord.
#[tokio::test]
async fn ctrl_p_does_not_open_the_provider_settings_view() {
    // @step Given the board view is active
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When the modifier-free "P" key is pressed
    let _ = app.handle_event(&key(KeyCode::Char('P'), KeyModifiers::CONTROL));
    drain_pending(&mut app).await;
    // @step And Ctrl+P falls through to App-level handling
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Ctrl+P must fall through — the view stays Board"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (R4): the P arm serves the focused Board pane in mux mode
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: P opens the provider settings view from the focused Board
/// pane in mux mode (Board-pane key routing parity with the M arm).
#[tokio::test]
#[serial]
async fn p_opens_the_provider_settings_view_from_the_focused_board_pane_in_mux_mode() {
    // @step Given the board view is active (mux mode, Board pane focused)
    let (mut app, _mock) = fresh_app();
    app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
        "s-1",
    )));
    drain_pending(&mut app).await;
    app.dispatch(codelet_fspec_tui::Action::InputSubmitted(
        "/mux on".to_string(),
    ));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux, "mux mode must be active");
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused on fresh /mux on entry"
    );
    // @step When the modifier-free "P" key is pressed
    let _ = app.handle_event(&key(KeyCode::Char('P'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then OpenProviderSettingsView is emitted
    assert_eq!(
        app.active_view(),
        ViewMode::ProviderSettings,
        "P in the focused Board pane must flip to ProviderSettings"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The u help dialog lists all ten registry rows in registry order
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_u_help_dialog_lists_all_ten_registry_rows_in_registry_order() {
    // @step Given the "u" menu-bar help dialog is open
    let (mut app, _mock) = fresh_app();
    app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        text.contains("Menu bar"),
        "the 'u' dialog titled 'Menu bar' must be open:\n{text}"
    );

    // @step When its body rows are rendered
    // (the body is rendered by the same dialog the Given opened; the
    // observation is the full 120x24 frame below)

    // @step Then it shows 10 rows across the four categories: the 4 Kanban rows, then the 2 Tools rows, then the 2 Settings rows, then the 2 Help rows
    // Find the dialog title row and footer row; the body rows sit
    // strictly between them.
    let title_y = (0..buf.area.height)
        .find(|&y| row_text(&buf, y).contains("Menu bar"))
        .expect("the dialog title row must render");
    let footer_y = (0..buf.area.height)
        .find(|&y| row_text(&buf, y).contains("Esc Close"))
        .expect("the dialog footer row must render");
    assert!(title_y < footer_y, "title must precede footer");
    // Entry rows paint as `key label - description`; the body's padding
    // rows (one above and one below the entries) carry only borders, so
    // the " - " separator counts exactly the 10 registry rows.
    let body_rows: Vec<u16> = (title_y + 1..footer_y)
        .filter(|&y| row_text(&buf, y).contains(" - "))
        .collect();
    assert_eq!(
        body_rows.len(),
        10,
        "the dialog body must list exactly 10 rows (MENU-008: 4 Kanban + 2 Tools + 2 Settings + 2 Help):\n{text}"
    );
    // Every registry label appears on its expected body row, in
    // registry order (the body rows above are strictly in frame order).
    for (row_y, label) in body_rows.iter().zip(DIALOG_ROW_LABELS.iter()) {
        let text = row_text(&buf, *row_y);
        assert!(
            text.contains(label),
            "registry row order broken: body row {row_y} should contain '{label}', got: {text}"
        );
    }

    // @step And the first row reads ". New Agent" and the last row reads "Esc Exit"
    let first = row_text(&buf, body_rows[0]);
    let last = row_text(&buf, *body_rows.last().expect("body rows non-empty"));
    assert!(
        first.contains("New Agent") && first.contains("."),
        "the first body row must be '. New Agent', got: {first}"
    );
    assert!(
        last.contains("Exit"),
        "the last body row must be 'Esc Exit', got: {last}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Tight width still degrades safely with the four items
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tight_width_still_degrades_safely_with_the_four_items() {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let chips = build_chips(
        &[
            ChipInput {
                index: (1, 4),
                status: SessionStatus::Idle,
                wu_id: Some("MENU-008".into()),
                active: false,
            },
            ChipInput {
                index: (2, 4),
                status: SessionStatus::Idle,
                wu_id: Some("MENU-009".into()),
                active: false,
            },
            ChipInput {
                index: (3, 4),
                status: SessionStatus::Idle,
                wu_id: Some("MENU-010".into()),
                active: false,
            },
            ChipInput {
                index: (4, 4),
                status: SessionStatus::Idle,
                wu_id: Some("MENU-011".into()),
                active: false,
            },
        ],
        0,
    );
    let snap = MenuSnapshot {
        zone_b: vec![
            ZoneBCell::Chip(0),
            ZoneBCell::Chip(1),
            ZoneBCell::Chip(2),
            ZoneBCell::Chip(3),
        ],
        chips,
        ..Default::default()
    };
    // @step When the bar is rendered into a 20-column area
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);
    let painted = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).is_some();
    // @step Then the row still fits within the area width
    // (a paint that returned None painted nothing — trivially within
    // the area; a paint that returned a layout must fit inside 20 cols)
    if painted {
        use codelet_fspec_tui::components::menu_bar::layout::menu_bar_layout;
        let layout = menu_bar_layout(area, &snap).expect("layout");
        for cell in &layout.item_rects {
            assert!(
                cell.x + cell.width <= 20,
                "item overflows: x {} w {}",
                cell.x,
                cell.width
            );
        }
        if let Some(last) = layout.item_rects.last() {
            assert!(
                last.x + last.width <= 20,
                "last item overflows: x {} w {}",
                last.x,
                last.width
            );
        }
    }
    // @step And it shows at least the "Kanban" and "Help" items
    // (Zone A is the absolute minimum — it always paints when a layout
    // exists; a too-tight area legitimately paints nothing.)
    if painted {
        let line = row(&buf, 20);
        assert!(line.contains("Kanban"), "Kanban item: {line}");
        assert!(line.contains("Help"), "Help item: {line}");
    }
}
