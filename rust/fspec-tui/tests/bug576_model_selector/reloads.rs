//! Reloads keep the expanded sections and the cursor, and only the newest
//! list requested is shown.

use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::KeyCode;

use super::support::{
    app, cloud, custom, drain, focused, mock, model, open, press, profile, pump_until, type_text,
    view, LIST,
};
use crate::common::TaskOutcome;

fn two_clouds() -> Vec<codelet_rpc_types::ProviderInfo> {
    vec![
        cloud("anthropic", "Anthropic", vec![model("a1"), model("a2")]),
        cloud("openai", "OpenAI", vec![model("o1"), model("o2")]),
    ]
}

fn local(models: &[&str]) -> Vec<codelet_rpc_types::ProviderInfo> {
    vec![
        cloud("anthropic", "Anthropic", vec![model("a1")]),
        profile(
            "openai",
            "local",
            models.iter().map(|id| custom(id)).collect(),
        ),
    ]
}

#[tokio::test(start_paused = true)]
async fn a_refresh_keeps_the_sections_i_opened_and_my_cursor() -> TestResult {
    // @step Given the model selector is open with Anthropic expanded and the cursor on one of its models
    let mock = mock(two_clouds());
    let mut app = app(&mock);
    open(&mut app).await?;
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    assert_eq!(focused(&mut app).as_deref(), Some("anthropic/a2"));

    // @step When I press r and the refreshed list answers
    press(&mut app, KeyCode::Char('r'));
    drain(&mut app).await?;

    // @step Then Anthropic is still expanded and the cursor is on the same model
    assert!(view(&app).is_expanded("anthropic"), "anthropic collapsed");
    assert_eq!(focused(&mut app).as_deref(), Some("anthropic/a2"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn an_added_custom_models_section_opens_on_it() -> TestResult {
    // @step Given the profile local is collapsed in the model selector
    let mock = mock(local(&["m1"]));
    let mut app = app(&mock);
    open(&mut app).await?;
    assert!(!view(&app).is_expanded("openai:local"));

    // @step When I add the custom model m2 under local and the reloaded list answers
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('a'));
    type_text(&mut app, "m2");
    mock.seed_providers(local(&["m1", "m2"]));
    press(&mut app, KeyCode::Enter);
    drain(&mut app).await?;

    // @step Then local is expanded and the cursor is on m2
    assert!(view(&app).is_expanded("openai:local"), "local collapsed");
    assert_eq!(focused(&mut app).as_deref(), Some("openai:local/m2"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn an_older_list_that_answers_last_is_dropped() -> TestResult {
    // @step Given I pressed r and added a custom model before the refresh's list answered
    let mock = mock(local(&["m1"]));
    let mut app = app(&mock);
    open(&mut app).await?;
    mock.gates.gate(LIST);
    press(&mut app, KeyCode::Char('r'));
    pump_until(&mut app, "the refresh's list to be held", |_| {
        mock.gates.held(LIST) == 1
    })
    .await?;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('a'));
    type_text(&mut app, "m2");
    mock.seed_providers(local(&["m1", "m2"]));
    press(&mut app, KeyCode::Enter);
    pump_until(&mut app, "the add's list to be held", |_| {
        mock.gates.held(LIST) == 2
    })
    .await?;

    // @step When the add's list answers before the refresh's list
    let add = app.next_pending_task().or_fail("the add's task")?;
    let refresh = app.next_pending_task().or_fail("the refresh's task")?;
    mock.gates.release(LIST, 1);
    add.await.completed_or_cancelled()?;
    drain(&mut app).await?;
    mock.gates.release(LIST, 0);
    refresh.await.completed_or_cancelled()?;
    drain(&mut app).await?;

    // @step Then the new custom model is still listed
    assert_eq!(view(&app).total_model_count(), 3, "m2 is no longer listed");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_refresh_mark_stays_until_the_newest_refresh_answers() -> TestResult {
    // @step Given I pressed r twice
    let mock = mock(two_clouds());
    let mut app = app(&mock);
    open(&mut app).await?;
    mock.gates.gate(LIST);
    press(&mut app, KeyCode::Char('r'));
    pump_until(&mut app, "the first refresh's list to be held", |_| {
        mock.gates.held(LIST) == 1
    })
    .await?;
    press(&mut app, KeyCode::Char('r'));
    pump_until(&mut app, "the second refresh's list to be held", |_| {
        mock.gates.held(LIST) == 2
    })
    .await?;
    let second = app.next_pending_task().or_fail("the second refresh")?;
    let first = app.next_pending_task().or_fail("the first refresh")?;

    // @step When the first refresh's list answers
    mock.gates.release(LIST, 0);
    first.await.completed_or_cancelled()?;
    drain(&mut app).await?;

    // @step Then the title still says "(refreshing...)"
    assert!(view(&app).title_text().contains("(refreshing...)"));

    // @step And once the second refresh's list answers it does not
    mock.gates.release(LIST, 1);
    second.await.completed_or_cancelled()?;
    drain(&mut app).await?;
    assert!(!view(&app).title_text().contains("(refreshing...)"));
    Ok(())
}
