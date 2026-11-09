//! TUI-131 — status and error rows: the model selector write error, the
//! board card details footer and attachment error, the Add workspace path
//! and error, and the Provider Settings profile-form status.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature

use std::sync::Arc;

use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
use codelet_fspec_tui::views::ProviderSettingsMode;
use codelet_fspec_tui::Action;
use codelet_rpc_types::{CustomModelDefinition, ModelEntry, ProfileDefinition, WorkUnitInfo};
use codelet_sessions::profile_sections::build_profile_provider_info;
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::fixture::*;
use super::screen::{dialog_shows, Checks, Screen};
use crate::common::{MockBackend, TaskOutcome};

/// The model selector over one OpenAI profile section with one model.
pub async fn model_selector(mock: Arc<MockBackend>) -> Result<Fixture, Box<dyn std::error::Error>> {
    let models = vec![ModelEntry {
        id: "gpt-5-codex".into(),
        display_name: "GPT-5 Codex".into(),
        context_window: Some(400_000),
        ..ModelEntry::default()
    }];
    mock.seed_providers(vec![build_profile_provider_info(
        "openai",
        "my-profile",
        models,
        false,
    )]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenModelSelectorView);
    settle(&mut fx.app).await?;
    Ok(fx)
}

/// Scenario: A long model selector write error is shown whole
#[tokio::test(start_paused = true)]
async fn a_long_model_selector_write_error_is_shown_whole() -> TestResult {
    // @step Given the model selector is open
    let mut fx = model_selector(Arc::new(MockBackend::new())).await?;
    let mut checks = Checks::default();

    // @step When saving a custom model fails with a 200-character error
    let error = prose("profile", 200, "not-writable");
    fx.mock.fail_custom_model_writes(&error);
    fx.app.dispatch(Action::AddCustomModel {
        provider_id: "openai".into(),
        profile_name: "my-profile".into(),
        definition: CustomModelDefinition {
            id: "gpt-x".into(),
            ..CustomModelDefinition::default()
        },
    });
    settle(&mut fx.app).await?;

    // @step Then the error is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        screen.shown_whole(&error),
        "80x24: the write error is not shown whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        screen.shown_whole(&error),
        "120x40: the write error is not shown whole",
        &screen,
    );
    checks.done()
}

/// The board with card AUTH-001 selected in backlog, in workspace /work/alpha.
pub fn board_fixture(mock: Arc<MockBackend>) -> Fixture {
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::WorkUnitsLoaded(vec![WorkUnitInfo {
        id: "AUTH-001".into(),
        title: "Sign in".into(),
        status: "backlog".into(),
        work_type: "story".into(),
        description: Some("short".into()),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }]));
    fx.app
        .board_store_mut()
        .set_workspaces(vec![codelet_rpc_types::WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        }]);
    fx.app.board_store_mut().set_focused_column("backlog");
    fx.app
        .board_store_mut()
        .set_selected_index_for("backlog", 0);
    fx
}

/// Open the full details of the selected card (Enter, Down, Enter) and
/// answer them with a document listing `attachment`.
async fn open_card_details(fx: &mut Fixture, attachment: &str) -> TestResult {
    let _ = Screen::of(&mut fx.app, 120, 36);
    key(fx, KeyCode::Enter).await?;
    // The chooser's Down and Enter go in before its details request is
    // answered, so the request can be answered with this document.
    for code in [KeyCode::Down, KeyCode::Enter] {
        let _ = fx
            .app
            .handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }
    let mut generation = None;
    for _ in 0..16 {
        while let Some(handle) = fx.app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = fx.app.try_recv_action() {
            match action {
                Action::BoardCardDetailsLoaded {
                    request_generation, ..
                } => generation = Some(request_generation),
                other => fx.app.dispatch(other),
            }
        }
        if generation.is_some() {
            break;
        }
    }
    let request_generation = generation.or_fail("the card details were requested")?;
    let document = serde_json::json!({
        "id": "AUTH-001",
        "workspace": "/work/alpha",
        "workUnit": {"id": "AUTH-001", "title": "Sign in", "type": "story",
                     "status": "backlog", "description": "short",
                     "attachments": [attachment]},
    });
    fx.app.dispatch(Action::BoardCardDetailsLoaded {
        request_generation,
        work_unit_id: "AUTH-001".into(),
        result: Ok(document.to_string()),
    });
    settle(&mut fx.app).await
}

/// The board card details footer, by its `│`-separated hints.
const CARD_FOOTER: &[&str] = &[
    "Tab/Shift-Tab select link or attachment",
    "Enter open",
    "Esc back",
    "↑↓ PgUp PgDn Home End wheel scroll",
    "c copy",
    "y copy id",
];

fn footer_whole(screen: &Screen, hints: &[&str]) -> bool {
    hints.iter().all(|hint| screen.shown_whole(hint))
}

/// Scenario: The board card details footer and a long attachment error are shown whole
#[tokio::test(start_paused = true)]
async fn the_board_card_details_footer_and_a_long_attachment_error_are_shown_whole() -> TestResult {
    // @step Given the board card details are open at 80x24
    let path = token("spec/attachments/AUTH-001/", 57) + ".md";
    let mut fx = board_fixture(Arc::new(MockBackend::new()));
    open_card_details(&mut fx, &path).await?;
    let mut checks = Checks::default();
    let screen = Screen::of(&mut fx.app, 80, 24);

    // @step Then the footer is on screen whole, including "wheel scroll" and "y copy id"
    checks.check(
        footer_whole(&screen, CARD_FOOTER),
        "80x24: the card details footer is not shown whole",
        &screen,
    );

    // @step When opening an attachment with a 60-character path fails with an 80-character message
    let message = prose("attachment", 80, "unreadable");
    fx.mock.script_card_attachment(Err(message.clone()));
    key(&mut fx, KeyCode::Tab).await?;
    key(&mut fx, KeyCode::Enter).await?;
    assert_eq!(
        fx.mock.card_attachment_requests().len(),
        1,
        "{path} was opened"
    );

    // @step Then the error is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    for part in [path.as_str(), message.as_str()] {
        checks.check(
            screen.shown_whole(part),
            &format!("80x24: the attachment error does not show {part:?} whole"),
            &screen,
        );
    }
    // @step And at 120x40 the footer and the error are on screen whole
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        footer_whole(&screen, CARD_FOOTER),
        "120x40: the card details footer is not shown whole",
        &screen,
    );
    for part in [path.as_str(), message.as_str()] {
        checks.check(
            screen.shown_whole(part),
            &format!("120x40: the attachment error does not show {part:?} whole"),
            &screen,
        );
    }
    checks.done()
}

const ADD_WORKSPACE: &str = "Add workspace (path";

/// Scenario: Add workspace shows the end of a long path and the whole error
#[tokio::test(start_paused = true)]
async fn add_workspace_shows_the_end_of_a_long_path_and_the_whole_error() -> TestResult {
    // @step Given the Add workspace dialog is open
    let mut fx = fixture();
    fx.app.dispatch(Action::OpenAddWorkspaceDialog);
    settle(&mut fx.app).await?;
    let mut checks = Checks::default();

    // @step When the user pastes a 200-character path
    let path = token("/home/user/projects/clients/", 187) + "/the-path-end";
    assert_eq!(path.chars().count(), 200);
    paste(&mut fx, &path).await?;
    let tail: String = path.chars().skip(180).collect();

    // @step Then the path's last 20 characters are on screen at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, ADD_WORKSPACE, &tail),
        &format!("80x24: the path's last 20 characters {tail:?} are not on screen"),
        &screen,
    );

    // @step When adding the workspace fails with a 300-character error
    let error = prose("workspace", 300, "not-an-fspec-project");
    fx.mock.fail_add_workspace(&error);
    key(&mut fx, KeyCode::Enter).await?;

    // @step Then the error is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        dialog_shows(&screen, ADD_WORKSPACE, &error),
        "80x24: the add-workspace error is not shown whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        dialog_shows(&screen, ADD_WORKSPACE, &error),
        "120x40: the add-workspace error is not shown whole",
        &screen,
    );
    checks.done()
}

/// Scenario: A long profile form status is shown whole
#[tokio::test(start_paused = true)]
async fn a_long_profile_form_status_is_shown_whole() -> TestResult {
    // @step Given the Provider Settings profile form is open
    let mut fx = fixture();
    fx.app.dispatch(Action::OpenProviderSettingsView);
    settle(&mut fx.app).await?;
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "work-vllm".into();
    fx.app.navigator_mut().provider_settings.mode = ProviderSettingsMode::CreateProfile {
        provider_id: "openai".into(),
        form,
    };
    let mut checks = Checks::default();

    // @step When saving the profile fails with a 250-character error
    let error = prose("save", 250, "permission-denied");
    fx.mock.set_save_profile_error(error.clone());
    fx.app.dispatch(Action::SaveProfile {
        provider_id: "openai".into(),
        profile_name: "work-vllm".into(),
        old_profile_name: None,
        definition: ProfileDefinition {
            base_url: "http://localhost:8888".into(),
            ..ProfileDefinition::default()
        },
    });
    settle(&mut fx.app).await?;
    assert!(
        matches!(
            fx.app.navigator().provider_settings.mode,
            ProviderSettingsMode::CreateProfile { .. }
        ),
        "the profile form stays open"
    );

    // @step Then the error is on screen whole at 80x24
    let screen = Screen::of(&mut fx.app, 80, 24);
    checks.check(
        screen.shown_whole(&error),
        "80x24: the profile form status is not shown whole",
        &screen,
    );
    // @step And it is on screen whole at 120x40
    let screen = Screen::of(&mut fx.app, 120, 40);
    checks.check(
        screen.shown_whole(&error),
        "120x40: the profile form status is not shown whole",
        &screen,
    );
    checks.done()
}
