//! TUI-123 — the daemon switcher is as reachable as the workspace dropdown:
//! clickable header label, Settings menu entries, help rows, clickable rows,
//! a fresh read of daemons.json on every open and an in-panel error for a
//! corrupt file.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

use codelet_test_helpers::{OrFail, TestResult};
use std::collections::HashMap;
use std::sync::Arc;

use codelet_common::saved_daemons;
use codelet_fspec_tui::Action;
use crossterm::event::KeyCode;

mod common;
mod tui123_support;
use common::MockBackend;
use tui123_support::*;

const KEY_HINT: &str = "Enter connect · a add · e edit · d remove · Esc close";

#[tokio::test(flavor = "current_thread")]
async fn clicking_the_header_daemon_label_opens_the_switcher() -> TestResult {
    // @step Given the board is connected to local daemon "alpha"
    let mut fx = fixture(Setup::default()).await?;
    let buf = render(&mut fx.app, 120, 24);
    let y = header_row(&buf).or_fail("header row")?;
    let x = find_column(&buf, y, "Daemon: alpha ▾").or_fail("daemon label")?;

    // @step When the user clicks the header label "Daemon: alpha ▾"
    click(&mut fx.app, x + 3, y).await?;

    // @step Then the daemon switcher opens
    assert!(fx.app.board_store().daemon_switcher().is_some());
    assert!(fx.app.board_store().workspace_dropdown().is_none());

    // @step And pressing "s" on the board opens it as well
    press(&mut fx.app, KeyCode::Esc).await?;
    assert!(fx.app.board_store().daemon_switcher().is_none());
    press(&mut fx.app, KeyCode::Char('s')).await?;
    assert!(fx.app.board_store().daemon_switcher().is_some());
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_settings_menu_opens_the_daemon_switcher_and_the_workspace_dropdown() -> TestResult {
    // @step Given the board with its menu bar
    let mut fx = fixture(Setup::default()).await?;

    // @step When the user opens the Settings menu
    fx.app.dispatch(Action::MenuOpenDropdown(2));
    settle(&mut fx.app).await?;
    let screen = text(&render(&mut fx.app, 120, 30));

    // @step Then it lists "M Mux", "P Providers", "s Daemons" and "w Workspaces"
    let order: Vec<usize> = ["Mux", "Providers", "Daemons", "Workspaces"]
        .iter()
        .map(|label| screen.find(label).or_fail(label))
        .collect::<Result<_, _>>()?;
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{screen}");

    // @step And executing "s Daemons" opens the daemon switcher
    fx.app.dispatch(Action::MenuExecuteItem {
        category: 2,
        row: 2,
    });
    settle(&mut fx.app).await?;
    assert!(fx.app.board_store().daemon_switcher().is_some());
    press(&mut fx.app, KeyCode::Esc).await?;

    // @step And executing "w Workspaces" opens the workspace dropdown
    fx.app.dispatch(Action::MenuOpenDropdown(2));
    fx.app.dispatch(Action::MenuExecuteItem {
        category: 2,
        row: 3,
    });
    settle(&mut fx.app).await?;
    assert!(fx.app.board_store().workspace_dropdown().is_some());
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_board_help_lists_the_daemon_and_workspace_keys() -> TestResult {
    // @step Given the board
    let mut fx = fixture(Setup::default()).await?;

    // @step When the user opens the board help with "?"
    fx.app.dispatch(Action::OpenBoardHelp);
    settle(&mut fx.app).await?;
    let screen = text(&render(&mut fx.app, 120, 50));

    // @step Then it lists "s" for daemons and "w" for workspaces
    assert!(screen.contains("s             Daemons"), "{screen}");
    assert!(screen.contains("w             Workspaces"), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_open_switcher_marks_the_connected_daemon_and_shows_its_keys() -> TestResult {
    // @step Given local daemon "alpha" is connected and daemon "work" is saved
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some("t"))],
        ..Setup::default()
    })
    .await?;

    // @step When the daemon switcher is open
    press(&mut fx.app, KeyCode::Char('s')).await?;
    let screen = text(&render(&mut fx.app, 120, 24));

    // @step Then it shows "alpha (connected)" and "work (saved)"
    assert!(screen.contains("alpha (connected)"), "{screen}");
    assert!(screen.contains("work (saved)"), "{screen}");

    // @step And its border reads "Enter connect · a add · e edit · d remove · Esc close"
    assert!(screen.contains(KEY_HINT), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn switcher_rows_are_clickable_and_a_click_outside_closes_it() -> TestResult {
    // @step Given the daemon switcher is open listing "alpha" and "work"
    let work = Arc::new(MockBackend::new());
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some("t"))],
        outcomes: HashMap::from([("work".to_string(), Outcome::Backend(work))]),
        ..Setup::default()
    })
    .await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;
    let buf = render(&mut fx.app, 120, 24);
    let y = find_row(&buf, "work (saved)").or_fail("work row")?;
    let x = find_column(&buf, y, "work (saved)").or_fail("work column")?;

    // @step When the user clicks the "work" row
    click(&mut fx.app, x, y).await?;

    // @step Then the board switches to "work"
    assert_eq!(fx.app.board_store().connected_daemon(), Some("work"));
    assert!(fx.app.board_store().daemon_switcher().is_none());

    // @step And a click outside the open switcher closes it
    press(&mut fx.app, KeyCode::Char('s')).await?;
    assert!(fx.app.board_store().daemon_switcher().is_some());
    render(&mut fx.app, 120, 24);
    click(&mut fx.app, 110, 20).await?;
    assert!(fx.app.board_store().daemon_switcher().is_none());
    assert_eq!(fx.app.board_store().connected_daemon(), Some("work"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_switcher_rereads_daemons_json_every_time_it_opens() -> TestResult {
    // @step Given the TUI is running with no saved daemons
    let mut fx = fixture(Setup::default()).await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;
    assert!(!fx.app.board_store().daemons().iter().any(|d| d.saved));
    press(&mut fx.app, KeyCode::Esc).await?;

    // @step When "work" is saved by another process and the switcher is opened
    saved_daemons::save(&fx.path, saved("work", "ws://127.0.0.1:4002", None))?;
    press(&mut fx.app, KeyCode::Char('s')).await?;

    // @step Then the switcher lists "work"
    let screen = text(&render(&mut fx.app, 120, 24));
    assert!(screen.contains("work (saved)"), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_corrupt_daemons_json_is_an_error_in_the_switcher_and_is_never_overwritten() -> TestResult
{
    // @step Given daemons.json contains "{not json"
    let mut fx = fixture(Setup {
        raw_file: Some("{not json".into()),
        ..Setup::default()
    })
    .await?;

    // @step When the daemon switcher is opened
    press(&mut fx.app, KeyCode::Char('s')).await?;

    // @step Then the switcher shows that daemons.json could not be read, with its path, and no daemon rows
    assert!(fx.app.board_store().daemon_switcher().is_some());
    let screen = text(&render(&mut fx.app, 240, 30));
    assert!(screen.contains("could not be read"), "{screen}");
    assert!(screen.contains(&fx.path.display().to_string()), "{screen}");
    assert!(!screen.contains("alpha (connected)"), "{screen}");
    assert!(fx.app.board_store().daemons().is_empty());

    // @step And saving a new daemon is refused
    press(&mut fx.app, KeyCode::Char('a')).await?;
    type_text(&mut fx.app, "work").await?;
    press(&mut fx.app, KeyCode::Tab).await?;
    type_text(&mut fx.app, "ws://127.0.0.1:4002").await?;
    press(&mut fx.app, KeyCode::Enter).await?;
    assert!(fx.app.compositor().contains("daemon-form-dialog"));
    let screen = text(&render(&mut fx.app, 240, 30));
    assert!(screen.contains("not valid saved-daemons JSON"), "{screen}");
    assert!(screen.contains("not changed"), "{screen}");

    // @step And daemons.json still contains "{not json"
    assert_eq!(std::fs::read_to_string(&fx.path)?, "{not json");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn switching_to_a_daemon_that_rejects_the_token_keeps_the_current_connection() -> TestResult {
    // @step Given the board is connected to "alpha" and saved daemon "work" rejects the token
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some("wrong"))],
        outcomes: HashMap::from([("work".to_string(), Outcome::TokenRejected)]),
        ..Setup::default()
    })
    .await?;

    // @step When the user switches to "work"
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Down).await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then an error says daemon "work" rejected the token
    let screen = text(&render(&mut fx.app, 240, 40));
    assert!(
        screen.contains("Could not connect to daemon work"),
        "{screen}"
    );
    assert!(screen.contains("rejected the token"), "{screen}");

    // @step And the header still reads "Daemon: alpha ▾"
    assert_eq!(fx.app.board_store().connected_daemon(), Some("alpha"));
    press(&mut fx.app, KeyCode::Esc).await?;
    let screen = text(&render(&mut fx.app, 240, 40));
    assert!(screen.contains("Daemon: alpha ▾"), "{screen}");
    Ok(())
}
