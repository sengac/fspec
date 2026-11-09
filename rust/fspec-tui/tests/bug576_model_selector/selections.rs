//! Selections are tied to the latest one: an older missing-limit answer does
//! not reselect its model, and no-session selections create one session.

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::App;
use codelet_rpc_types::{ProfileDefinition, ProviderProfileConfig, ProviderSettingsConfig};
use codelet_sessions::profile_limits::{LimitField, MissingProfileLimit};
use codelet_test_helpers::{deadlock_guard, OrFail, TestError, TestResult};

use super::support::{
    app, cloud, drain, mock, model, open_session, profile, pump_until, session, session_info,
};
use crate::common::TaskOutcome;

fn missing_limit() -> MissingProfileLimit {
    MissingProfileLimit {
        model: "openai:local/x".to_string(),
        provider: "openai".to_string(),
        profile: "local".to_string(),
        field: LimitField::ContextWindow,
        config_path: "/tmp/fspec-config.json".to_string(),
    }
}

/// Finish every task and dispatch everything, keeping the last
/// missing-limit prompt that opened.
async fn drain_keeping_prompt(app: &mut App) -> Result<Option<Action>, TestError> {
    let prompt = deadlock_guard("every task to finish", async {
        let mut prompt = None;
        loop {
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
            let Some(action) = app.try_recv_action() else {
                return Ok::<_, TestError>(prompt);
            };
            if matches!(action, Action::OpenMissingProfileLimitPrompt { .. }) {
                prompt = Some(action.clone());
            }
            app.dispatch(action);
        }
    })
    .await??;
    Ok(prompt)
}

#[tokio::test(start_paused = true)]
async fn an_answer_to_an_older_missing_limit_prompt_does_not_select_its_model_again() -> TestResult
{
    // @step Given I selected model X, which is missing a limit, and then selected model Y
    let mock = mock(vec![profile(
        "openai",
        "local",
        vec![model("x"), model("y")],
    )]);
    mock.seed_sessions(vec![session_info("s1", "openai:local", "y")]);
    mock.seed_provider_settings(vec![ProviderSettingsConfig {
        provider_id: "openai".to_string(),
        settings: ProfileDefinition::default(),
        profiles: vec![ProviderProfileConfig {
            name: "local".to_string(),
            definition: ProfileDefinition {
                base_url: "http://127.0.0.1:9/v1".to_string(),
                ..ProfileDefinition::default()
            },
        }],
    }])
    .or_fail("seed settings")?;
    mock.model_selections
        .fail_model("x", &missing_limit().to_string());
    let mut app = app(&mock);
    open_session(&mut app, "s1").await?;
    app.dispatch(Action::ModelSelected(
        Some(session("s1")),
        "openai:local".to_string(),
        "x".to_string(),
    ));
    app.dispatch(Action::ModelSelected(
        Some(session("s1")),
        "openai:local".to_string(),
        "y".to_string(),
    ));
    let prompt = drain_keeping_prompt(&mut app).await?;
    let Some(Action::OpenMissingProfileLimitPrompt { limit, retry }) = prompt else {
        return Err("X's missing-limit prompt did not open".into());
    };

    // @step When I answer X's missing-limit prompt
    app.dispatch(Action::SaveMissingProfileLimit {
        limit,
        value: 64_000,
        retry,
    });
    drain(&mut app).await?;

    // @step Then the limit is saved and the session stays on Y
    assert_eq!(mock.save_profile_calls(), 1, "the limit was not saved");
    let last = mock.model_selections.calls().last().cloned();
    assert_eq!(
        last,
        Some((session("s1"), "openai:local".to_string(), "y".to_string()))
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn two_selections_with_no_session_create_one_session() -> TestResult {
    // @step Given no session is open
    let mock = mock(vec![cloud(
        "anthropic",
        "Anthropic",
        vec![model("a1"), model("a2")],
    )]);
    let mut app = app(&mock);
    mock.gates.gate("create_session");

    // @step When I select a model and select another before the session is created
    app.dispatch(Action::ModelSelected(None, "anthropic".into(), "a1".into()));
    pump_until(&mut app, "the session creation to be held", |_| {
        mock.gates.held("create_session") == 1
    })
    .await?;
    app.dispatch(Action::ModelSelected(None, "anthropic".into(), "a2".into()));
    mock.gates.open("create_session");
    drain(&mut app).await?;

    // @step Then one session is created and it uses the second model
    assert_eq!(mock.create_session_calls(), 1, "more than one session");
    let last = mock.model_selections.calls().last().cloned();
    assert_eq!(
        last,
        Some((
            session("s-mock-default"),
            "anthropic".to_string(),
            "a2".to_string()
        ))
    );
    Ok(())
}
