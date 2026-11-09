//! BUG-577: a failing Codex /models never replaces the saved model, and the
//! model selector words and handles an empty Codex section.
//!
//! Feature: spec/features/codex-models-never-block-the-provider-list-or-replace-the-saved-model.feature
//!
//! The startup scenario drives the real `App::bootstrap` against
//! `MockBackend`, with the saved model in a throwaway user directory
//! (`FSPEC_USER_DIR`, hence `#[serial]`). The model selector scenarios
//! render the real `ModelSelectorView` and drive its public key API.

use codelet_fspec_tui::views::ModelSelectorView;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId};
use codelet_sessions::last_used_model_persistence::{
    load_persisted_model_string_from, save_persisted_model_string_to,
};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serial_test::serial;

mod common;
use common::harness::AppTestHarness;

const SAVED: &str = "codex/gpt-6.1-terra";
const FAILED: &str = "model list unavailable: the Codex /models request failed: \
                      error sending request for url (https://chatgpt.com/backend-api/codex/models)";

fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(200_000),
        supports_reasoning: true,
        supports_vision: false,
        is_custom: false,
    }
}

fn codex_without_models(reason: &str) -> ProviderInfo {
    ProviderInfo {
        key: "codex".to_string(),
        display_name: "Codex (ChatGPT)".to_string(),
        provider_id: "codex".to_string(),
        unavailable_reason: Some(reason.to_string()),
        ..ProviderInfo::default()
    }
}

fn profile_with(model_id: &str) -> ProviderInfo {
    ProviderInfo {
        key: "openai:strata".to_string(),
        display_name: "openai: strata".to_string(),
        models: vec![model(model_id)],
        profile_name: Some("strata".to_string()),
        provider_id: "openai".to_string(),
        ..ProviderInfo::default()
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// A model selector listing an anthropic section and the empty Codex one.
fn selector(reason: &str) -> ModelSelectorView {
    let anthropic = ProviderInfo {
        key: "anthropic".to_string(),
        display_name: "anthropic".to_string(),
        models: vec![model("claude-opus-5-5")],
        provider_id: "anthropic".to_string(),
        ..ProviderInfo::default()
    };
    let mut view = ModelSelectorView::new();
    view.set_session(Some(SessionId::new("s-1")));
    view.set_providers(vec![codex_without_models(reason), anthropic]);
    view
}

fn screen(view: &mut ModelSelectorView, width: u16) -> Result<Vec<String>, TestError> {
    let mut terminal = Terminal::new(TestBackend::new(width, 16)).or_fail("terminal")?;
    terminal
        .draw(|frame| view.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buffer = terminal.backend().buffer();
    Ok((0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn a_saved_codex_model_is_kept_when_codex_models_fails_at_startup() -> TestResult {
    // @step Given my saved model is codex/gpt-6.1-terra
    let user = tempfile::tempdir()?;
    save_persisted_model_string_to(user.path(), SAVED)?;
    std::env::set_var("FSPEC_USER_DIR", user.path());
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;

    // @step And the provider list's Codex section lists no models because the Codex /models request failed
    harness.mock.seed_providers(vec![
        profile_with("qwen3.8-flash-next"),
        codex_without_models(FAILED),
    ]);

    // @step When fspec starts
    let started = harness.app.bootstrap().await;
    std::env::remove_var("FSPEC_USER_DIR");
    started?;
    harness.drain_pending().await?;

    // @step Then the startup model is codex/gpt-6.1-terra
    let calls = harness.mock.set_default_model_calls();
    assert!(calls <= 1, "set_default_model called {calls} times");
    if calls == 1 {
        assert_eq!(
            harness.mock.last_set_default_model().as_deref(),
            Some(SAVED)
        );
    }

    // @step And no other model is saved
    assert_eq!(
        load_persisted_model_string_from(user.path())?.as_deref(),
        Some(SAVED)
    );
    Ok(())
}

#[test]
fn a_long_reason_is_shortened_with_an_ellipsis_in_the_model_selector() -> TestResult {
    // @step Given the Codex section lists no models with a reason longer than the row
    let mut view = selector(FAILED);

    // @step When the model selector is drawn 80 columns wide
    let screen = screen(&mut view, 80)?;

    // @step Then the Codex header shows the start of the reason and ends with an ellipsis
    let header = screen
        .iter()
        .find(|line| line.contains("Codex (ChatGPT)"))
        .or_fail("the Codex header")?;
    assert!(header.contains("(model list unavailable"), "{header}");
    assert!(header.ends_with('…'), "{header:?}");
    assert!(header.chars().count() <= 80, "{header:?}");
    Ok(())
}

#[test]
fn an_empty_codex_section_with_a_reason_does_not_expand() -> TestResult {
    // @step Given the Codex section lists no models with a reason
    let mut view = selector("Codex models need ChatGPT sign-in; sign in under /provider");
    let rows_before = screen(&mut view, 120)?;
    let codex_row = rows_before
        .iter()
        .position(|line| line.contains("Codex (ChatGPT)"))
        .or_fail("the Codex header")?;
    for _ in 0..rows_before.len() {
        if view.selected_index() == 0 {
            break;
        }
        view.handle_key(key(KeyCode::Up));
    }
    assert!(codex_row > 0, "{rows_before:?}");

    // @step When the user presses Enter and then Right on the Codex header
    view.handle_key(key(KeyCode::Enter));
    view.handle_key(key(KeyCode::Right));

    // @step Then the Codex section stays collapsed
    assert!(!view.is_expanded("codex"));
    let rows_after = screen(&mut view, 120)?;
    assert!(
        rows_after
            .iter()
            .any(|line| line.contains("▶ Codex (ChatGPT)")),
        "{rows_after:?}"
    );
    Ok(())
}
