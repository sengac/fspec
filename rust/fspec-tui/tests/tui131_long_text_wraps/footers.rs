//! TUI-131 — full-screen view footers and the board footer wrap.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::Action;
use codelet_rpc_types::{BlocklistRuleInfo, ChangedFile, CheckpointInfo};
use codelet_test_helpers::TestResult;

use super::fixture::*;
use super::screen::{Checks, Screen};
use super::status_rows::{board_fixture, model_selector};
use crate::common::MockBackend;

/// A footer is on screen whole when each of its hints is.
fn footer_whole(screen: &Screen, footer: &str, separator: &str) -> bool {
    footer.split(separator).all(|hint| screen.shown_whole(hint))
}

const BLOCKLIST_FOOTER: &str = "↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | c/C: Copy Pattern/Rule | Esc: Close";
const CHANGED_FILES_FOOTER: &str = "ESC: Back | Tab: Switch Panes | ↑↓: Navigate/Scroll | PgUp/PgDn: Scroll | c: Copy Path | C: Copy Diff";
const CHECKPOINTS_FOOTER: &str = "ESC: Back | Tab/→: Next Pane | ←: Prev Pane | ↑↓: Navigate/Scroll | r: Restore File | t: Restore All | d: Delete | a: Delete All | c: Copy Name";
const MODEL_SELECTOR_FOOTER: &str =
    "Enter Select | ←→ Expand/Collapse | / Filter | c Copy | r Refresh | Tab Providers | a/e/d Custom model | Esc Close";

async fn blocklist_view() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![BlocklistRuleInfo {
        id: "cat-block".into(),
        pattern: "^cat\\s+".into(),
        action: "block".into(),
        reason: "cat is blocked".into(),
        guidance: None,
        source: "project".into(),
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    settle(&mut fx.app).await?;
    let session = fx.app.current_session();
    fx.app.dispatch(Action::SlashCommandSelected(
        session,
        SlashCommandAction::Blocklist,
    ));
    settle(&mut fx.app).await?;
    Ok(fx)
}

async fn changed_files_view() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mock = Arc::new(MockBackend::new());
    mock.set_changed_files(vec![ChangedFile {
        path: "src/main.rs".into(),
        change_type: "M".into(),
        staged: false,
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenChangedFilesView);
    settle(&mut fx.app).await?;
    fx.app.dispatch(Action::FileDiffLoaded {
        path: "src/main.rs".into(),
        diff: Some("@@ -1 +1 @@\n-old\n+new".into()),
    });
    settle(&mut fx.app).await?;
    Ok(fx)
}

async fn checkpoints_view() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoints(vec![CheckpointInfo {
        work_unit_id: "TUI-131".into(),
        name: "before-wrap".into(),
        timestamp: "2026-11-07T00:00:00.000Z".into(),
        is_automatic: false,
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenCheckpointsView);
    settle(&mut fx.app).await?;
    Ok(fx)
}

/// Scenario: Full-screen view footers wrap and the lists keep their rows
#[tokio::test(start_paused = true)]
async fn full_screen_view_footers_wrap_and_the_lists_keep_their_rows() -> TestResult {
    // @step Given the blocklist, changed files, checkpoints and model selector views each have rows to show
    let mut blocklist = blocklist_view().await?;
    let mut changed = changed_files_view().await?;
    let mut checkpoints = checkpoints_view().await?;
    let mut models = model_selector(Arc::new(MockBackend::new())).await?;
    assert_eq!(blocklist.app.navigator().active_view, ViewMode::Blocklist);
    assert_eq!(changed.app.navigator().active_view, ViewMode::ChangedFiles);
    assert_eq!(
        checkpoints.app.navigator().active_view,
        ViewMode::Checkpoints
    );
    assert_eq!(models.app.navigator().active_view, ViewMode::ModelSelector);
    let mut checks = Checks::default();

    // @step When each view is drawn at 80x24
    let blocklist_screen = Screen::of(&mut blocklist.app, 80, 24);
    let changed_screen = Screen::of(&mut changed.app, 80, 24);
    let checkpoints_screen = Screen::of(&mut checkpoints.app, 80, 24);
    let models_screen = Screen::of(&mut models.app, 80, 24);

    // @step Then the blocklist footer is on screen whole, including "Esc: Close"
    checks.check(
        footer_whole(&blocklist_screen, BLOCKLIST_FOOTER, " | "),
        "80x24: the blocklist footer is not shown whole",
        &blocklist_screen,
    );
    // @step And the changed files footer is on screen whole, including "C: Copy Diff"
    checks.check(
        footer_whole(&changed_screen, CHANGED_FILES_FOOTER, " | "),
        "80x24: the changed files footer is not shown whole",
        &changed_screen,
    );
    // @step And the checkpoints footer is on screen whole, including "c: Copy Name"
    checks.check(
        footer_whole(&checkpoints_screen, CHECKPOINTS_FOOTER, " | "),
        "80x24: the checkpoints footer is not shown whole",
        &checkpoints_screen,
    );
    // @step And the model selector footer is on screen whole, including "Esc Close"
    checks.check(
        footer_whole(&models_screen, MODEL_SELECTOR_FOOTER, " | "),
        "80x24: the model selector footer is not shown whole",
        &models_screen,
    );
    // @step And each view still shows its first row
    for (screen, first) in [
        (&blocklist_screen, "cat-block"),
        (&changed_screen, "src/main.rs"),
        (&checkpoints_screen, "before-wrap"),
        (&models_screen, "openai: my-profile"),
    ] {
        checks.check(
            screen.find(first).is_some(),
            &format!("80x24: the first row {first:?} is not on screen"),
            screen,
        );
    }
    // @step And at 120x40 the checkpoints footer is on screen whole
    let screen = Screen::of(&mut checkpoints.app, 120, 40);
    checks.check(
        footer_whole(&screen, CHECKPOINTS_FOOTER, " | "),
        "120x40: the checkpoints footer is not shown whole",
        &screen,
    );
    checks.done()
}

/// The board footer: the hint naming the card chooser, or the compact one.
const BOARD_FOOTERS: [&str; 2] = [
    "← → Cycle Columns → menu items → chips ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Agent / Details (choose) ◆ ESC Exit",
    "← → Cycle Columns → menu items → chips ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Choose ◆ ESC Exit",
];

fn board_footer_whole(screen: &Screen) -> bool {
    BOARD_FOOTERS
        .iter()
        .any(|footer| footer_whole(screen, footer, " ◆ "))
}

/// Scenario: The board footer is shown whole at 80 and at 120 columns
#[tokio::test(start_paused = true)]
async fn the_board_footer_is_shown_whole_at_80_and_at_120_columns() -> TestResult {
    // @step Given the board is open
    let mut fx = board_fixture(Arc::new(MockBackend::new()));
    let mut checks = Checks::default();

    // @step When the board is drawn at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then the board footer is on screen whole, including "ESC Exit"
    checks.check(
        board_footer_whole(&screen),
        "80x24: the board footer is not shown whole",
        &screen,
    );
    // @step And at 120x40 the board footer is on screen whole
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        board_footer_whole(&screen),
        "120x40: the board footer is not shown whole",
        &screen,
    );
    checks.done()
}
