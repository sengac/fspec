//! MENU-007 — the agent view's 2-zone bar shows a single 'Board View'
//! item instead of the board-only Actions/Help menus.
//!
//! Feature: spec/features/agent-view-menu-bar-board-view-item.feature
//!
//! Harness (App + MockBackend, TestBackend render): `menu007_support/mod.rs`.

use codelet_fspec_tui::ViewMode;
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::KeyCode;
use ratatui::{backend::TestBackend, style::Color, Terminal};

mod common;
mod menu007_support;
use menu007_support::*;

/// Scenario: The agent bar paints a single 'Board View' Zone A item instead of Actions and Help
#[tokio::test]
async fn scenario_the_agent_bar_paints_a_single_board_view_item_instead_of_actions_and_help(
) -> TestResult {
    // @step Given the agent pane has 2 open sessions (one running, one idle) and an empty input
    let mut app = agent_view_with_sessions(2).await?;
    app.agent_view_store_mut().set_session_status(
        SessionId::new("s-1".to_string()),
        codelet_rpc_types::SessionStatus::Running,
    );
    drain_pending(&mut app).await?;
    let buf = render_app(&mut app)?;

    // @step When the agent pane renders
    let row = row_text(&buf, AGENT_BAR_ROW);

    // @step Then the bar row shows the 'Board View' item and the dim separator and the two session chips
    assert!(
        row.contains("Board View"),
        "the bar row must show 'Board View': {row:?}"
    );
    assert!(
        row.contains('│'),
        "the bar row must show the separator: {row:?}"
    );
    assert!(row.contains("#1"), "the bar row must show chip #1: {row:?}");
    assert!(row.contains("#2"), "the bar row must show chip #2: {row:?}");

    // @step And the bar row does not show the 'Kanban' item (MENU-008: the board-only 'Actions' item was renamed to 'Kanban')
    assert!(
        !row.contains("Kanban"),
        "the agent bar must NOT paint the board-only 'Kanban' item: {row:?}"
    );

    // @step And the bar row does not show the 'Help' item
    assert!(
        !row.contains("Help"),
        "the agent bar must NOT paint the board-only 'Help' item: {row:?}"
    );
    Ok(())
}

/// Scenario: The 'Board View' item paints the selected-item highlight when the ring focuses it
#[tokio::test]
async fn scenario_the_board_view_item_paints_the_selected_item_highlight_when_the_ring_focuses_it(
) -> TestResult {
    // @step Given the agent pane has 1 open session and an empty input
    let mut app = agent_view_with_sessions(1).await?;
    render_app(&mut app)?; // cache the bar geometry (chip count)

    // @step When I press bare Left once
    press(&mut app, KeyCode::Left).await?;
    let buf = render_app(&mut app)?;

    // @step Then the 'Board View' item paints inverse-video (bg Cyan fg Black bold) with no dropdown open
    let x = board_view_x(&buf)?;
    let cell = &buf[(x, AGENT_BAR_ROW)];
    assert_eq!(
        cell.bg,
        Color::Cyan,
        "'Board View' must paint inverse-video bg"
    );
    assert_eq!(
        cell.fg,
        Color::Black,
        "'Board View' must paint inverse-video fg"
    );
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::BOLD),
        "'Board View' must paint bold"
    );
    assert!(
        app.navigator().agent.open_menu().is_none(),
        "no dropdown may open for the 'Board View' item"
    );
    Ok(())
}

/// Scenario: Enter on 'Board View' returns to the Board view
#[tokio::test]
async fn scenario_enter_on_board_view_returns_to_the_board_view() -> TestResult {
    // @step Given the App is in the Agent view with 2 open sessions and the bar is focused on the 'Board View' item
    let mut app = agent_view_with_sessions(2).await?;
    render_app(&mut app)?; // cache the bar geometry (chip count)
    press(&mut app, KeyCode::Left).await?; // bare Left → Item(0) = 'Board View'

    // @step When I press Enter once
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the active view is the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Enter on 'Board View' must return to the Board view"
    );

    // @step And the agent bar highlight clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "the bar highlight must clear after activating 'Board View'"
    );
    Ok(())
}

/// Scenario: Clicking 'Board View' returns to the Board view
#[tokio::test]
async fn scenario_clicking_board_view_returns_to_the_board_view() -> TestResult {
    // @step Given the App is in the Agent view with 2 open sessions
    let mut app = agent_view_with_sessions(2).await?;
    let buf = render_app(&mut app)?;
    let x = board_view_x(&buf)?;

    // @step When I click the 'Board View' item in the bar
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await?;

    // @step Then the active view is the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "clicking 'Board View' must return to the Board view"
    );

    // @step And no dropdown panel paints over the pane
    render_app(&mut app)?;
    assert!(
        app.navigator().agent.open_menu().is_none(),
        "clicking 'Board View' must not open a dropdown"
    );
    Ok(())
}

/// Scenario: The agent ring wraps from 'Board View' through the chips and back
#[tokio::test]
async fn scenario_the_agent_ring_wraps_from_board_view_through_the_chips_and_back() -> TestResult {
    // @step Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    let mut app = agent_view_with_sessions(2).await?;
    render_app(&mut app)?; // cache the bar geometry (chip count)
    press(&mut app, KeyCode::Left).await?; // Item(0) = 'Board View'

    // @step When I press Right six times
    // (MENU-009 + TUI-114: 2 chips + 3 Zone C buttons = 5 stops after the item.)
    let stops = [
        (MenuFocus::ZoneB(0), "chip #1 (no 2nd Zone A item)"),
        (MenuFocus::ZoneB(1), "chip #2"),
        (MenuFocus::ZoneC(0), "'New Bot'"),
        (MenuFocus::ZoneC(1), "'New Agent'"),
        (MenuFocus::ZoneC(2), "'Close'"),
    ];
    for (index, (focus, label)) in stops.into_iter().enumerate() {
        press(&mut app, KeyCode::Right).await?;
        assert_focus(
            &app,
            focus,
            &format!("Right #{} must land on {label}", index + 1),
        );
    }
    press(&mut app, KeyCode::Right).await?;

    // @step Then the focus lands on chip #1 then chip #2 then 'New Bot' then 'New Agent' then 'Close' and finally back on 'Board View'
    assert_focus(
        &app,
        MenuFocus::Item(0),
        "the 6th Right must wrap to 'Board View'",
    );
    Ok(())
}

/// Scenario: Left from 'Board View' wraps to the last Zone C button
#[tokio::test]
async fn scenario_left_from_board_view_wraps_to_the_last_chip() -> TestResult {
    // @step Given the agent pane has 3 open sessions and the menu bar is focused on the 'Board View' item
    let mut app = agent_view_with_sessions(3).await?;
    render_app(&mut app)?; // cache the bar geometry (chip count + Zone C rects)
    press(&mut app, KeyCode::Left).await?; // Item(0) = 'Board View'

    // @step When I press Left once
    press(&mut app, KeyCode::Left).await?;
    let buf = render_app(&mut app)?;

    // @step Then the 'Close' button paints inverse-video (the ring's last stop — MENU-009)
    assert_focus(
        &app,
        MenuFocus::ZoneC(2),
        "Left wraps to the last Zone C button",
    );
    let x = zone_c_button_x(&buf, "Close")?;
    assert_eq!(
        buf[(x, AGENT_BAR_ROW)].bg,
        Color::Cyan,
        "'Close' must paint inverse-video"
    );
    Ok(())
}

/// Scenario: Chip activation in the agent view is unchanged
#[tokio::test]
async fn scenario_chip_activation_in_the_agent_view_is_unchanged() -> TestResult {
    // @step Given the App is in the Agent view with 3 open sessions
    let mut app = agent_view_with_sessions(3).await?;
    let buf = render_app(&mut app)?;

    // @step When I click chip #2 in the menu bar
    let x = chip_x(&buf, 2)?;
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await?;

    // @step Then the agent pane shows session s-2
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2".to_string())),
        "clicking chip #2 must switch to s-2"
    );

    // @step And the active view is still the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "chip activation must stay in the Agent view"
    );
    Ok(())
}

/// Scenario: The agent bar with no open sessions shows only 'Board View'
#[tokio::test]
async fn scenario_the_agent_bar_with_no_open_sessions_shows_only_board_view() -> TestResult {
    // @step Given the agent pane has no open sessions
    // (view-level harness — the App cannot enter the Agent view with an
    // empty open-session list; it opens the CreateSessionDialog instead)
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = codelet_fspec_tui::AgentView::new(tx);
    let mut store = codelet_fspec_tui::AgentViewStore::default();

    // @step When the agent pane renders
    let mut term = Terminal::new(TestBackend::new(100, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| view.render_with_store(frame.area(), frame.buffer_mut(), &mut store))
        .or_fail("draw")?;
    let buf = term.backend().buffer().clone();

    // @step Then the bar row shows only the 'Board View' item with no separator and no chips
    let row = row_text(&buf, AGENT_BAR_ROW);
    assert!(
        row.contains("Board View"),
        "the bar must still show 'Board View': {row:?}"
    );
    assert!(
        !row.contains('│'),
        "no separator may paint without chips: {row:?}"
    );
    assert!(
        !row.contains("#1"),
        "no chips may paint without sessions: {row:?}"
    );
    Ok(())
}

/// Scenario: Board and mux bars keep their Actions and Help items
#[tokio::test]
async fn scenario_board_and_mux_bars_keep_their_actions_and_help_items() -> TestResult {
    // @step Given the App is in the Board view
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I render the board header row
    let buf = render_app(&mut app)?;
    let bar_row = (0..buf.area.height)
        .find(|&y| {
            let row = row_text(&buf, y);
            row.contains("Kanban") && row.contains("Help")
        })
        .or_fail("the board bar row must render")?;

    // @step Then the board menu bar still shows the 'Kanban' item and the 'Help' item (MENU-008)
    assert!(
        row_text(&buf, bar_row).contains("Kanban"),
        "the board bar must keep the 'Kanban' item"
    );
    assert!(
        row_text(&buf, bar_row).contains("Help"),
        "the board bar must keep the 'Help' item"
    );
    Ok(())
}
