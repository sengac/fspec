//! MUX-010 grid scenarios: nesting, alternation, weights, errors, the
//! removed syntax, the kept subcommands, the minimum pane size and
//! unfilled agent slots.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_test_helpers::TestResult;
use ratatui::layout::Rect;

use super::support::{
    app_with_session, layout_string, mux_with, pane_rects, render, scrollback, scrollback_has,
    submit,
};

/// Scenario: A bracket layout nests rows inside a column
#[tokio::test(start_paused = true)]
async fn a_bracket_layout_nests_rows_inside_a_column() -> TestResult {
    // @step Given the terminal is 80x24 and mux is off
    let (mut app, _mock) = app_with_session().await?;
    assert!(!app.navigator().mux.config().enabled);
    // @step When the user submits "/mux agent, [board, board], board, files"
    mux_with(&mut app, "agent, [board, board], board, files", 80, 24).await?;
    // @step Then mux is on with four columns 19, 19, 19 and 20 cells wide over the 21-row body below the two bar rows
    let rects = pane_rects(&app);
    let columns: Vec<(u16, u16)> = rects.iter().map(|r| (r.x, r.width)).collect();
    assert_eq!(
        columns,
        vec![(0, 19), (20, 19), (20, 19), (40, 19), (60, 20)]
    );
    assert_eq!((rects[0].y, rects[0].height), (2, 21));
    // @step And the second column holds two board panes stacked as rows 10 and 10 cells tall
    assert_eq!((rects[1].y, rects[1].height), (2, 10));
    assert_eq!((rects[2].y, rects[2].height), (13, 10));
    // @step And the panes in reading order are agent, board, board, board, files
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[
            MuxPaneKind::Agent,
            MuxPaneKind::Board,
            MuxPaneKind::Board,
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles
        ]
    );
    Ok(())
}

/// Scenario: Groups alternate direction at every nesting level
#[tokio::test(start_paused = true)]
async fn groups_alternate_direction_at_every_nesting_level() -> TestResult {
    // @step Given the terminal is 120x40
    let (mut app, _mock) = app_with_session().await?;
    // @step When the user submits "/mux [agent, [board, files]:2]"
    mux_with(&mut app, "[agent, [board, files]:2]", 120, 40).await?;
    // @step Then one column holds two rows, the bottom row twice as tall as the top row
    let rects = pane_rects(&app);
    assert_eq!(rects[0], Rect::new(0, 1, 120, 12));
    assert_eq!((rects[1].y, rects[1].height), (14, 25));
    // @step And the bottom row is split into a board column and a files column
    assert_eq!(rects[1], Rect::new(0, 14, 59, 25));
    assert_eq!(rects[2], Rect::new(60, 14, 60, 25));
    assert_eq!(
        &app.navigator().mux.effective_panes()[1..],
        &[MuxPaneKind::Board, MuxPaneKind::ChangedFiles]
    );
    Ok(())
}

/// Scenario: Weights size panes and groups within their group
#[tokio::test(start_paused = true)]
async fn weights_size_panes_and_groups_within_their_group() -> TestResult {
    // @step Given the terminal is 120x40
    let (mut app, _mock) = app_with_session().await?;
    // @step When the user submits "/mux agent:2, [board, board], files"
    mux_with(&mut app, "agent:2, [board, board], files", 120, 40).await?;
    // @step Then the columns are 59, 29 and 30 cells wide
    let widths: Vec<u16> = pane_rects(&app).iter().map(|r| r.width).collect();
    assert_eq!(widths, vec![59, 29, 29, 30]);
    // @step And the saved layout string reads "agent:2, [board, board], files"
    assert_eq!(layout_string(&app), "agent:2, [board, board], files");
    Ok(())
}

/// Scenario: Layout parse errors are exact, carry the column and leave the live layout untouched
#[tokio::test(start_paused = true)]
async fn layout_parse_errors_are_exact_and_leave_the_layout_untouched() -> TestResult {
    // @step Given mux is on with the layout "board, agent"
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "board, agent", 120, 24).await?;
    // @step When the user submits each of "/mux agent, [board, board", "/mux agent, [], files", "/mux agent, bord" and "/mux agent:0, board"
    for line in [
        "/mux agent, [board, board",
        "/mux agent, [], files",
        "/mux agent, bord",
        "/mux agent:0, board",
    ] {
        submit(&mut app, line).await?;
    }
    // @step Then the errors read "/mux: unbalanced bracket: '[' at column 8 is never closed", "/mux: empty group at column 8", "/mux: unknown pane 'bord' at column 8; panes are board, agent, files, checkpoints" and "/mux: bad weight '0' at column 7; weights are positive integers"
    let text = scrollback(&app);
    for error in [
        "/mux: unbalanced bracket: '[' at column 8 is never closed",
        "/mux: empty group at column 8",
        "/mux: unknown pane 'bord' at column 8; panes are board, agent, files, checkpoints",
        "/mux: bad weight '0' at column 7; weights are positive integers",
    ] {
        assert!(scrollback_has(&app, error), "missing {error:?} in {text}");
    }
    // @step And the live layout is still "board, agent"
    assert_eq!(layout_string(&app), "board, agent");
    Ok(())
}

/// Scenario: The removed mux syntax fails with the new syntax in the error
#[tokio::test(start_paused = true)]
async fn the_removed_mux_syntax_fails_with_the_new_syntax_in_the_error() -> TestResult {
    // @step Given mux is on with the layout "board, agent"
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "board, agent", 120, 24).await?;
    // @step When the user submits each of "/mux v", "/mux horizontal", "/mux 3" and "/mux board agent 40"
    let removed = ["v", "horizontal", "3", "board agent 40"];
    for args in removed {
        submit(&mut app, &format!("/mux {args}")).await?;
    }
    // @step Then each error reads "/mux: '<args>' is the removed mux syntax; write a layout string, e.g. /mux agent, [board, board], board, files" with its own arguments
    let text = scrollback(&app);
    for args in removed {
        let error = format!(
            "/mux: '{args}' is the removed mux syntax; write a layout string, \
             e.g. /mux agent, [board, board], board, files"
        );
        assert!(scrollback_has(&app, &error), "missing {error:?} in {text}");
    }
    // @step And the live layout is still "board, agent"
    assert_eq!(layout_string(&app), "board, agent");
    Ok(())
}

/// Scenario: The kept subcommands still work
#[tokio::test(start_paused = true)]
async fn the_kept_subcommands_still_work() -> TestResult {
    let (_guard, data) = super::config::root_data_dir().await?;
    // @step Given mux is off
    let (mut app, _mock) = app_with_session().await?;
    render(&mut app, 120, 24)?;
    app.navigator_mut().mux.config_mut().layout =
        codelet_fspec_tui::MuxLayout::parse("[board, files]")?;
    // @step When the user submits "/mux on", "/mux save", "/mux default", "/mux off" and "/mux help" in turn
    submit(&mut app, "/mux on").await?;
    // @step Then mux turns on with the saved layout, saves it, resets to "board, agent" and turns off
    assert_eq!(app.navigator().active_view, ViewMode::Mux);
    assert_eq!(layout_string(&app), "[board, files]");
    submit(&mut app, "/mux save").await?;
    assert_eq!(super::config::saved_mux(&data)?["layout"], "[board, files]");
    submit(&mut app, "/mux default").await?;
    assert_eq!(layout_string(&app), "board, agent");
    submit(&mut app, "/mux off").await?;
    assert_ne!(app.navigator().active_view, ViewMode::Mux);
    submit(&mut app, "/mux help").await?;
    // @step And the help output lists the layout grammar, the weight suffix, the subcommands on, off, save, default and help, and Shift+arrow focus
    let text = scrollback(&app);
    for part in [
        "/mux agent, [board, board], board, files",
        "each [..] splits the other way",
        "pane:N or [..]:N",
        "/mux on|off|save|default|help",
        "Shift+Left/Right move to the previous/next pane in layout order",
        "Shift+Up/Down to the pane above/below",
    ] {
        assert!(
            scrollback_has(&app, part),
            "the help lacks {part:?}: {text}"
        );
    }
    Ok(())
}

/// Scenario: A layout whose panes would be smaller than the minimum pane size is refused
#[tokio::test(start_paused = true)]
async fn a_layout_below_the_minimum_pane_size_is_refused() -> TestResult {
    // @step Given the terminal is 80x24 and mux is on with the layout "board, agent"
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "board, agent", 80, 24).await?;
    // @step When the user submits "/mux board, agent, files, checkpoints, board"
    submit(&mut app, "/mux board, agent, files, checkpoints, board").await?;
    // @step Then the error reads "/mux: layout does not fit 80x21: pane 1 (board) would be 15x21; every pane needs at least 16x5"
    let error = "/mux: layout does not fit 80x21: pane 1 (board) would be 15x21; \
                 every pane needs at least 16x5";
    assert!(scrollback_has(&app, error), "{}", scrollback(&app));
    // @step And the live layout is still "board, agent"
    assert_eq!(layout_string(&app), "board, agent");
    // @step But at 120x40 the same layout is applied with five columns
    mux_with(&mut app, "board, agent, files, checkpoints, board", 120, 40).await?;
    assert_eq!(pane_rects(&app).len(), 5);
    Ok(())
}

/// Scenario: Unfilled agent slots drop out of a nested layout without touching the stored layout
#[tokio::test(start_paused = true)]
async fn unfilled_agent_slots_drop_out_of_a_nested_layout() -> TestResult {
    // @step Given the terminal is 120x40 and one agent session is open
    let (mut app, _mock) = app_with_session().await?;
    // @step When the user submits "/mux [agent, agent], board"
    mux_with(&mut app, "[agent, agent], board", 120, 40).await?;
    // @step Then the first column shows a single agent pane over its full height
    let rects = pane_rects(&app);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], Rect::new(0, 1, 59, 38));
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Agent, MuxPaneKind::Board]
    );
    // @step And the stored layout is still "[agent, agent], board"
    assert_eq!(layout_string(&app), "[agent, agent], board");
    Ok(())
}
