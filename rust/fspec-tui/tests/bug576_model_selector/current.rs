//! The current model is the session's provider and model together.

use codelet_fspec_tui::components::Action;
use codelet_test_helpers::TestResult;

use super::support::{
    app, cloud, drain, focused, mock, model, open, open_session, profile, pump_until, selector,
    session, session_info, view,
};

const SESSIONS: &str = "list_sessions";

#[tokio::test(start_paused = true)]
async fn the_current_model_is_matched_by_provider_and_model() -> TestResult {
    // @step Given codex and openai:local both list m1 and the session uses openai:local m1
    let mock = mock(vec![
        cloud("codex", "Codex", vec![model("m1")]),
        profile("openai", "local", vec![model("m1")]),
    ]);
    mock.seed_sessions(vec![session_info("s1", "openai:local", "m1")]);
    let mut app = app(&mock);
    open_session(&mut app, "s1").await?;

    // @step When I open /model
    open(&mut app).await?;

    // @step Then only openai:local is expanded and only its m1 is marked (current)
    assert!(
        view(&app).is_expanded("openai:local"),
        "openai:local collapsed"
    );
    assert!(!view(&app).is_expanded("codex"), "codex expanded");
    let text = selector(&mut app, 120).0.join("\n");
    assert_eq!(text.matches("(current)").count(), 1, "{text}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_current_model_that_becomes_known_after_the_list_opens_its_section() -> TestResult {
    // @step Given I opened /model before the session's model was known and the list has answered
    let mock = mock(vec![
        cloud("anthropic", "Anthropic", vec![model("a1")]),
        profile("openai", "local", vec![model("m1")]),
    ]);
    mock.seed_sessions(vec![session_info("s1", "openai:local", "m1")]);
    let mut app = app(&mock);
    mock.gates.gate(SESSIONS);
    app.dispatch(Action::SessionCreated(session("s1")));
    app.dispatch(Action::OpenAgentView(Some(session("s1"))));
    pump_until(&mut app, "the session's model lookup to be held", |_| {
        mock.gates.held(SESSIONS) >= 1
    })
    .await?;
    app.dispatch(Action::OpenModelSelectorView);
    pump_until(&mut app, "the list to answer", |app| {
        view(app).providers_loaded()
    })
    .await?;
    assert!(!view(&app).is_expanded("openai:local"));

    // @step When the session's model arrives
    mock.gates.open(SESSIONS);
    drain(&mut app).await?;

    // @step Then its section is expanded and the cursor is on it
    assert!(
        view(&app).is_expanded("openai:local"),
        "openai:local collapsed"
    );
    assert_eq!(focused(&mut app).as_deref(), Some("openai:local/m1"));
    Ok(())
}
