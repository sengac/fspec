//! TUI-129 question scenario: the empty-answer hint names what Esc does.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use codelet_fspec_tui::{Action, App, ViewMode};
use codelet_rpc_types::{HitlOption, HitlQuestion, HitlRequest};
use codelet_test_helpers::{TestError, TestResult};
use crossterm::event::KeyCode;

use super::fixture::*;
use crate::common::hitl_route;

fn request(options: &[&str]) -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".to_string(),
            header: "Header".to_string(),
            question: "Question?".to_string(),
            options: options
                .iter()
                .map(|label| HitlOption {
                    label: (*label).to_string(),
                    description: String::new(),
                })
                .collect(),
        }],
    }
}

/// s-1 on the agent view with `request` asked.
async fn asked(request: HitlRequest) -> Result<App, TestError> {
    let (mut app, _mock) = fresh_app();
    open_sessions(&mut app, 1).await?;
    app.navigator_mut().active_view = ViewMode::Agent;
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        route: hitl_route(&sid("s-1")),
        request,
    });
    drain(&mut app).await?;
    screen(&mut app, 100, 20)?;
    Ok(app)
}

/// Scenario: The empty-answer hint names what Esc does
#[tokio::test(start_paused = true)]
async fn the_empty_answer_hint_names_what_esc_does() -> TestResult {
    // @step Given a question with no options is shown
    let mut app = asked(request(&[])).await?;

    // @step When the user presses Enter on an empty answer
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the hint says "press Esc to cancel"
    let rows = screen(&mut app, 100, 20)?;
    assert!(
        shows(&rows, "⚠ Please type a response or press Esc to cancel"),
        "{}",
        rows.join("\n")
    );

    // @step Given a question with options where the user chose Other
    let mut app = asked(request(&["Yes", "No"])).await?;
    press(&mut app, KeyCode::Up).await?;
    press(&mut app, KeyCode::Enter).await?;
    // The view reads the prompt's mode from the frame it last painted.
    screen(&mut app, 100, 20)?;

    // @step When the user presses Enter on an empty answer
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the hint says "press Esc to go back"
    let rows = screen(&mut app, 100, 20)?;
    assert!(
        shows(&rows, "⚠ Please type a response or press Esc to go back"),
        "{}",
        rows.join("\n")
    );
    Ok(())
}
