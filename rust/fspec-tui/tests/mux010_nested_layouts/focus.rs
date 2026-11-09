//! MUX-010 focus and drag scenarios: spatial Shift+Up/Down and
//! layout-order Shift+Left/Right (MUX-012) across nested panes, the Shift+Up/Down fall-through to the agent's history keys, and
//! divider drags rewriting weights.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_test_helpers::TestResult;
use crossterm::event::{KeyCode, MouseEventKind};

use super::support::{
    app_with_session, drain, layout_string, left, mouse, mux_with, pane_rects, press, render, shift,
};

/// Scenario: Shift+Up/Down move focus spatially and Shift+Left/Right in layout order across nested panes
#[tokio::test(start_paused = true)]
async fn shift_up_down_move_spatially_and_left_right_in_layout_order_across_nested_panes(
) -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "agent, [board, board], files" and the top board focused
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "agent, [board, board], files", 120, 40).await?;
    app.navigator_mut().mux.set_focus(1);
    let focus = |app: &codelet_fspec_tui::App| app.navigator().mux.focus();
    // @step When the user presses Shift+Down
    press(&mut app, shift(KeyCode::Down)).await?;
    // @step Then the bottom board is focused
    assert_eq!(focus(&app), 2);
    // @step When the user presses Shift+Right
    press(&mut app, shift(KeyCode::Right)).await?;
    // @step Then the files pane is focused
    assert_eq!(focus(&app), 3);
    assert_eq!(
        app.navigator().mux.effective_panes()[3],
        MuxPaneKind::ChangedFiles
    );
    // @step When the user presses Shift+Left
    press(&mut app, shift(KeyCode::Left)).await?;
    // @step Then the bottom board, the previous pane in layout order, is focused
    assert_eq!(focus(&app), 2);
    // @step When the user presses Shift+Up and then Shift+Up again
    press(&mut app, shift(KeyCode::Up)).await?;
    let after_first = focus(&app);
    press(&mut app, shift(KeyCode::Up)).await?;
    // @step Then the top board is focused and the second Shift+Up leaves it focused
    assert_eq!(after_first, 1);
    assert_eq!(focus(&app), 1);
    Ok(())
}

/// Scenario: Dragging a divider rewrites the weights of its two neighbours
#[tokio::test(start_paused = true)]
async fn dragging_a_divider_rewrites_the_weights_of_its_two_neighbours() -> TestResult {
    // @step Given the terminal is 120x24 and mux is on with the layout "agent, board"
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "agent, board", 120, 24).await?;
    let divider = app.navigator().mux.dividers()[0].rect;
    assert_eq!(divider.x, 59);
    // @step When the user drags the divider to 40 percent of the width and releases it
    press(&mut app, mouse(left(MouseEventKind::Down), divider.x, 5)).await?;
    press(&mut app, mouse(left(MouseEventKind::Drag), 48, 5)).await?;
    assert_eq!(
        layout_string(&app),
        "agent, board",
        "the stored layout waits for the release"
    );
    press(&mut app, mouse(left(MouseEventKind::Up), 48, 5)).await?;
    // @step Then the stored layout reads "agent:2, board:3"
    assert_eq!(layout_string(&app), "agent:2, board:3");
    // @step And the agent pane is 47 cells wide
    render(&mut app, 120, 24)?;
    assert_eq!(pane_rects(&app)[0].width, 47);
    Ok(())
}

/// Scenario: Shift+Up and Shift+Down stay the agent pane's history keys when no pane is above or below
#[tokio::test(start_paused = true)]
async fn shift_up_stays_the_agent_history_key_without_a_pane_above() -> TestResult {
    // @step Given mux is on with the layout "board, agent", one open session and the agent pane focused
    let (mut app, _mock) = app_with_session().await?;
    mux_with(&mut app, "board, agent", 120, 24).await?;
    app.navigator_mut().mux.set_focus(1);
    // @step When the user presses Shift+Up
    let _ = app.handle_event(&shift(KeyCode::Up));
    // @step Then the agent pane stays focused and the agent input receives Shift+Up as its history-recall key
    assert_eq!(app.navigator().mux.focus(), 1);
    let mut actions = Vec::new();
    while let Some(action) = app.try_recv_action() {
        actions.push(action);
    }
    assert!(
        actions.iter().any(|a| matches!(a, Action::HistoryPrev)),
        "Shift+Up must reach the agent input: {actions:?}"
    );
    for action in actions {
        app.dispatch(action);
    }
    drain(&mut app).await?;
    Ok(())
}
