//! TUI-129 board scenarios: the shortcut keys, the board help, the footer
//! and the notices of keys that cannot act.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use codelet_fspec_tui::components::menu_bar::items::CATEGORIES;
use codelet_fspec_tui::App;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{KeyCode, KeyModifiers};

use super::fixture::*;

const ATTACHMENT_PICKER_ID: &str = "attachment-picker-dialog";

/// The board with TUI-001 (one attachment) selected.
async fn board_app() -> Result<App, TestError> {
    let (mut app, _mock) = fresh_app();
    select_card(
        &mut app,
        unit("TUI-001", &["spec/attachments/TUI-001/a.md"]),
    )
    .await?;
    Ok(app)
}

/// What the user can see and what is open: the view, every dialog, the
/// header dropdowns and the painted screen.
fn fingerprint(app: &mut App) -> Result<String, TestError> {
    let rows = screen(app, 120, 30)?;
    Ok(format!(
        "{:?} {:?} {:?} {:?}\n{}",
        app.navigator().active_view,
        app.compositor().layer_ids(),
        app.board_store().workspace_dropdown(),
        app.board_store().daemon_switcher(),
        rows.join("\n")
    ))
}

/// Every menu row whose key is one character: (key, label, row action).
fn single_key_rows() -> Vec<(char, &'static str, codelet_fspec_tui::Action)> {
    CATEGORIES
        .iter()
        .flat_map(|category| category.entries.iter())
        .filter_map(|entry| {
            let mut chars = entry.key.chars();
            match (chars.next(), chars.next()) {
                (Some(key), None) => Some((key, entry.label, entry.action.to_action())),
                _ => None,
            }
        })
        .collect()
}

/// Scenario: Every single-key menu row's key does the same thing on the board
#[tokio::test(start_paused = true)]
async fn every_single_key_menu_rows_key_does_the_same_thing_on_the_board() -> TestResult {
    // @step Given the board is shown with a selected card
    let mut pristine = board_app().await?;
    let untouched = fingerprint(&mut pristine)?;
    let rows = single_key_rows();
    assert_eq!(rows.len(), 12, "the registry's single-key rows: {rows:?}");

    for (key, label, action) in rows {
        let mut by_menu = board_app().await?;
        by_menu.dispatch(action);
        drain(&mut by_menu).await?;
        let expected = fingerprint(&mut by_menu)?;
        assert_ne!(expected, untouched, "the {label} row must change something");

        // @step When the user presses the key of each single-key menu row, in lower and in upper case
        let cases = if key.is_ascii_alphabetic() {
            vec![key.to_ascii_lowercase(), key.to_ascii_uppercase()]
        } else {
            vec![key]
        };
        for typed in cases {
            let mut app = board_app().await?;
            type_char(&mut app, typed).await?;

            // @step Then each key emits the same action as its menu row
            assert_eq!(
                fingerprint(&mut app)?,
                expected,
                "'{typed}' must do what the '{key} {label}' row does"
            );
        }
    }

    // @step And Ctrl+F opens nothing
    // Causality (TEST-042): '?' is pressed after Ctrl+F, so once the board
    // help it opens is up, Ctrl+F has been handled; the screen must equal
    // the one '?' alone gives.
    let mut help_only = board_app().await?;
    type_char(&mut help_only, '?').await?;
    let expected = fingerprint(&mut help_only)?;
    let mut app = board_app().await?;
    press_with(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL).await?;
    type_char(&mut app, '?').await?;
    assert_eq!(fingerprint(&mut app)?, expected, "Ctrl+F must open nothing");
    Ok(())
}

/// Scenario: The board help lists the menu keys exactly as the menu shows them
#[tokio::test(start_paused = true)]
async fn the_board_help_lists_the_menu_keys_exactly_as_the_menu_shows_them() -> TestResult {
    let mut app = board_app().await?;

    // @step When the user opens the board help
    type_char(&mut app, '?').await?;
    let rows = screen(&mut app, 120, 60)?;
    let row = |key: &str, label: &str| format!("{key:<14}{label}");

    // @step Then it lists "D" with "FOUNDATION.md", "A" with "Attachments" and "b" with "New Bot"
    for (key, label) in [
        ("D", "FOUNDATION.md"),
        ("A", "Attachments"),
        ("b", "New Bot"),
    ] {
        assert!(
            shows(&rows, &row(key, label)),
            "board help must list '{key}  {label}':\n{}",
            rows.join("\n")
        );
    }

    // @step And every single-key menu row appears in it with the same key and label
    for (key, label, _) in single_key_rows() {
        assert!(
            shows(&rows, &row(&key.to_string(), label)),
            "board help must list the '{key} {label}' row:\n{}",
            rows.join("\n")
        );
    }

    // @step And it lists "u" with "Menu bar help"
    assert!(shows(&rows, &row("u", "Menu bar help")));
    Ok(())
}

/// Scenario: The board footer names Exit for Esc and the chooser for Enter
#[tokio::test(start_paused = true)]
async fn the_board_footer_names_exit_for_esc_and_the_chooser_for_enter() -> TestResult {
    let mut app = board_app().await?;

    // @step When the board is rendered at width 200 and at width 120
    for width in [200, 120] {
        let rows = screen(&mut app, width, 30)?;
        let footer = rows
            .iter()
            .find(|row| row.contains("Cycle Columns"))
            .or_fail("the board footer row")?;

        // @step Then each footer shows "ESC Exit" and not "ESC Back"
        assert!(footer.contains("ESC Exit"), "width {width}: {footer}");
        assert!(!footer.contains("ESC Back"), "width {width}: {footer}");

        // @step And neither footer says "Work Agent"
        assert!(!footer.contains("Work Agent"), "width {width}: {footer}");
    }
    Ok(())
}

/// Scenario: Pressing a on a card without attachments says so
#[tokio::test(start_paused = true)]
async fn pressing_a_on_a_card_without_attachments_says_so() -> TestResult {
    // @step Given the selected card TUI-001 has no attachments
    let (mut app, _mock) = fresh_app();
    select_card(&mut app, unit("TUI-001", &[])).await?;

    // @step When the user presses "a"
    type_char(&mut app, 'a').await?;

    // @step Then the notice "No attachments on TUI-001" is shown
    assert!(app.compositor().contains(NOTICE_DIALOG_ID));
    let rows = screen(&mut app, 120, 30)?;
    assert!(
        shows(&rows, "No attachments on TUI-001"),
        "{}",
        rows.join("\n")
    );

    // @step And no attachment picker opens
    assert!(!app.compositor().contains(ATTACHMENT_PICKER_ID));
    Ok(())
}

/// Scenario: A card that cannot move says why
#[tokio::test(start_paused = true)]
async fn a_card_that_cannot_move_says_why() -> TestResult {
    // @step Given the board has no workspace yet
    let (mut app, mock) = fresh_app();
    select_card(&mut app, unit("TUI-001", &[])).await?;

    // @step When the user presses "]"
    type_char(&mut app, ']').await?;

    // @step Then a notice says the card cannot move until the workspace is known
    let rows = screen(&mut app, 120, 30)?;
    assert!(
        shows(&rows, "TUI-001 cannot move yet"),
        "{}",
        rows.join("\n")
    );
    assert_eq!(mock.move_work_unit_down_calls(), 0);

    // @step Given the board shows workspace "/ws" and the backend refuses to move TUI-001
    let (mut app, mock) = booted_app("/ws").await?;
    select_card(&mut app, unit("TUI-001", &[])).await?;
    mock.fail_move_work_unit("the board is locked");

    // @step When the user presses "["
    type_char(&mut app, '[').await?;

    // @step Then a notice names TUI-001 and the backend's error
    assert_eq!(mock.move_work_unit_up_calls(), 1);
    let rows = screen(&mut app, 120, 30)?;
    assert!(
        shows(&rows, "Could not move TUI-001: the board is locked"),
        "{}",
        rows.join("\n")
    );
    Ok(())
}
