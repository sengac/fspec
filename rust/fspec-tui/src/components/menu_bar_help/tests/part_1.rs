use super::*;

#[test]
fn new_defaults_cursor_to_first_registry_row() {
    let d = MenuBarHelpDialog::new();
    // R2: the stable BOARD-023 compositor id is kept unchanged.
    assert_eq!(d.id(), MENU_BAR_HELP_DIALOG_ID);
    assert_eq!(d.id(), "board-actions-dialog");
    assert_eq!(d.priority(), Priority::Foreground);
    assert_eq!(d.cursor(), 0);
    // R1: the FULL registry body — MENU-008 + TUI-114: 5 Kanban +
    // 2 Tools + 4 Settings (TUI-123) + 2 Help = 13 rows.
    assert_eq!(ROW_COUNT, 13);
}

#[test]
fn up_from_first_row_wraps_to_last_and_down_wraps_back() {
    let mut d = MenuBarHelpDialog::new();
    let _ = d.handle_event(&key(KeyCode::Up));
    assert_eq!(d.cursor(), 12);
    let _ = d.handle_event(&key(KeyCode::Down));
    assert_eq!(d.cursor(), 0);
}

#[test]
fn enter_on_row_zero_emits_open_create_session_dialog() {
    // BUG-203: the `.` New Agent row is payload-free — it ALWAYS
    // mounts the CreateSessionDialog (BUG-199 agent-bar parity),
    // it never re-enters / resumes a session.
    let mut d = MenuBarHelpDialog::new();
    let _ = d.handle_event(&key(KeyCode::Enter));
    assert!(
        matches!(
            d.take_pending_action(),
            Some(Action::OpenCreateSessionDialog { preselect: None })
        ),
        "row 0 (. New Agent) must emit OpenCreateSessionDialog {{ preselect: None }}"
    );
}

#[test]
fn enter_on_checkpoints_row_emits_open_checkpoints_view() {
    let mut d = MenuBarHelpDialog::new();
    for _ in 0..6 {
        let _ = d.handle_event(&key(KeyCode::Down)); // cursor 0 → 6 (C Checkpoints, Tools)
    }
    let _ = d.handle_event(&key(KeyCode::Enter));
    assert!(matches!(
        d.take_pending_action(),
        Some(Action::OpenCheckpointsView)
    ));
}

#[test]
fn enter_on_help_and_exit_rows_emit_the_new_variants() {
    let mut d = MenuBarHelpDialog::new();
    for _ in 0..11 {
        let _ = d.handle_event(&key(KeyCode::Down)); // cursor → 11 (? Help; TUI-123)
    }
    let _ = d.handle_event(&key(KeyCode::Enter));
    assert!(matches!(
        d.take_pending_action(),
        Some(Action::OpenBoardHelp)
    ));
    let mut d2 = MenuBarHelpDialog::new();
    for _ in 0..12 {
        let _ = d2.handle_event(&key(KeyCode::Down)); // cursor → 12 (Esc Exit)
    }
    let _ = d2.handle_event(&key(KeyCode::Enter));
    assert!(matches!(
        d2.take_pending_action(),
        Some(Action::OpenBoardExitConfirmation)
    ));
}

#[test]
fn u_and_unhandled_keys_are_consumed_no_op() {
    let mut d = MenuBarHelpDialog::new();
    assert!(d.handle_event(&key(KeyCode::Char('u'))).is_consumed());
    assert!(d.handle_event(&key(KeyCode::Char('U'))).is_consumed());
    assert!(d.handle_event(&key(KeyCode::Char('j'))).is_consumed());
    // TUI-129: a row's own key (f, ?) runs that row; these keys show in no row.
    assert!(d.handle_event(&key(KeyCode::Char('x'))).is_consumed());
    assert!(d.handle_event(&key(KeyCode::Char(']'))).is_consumed());
    assert_eq!(d.cursor(), 0);
    assert!(d.take_pending_action().is_none());
}

#[test]
fn esc_closes_without_emitting_any_action() {
    let mut d = MenuBarHelpDialog::new();
    assert!(d.handle_event(&key(KeyCode::Esc)).is_consumed());
    assert!(d.take_pending_action().is_none());
}

#[test]
fn menu_bar_help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
    let mut dialog = MenuBarHelpDialog::new();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            dialog.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    let buf = terminal.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::with_capacity(buf.area.width as usize);
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    insta::assert_yaml_snapshot!("menu_bar_help_dialog__centered_popup_80x24", rows);
}
