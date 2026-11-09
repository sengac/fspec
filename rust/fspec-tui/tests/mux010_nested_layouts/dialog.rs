//! MUX-010 config-dialog scenarios: the layout-string field, the live
//! preview, the exact errors under the field and the dialog's keys.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{KeyCode, KeyModifiers};

use super::config::{root_data_dir, saved_mux};
use super::support::{app_with_session, key, layout_string, press, render, submit, type_text};

const DIALOG_ID: &str = "mux-config-dialog";

/// The (row, column) of the first `needle` in `rows`, scanning top down.
fn find(rows: &[String], needle: &str) -> Option<(usize, usize)> {
    rows.iter().enumerate().find_map(|(y, row)| {
        row.find(needle)
            .map(|byte| (y, row[..byte].chars().count()))
    })
}

fn row_with<'a>(rows: &'a [String], needle: &str) -> Result<&'a String, TestError> {
    Ok(rows
        .iter()
        .find(|row| row.contains(needle))
        .or_fail(needle)?)
}

async fn open_dialog(app: &mut codelet_fspec_tui::App) -> TestResult {
    render(app, 120, 40)?;
    submit(app, "/mux").await?;
    assert!(
        app.compositor().contains(DIALOG_ID),
        "bare /mux opens the dialog"
    );
    Ok(())
}

/// Scenario: The config dialog edits the layout string with a live preview
#[tokio::test(start_paused = true)]
async fn the_config_dialog_edits_the_layout_string_with_a_live_preview() -> TestResult {
    // @step Given the terminal is 120x40 and mux is off
    let (mut app, _mock) = app_with_session().await?;
    // @step When the user opens the mux config dialog with bare /mux
    open_dialog(&mut app).await?;
    let rows = render(&mut app, 120, 40)?;
    // @step Then the dialog shows the layout field "board, agent", a preview of two boxes labelled board and agent, the Enabled toggle and Apply and Save buttons
    row_with(&rows, "Layout  board, agent")?;
    let (board_y, board_x) = find(&rows, "│board").or_fail("a board box")?;
    let (agent_y, agent_x) = find(&rows, "│agent").or_fail("an agent box")?;
    assert_eq!(board_y, agent_y, "the two boxes sit side by side");
    assert!(agent_x > board_x);
    row_with(&rows, "Enabled  Off")?;
    row_with(&rows, "[ Apply ]")?;
    row_with(&rows, "[ Save ]")?;
    // @step And the dialog has no Orientation row and no per-pane rows
    assert!(!rows
        .iter()
        .any(|r| r.contains("Orientation") || r.contains("Pane 1")));
    // @step When the user types ", [files, checkpoints]" at the end of the field
    type_text(&mut app, ", [files, checkpoints]").await?;
    let rows = render(&mut app, 120, 40)?;
    // @step Then the preview redraws as three columns, the last split into a files box over a checkpoints box
    row_with(&rows, "Layout  board, agent, [files, checkpoints]")?;
    let (files_y, files_x) = find(&rows, "│files").or_fail("a files box")?;
    let (check_y, check_x) = find(&rows, "│checkpoints").or_fail("a checkpoints box")?;
    let (agent_y, agent_x) = find(&rows, "│agent").or_fail("the agent box")?;
    assert_eq!(
        files_x, check_x,
        "files and checkpoints share the third column"
    );
    assert!(
        files_x > agent_x,
        "the third column is right of the agent box"
    );
    assert!(
        check_y > files_y && files_y == agent_y,
        "checkpoints is below files"
    );
    Ok(())
}

/// Scenario: The config dialog shows parse errors under the field and refuses to apply them
#[tokio::test(start_paused = true)]
async fn the_config_dialog_shows_parse_errors_and_refuses_to_apply_them() -> TestResult {
    let (_guard, data) = root_data_dir().await?;
    // @step Given the mux config dialog is open with the layout field "board, agent"
    let (mut app, _mock) = app_with_session().await?;
    open_dialog(&mut app).await?;
    // @step When the user types "]" at the end of the field
    type_text(&mut app, "]").await?;
    let rows = render(&mut app, 120, 40)?;
    // @step Then the line under the field reads "unbalanced bracket: ']' at column 13 has no matching '['" and the preview keeps the last valid layout
    let (field_y, _) = find(&rows, "Layout  board, agent]").or_fail("the field")?;
    let (error_y, _) = find(
        &rows,
        "unbalanced bracket: ']' at column 13 has no matching '['",
    )
    .or_fail("the error")?;
    assert_eq!(error_y, field_y + 1, "the error sits right under the field");
    assert!(find(&rows, "│board").is_some() && find(&rows, "│agent").is_some());
    // @step When the user activates Apply
    press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    press(&mut app, key(KeyCode::Enter, KeyModifiers::NONE)).await?;
    // @step Then the dialog stays open with the error and the live mux layout is unchanged
    assert!(app.compositor().contains(DIALOG_ID));
    assert!(find(&render(&mut app, 120, 40)?, "unbalanced bracket").is_some());
    assert_eq!(layout_string(&app), "board, agent");
    // @step When the user deletes the "]" and activates Save
    press(&mut app, key(KeyCode::BackTab, KeyModifiers::SHIFT)).await?;
    press(&mut app, key(KeyCode::BackTab, KeyModifiers::SHIFT)).await?;
    press(&mut app, key(KeyCode::End, KeyModifiers::NONE)).await?;
    press(&mut app, key(KeyCode::Backspace, KeyModifiers::NONE)).await?;
    for _ in 0..3 {
        press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    }
    press(&mut app, key(KeyCode::Enter, KeyModifiers::NONE)).await?;
    // @step Then the dialog closes, mux uses "board, agent" and tui.mux holds the layout string
    assert!(!app.compositor().contains(DIALOG_ID));
    assert_eq!(layout_string(&app), "board, agent");
    assert_eq!(saved_mux(&data)?["layout"], "board, agent");
    Ok(())
}

/// Scenario: The config dialog's focus moves between the field, the Enabled toggle and the buttons
#[tokio::test(start_paused = true)]
async fn the_config_dialog_focus_moves_between_its_controls() -> TestResult {
    // @step Given the mux config dialog is open with the layout field "board, agent" focused and Enabled Off
    let (mut app, _mock) = app_with_session().await?;
    open_dialog(&mut app).await?;
    row_with(&render(&mut app, 120, 40)?, "Enabled  Off")?;
    // @step When the user presses Tab
    press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    // @step Then the Enabled toggle is focused and pressing Space turns it On
    press(&mut app, key(KeyCode::Char(' '), KeyModifiers::NONE)).await?;
    let rows = render(&mut app, 120, 40)?;
    row_with(&rows, "Enabled  On")?;
    row_with(&rows, "Layout  board, agent ")?;
    // @step When the user presses Tab twice and then Down
    press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    press(&mut app, key(KeyCode::Tab, KeyModifiers::NONE)).await?;
    press(&mut app, key(KeyCode::Down, KeyModifiers::NONE)).await?;
    // @step Then focus wraps from the Save button back to the layout field
    // @step And typing "s" in the field inserts it into the layout text instead of saving
    type_text(&mut app, "s").await?;
    assert!(
        app.compositor().contains(DIALOG_ID),
        "'s' does not save and close"
    );
    let rows = render(&mut app, 120, 40)?;
    row_with(&rows, "Layout  board, agents")?;
    // @step And the dialog footer lists Tab move, type to edit, Space toggle, Enter apply and Esc cancel
    row_with(
        &rows,
        "Tab/↑↓ Move · Type to edit · Space Toggle · Enter Apply · Esc Cancel",
    )?;
    Ok(())
}
