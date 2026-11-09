//! MODEL-011: the thinking dialog and `/thinking` offer only the current
//! model's levels.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//!
//! Each model is a real `ThinkingTarget` (the MODEL-010 wire table with its
//! MODEL-009 / MODEL-016 capability). The backend's offer for it is computed
//! by `codelet_sessions::thinking_options::options_for`, exactly as the
//! daemon's `get_thinking_options` does, and fed to the dialog or to a
//! `MockBackend` behind the App. What a chosen level sends is that target's
//! own wire form.

mod common;

use std::sync::Arc;

use crate::common::TaskOutcome;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, Component, FspecBackend, ThinkingLevelDialog};
use codelet_providers::local_reasoning::{LocalEngine, LocalReasoning};
use codelet_providers::models::{
    BudgetRange, NumericKind, NumericRange, ReasoningCapability, ReasoningLevel, ReasoningOption,
    TemplateToggle,
};
use codelet_providers::thinking_wire::{ThinkingTarget, ThinkingWire, TurnLevel};
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_sessions::thinking_options::options_for;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use common::MockBackend;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serde_json::{json, Number, Value};

// ───────────────────────── models ─────────────────────────────────────────

fn sid() -> SessionId {
    SessionId::new("s-1")
}

/// A catalog model on `wire` advertising `options` (models.dev shapes).
fn catalog(
    wire: ThinkingWire,
    model: &str,
    options: &[ReasoningOption],
) -> Result<ThinkingTarget, TestError> {
    let api = model.split_once('/').map_or(model, |(_, id)| id);
    let capability = ReasoningCapability::resolve(model, Some(options))?;
    Ok(ThinkingTarget::new(Ok(wire), model, api, Ok(capability)))
}

fn efforts(values: &[&str]) -> ReasoningOption {
    ReasoningOption::Effort(values.iter().map(ToString::to_string).collect())
}

fn budget(min: Option<u64>, max: Option<u64>) -> ReasoningOption {
    ReasoningOption::BudgetTokens(BudgetRange { min, max })
}

fn gpt_5_5() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::OpenAiChat,
        "openai/gpt-5.5",
        &[efforts(&["none", "low", "medium", "high", "xhigh"])],
    )
}

fn gpt_5() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::OpenAiChat,
        "openai/gpt-5",
        &[efforts(&["minimal", "low", "medium", "high"])],
    )
}

fn haiku_4_5() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::Anthropic,
        "anthropic/claude-haiku-4-5",
        &[budget(Some(1024), None)],
    )
}

/// A local profile model on `engine` declaring `capability` (MODEL-016).
fn local(engine: LocalEngine, model: &str, capability: ReasoningCapability) -> ThinkingTarget {
    ThinkingTarget::local_profile(
        Ok(ThinkingWire::LocalOpenAi),
        model,
        model,
        Ok(Some(LocalReasoning {
            profile: "local".to_string(),
            engine,
            capability,
        })),
    )
}

fn whole(min: u64, max: u64) -> Result<NumericRange, TestError> {
    Ok(NumericRange::new(
        NumericKind::Whole,
        Number::from(min),
        Number::from(max),
    )?)
}

fn strata_numeric_and_budget() -> Result<ThinkingTarget, TestError> {
    Ok(local(
        LocalEngine::Strata,
        "local/qwen3.8-flash-next",
        ReasoningCapability::declared(
            &[],
            Some(whole(1, 100)?),
            Some(BudgetRange {
                min: Some(128),
                max: Some(32768),
            }),
            None,
        ),
    ))
}

fn effort(name: &str) -> ThinkingLevel {
    ThinkingLevel::Effort(name.to_string())
}

/// What a turn at `level` on `target` sends.
fn sent(target: &ThinkingTarget, level: ReasoningLevel) -> Result<Value, TestError> {
    Ok(target
        .config_for(Some(&TurnLevel::Level(level)))?
        .or_fail("the turn sends a thinking parameter")?)
}

// ───────────────────────── dialog ─────────────────────────────────────────

fn dialog_for(target: &ThinkingTarget, current: ThinkingLevel) -> ThinkingLevelDialog {
    ThinkingLevelDialog::new(sid(), current, options_for(target))
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn type_text(dialog: &mut ThinkingLevelDialog, text: &str) {
    for c in text.chars() {
        dialog.handle_event(&key(KeyCode::Char(c)));
    }
}

fn select_row(dialog: &mut ThinkingLevelDialog, label: &str) -> TestResult {
    let index = dialog
        .row_labels()
        .iter()
        .position(|row| row == label)
        .or_fail("the dialog has the row")?;
    while dialog.selected_index() != index {
        dialog.handle_event(&key(KeyCode::Down));
    }
    Ok(())
}

fn painted(dialog: &mut ThinkingLevelDialog) -> Result<String, TestError> {
    let mut term = Terminal::new(TestBackend::new(100, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| Component::render(dialog, frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    Ok(out)
}

// ───────────────────────── app ────────────────────────────────────────────

/// An App with session s-1 whose backend offers `target`'s levels.
fn app_for(target: &ThinkingTarget) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    mock.set_thinking_options(options_for(target));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid()));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Point the App's backend at another model.
fn switch_model(mock: &MockBackend, target: &ThinkingTarget) {
    mock.set_thinking_options(options_for(target));
}

async fn drain(app: &mut App) -> TestResult {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// Submit `text` and let the App's backend round trips finish.
async fn submit(app: &mut App, text: &str) -> TestResult {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, text.to_string()));
    }
    drain(app).await
}

fn scrollback(app: &App) -> String {
    app.agent_view_store()
        .session_context_for(&sid())
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| {
            chunk.lines.iter().map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        // Lines wrap at spaces; joining with a space restores the notice.
        .join(" ")
}

fn header_row(app: &mut App) -> Result<String, TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    Ok((0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect())
}

fn last_set(mock: &MockBackend) -> Option<ThinkingLevel> {
    mock.last_set_thinking_level().map(|(_, level)| level)
}

// ───────────────────────── scenarios ──────────────────────────────────────

/// Scenario: An OpenAI model with a none effort lists its efforts in canonical order and no separate Off
#[test]
fn an_openai_model_with_a_none_effort_lists_its_efforts_and_no_separate_off() -> TestResult {
    // @step Given the session's model is openai/gpt-5.5, which advertises the efforts none, low, medium, high and xhigh
    let target = gpt_5_5()?;
    // @step When the user opens the thinking dialog
    let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists none, low, medium, high, xhigh and Model default, in that order
    assert_eq!(
        dialog.row_labels(),
        ["none", "low", "medium", "high", "xhigh", "Model default"]
    );
    // @step And the dialog has no Off row and no More… row
    let screen = painted(&mut dialog)?;
    assert!(!screen.contains("Off"), "no Off row: {screen}");
    assert!(!screen.contains("More"), "no More… row: {screen}");
    Ok(())
}

/// Scenario: A model without an off form offers no Off and refuses /thinking off
#[tokio::test]
async fn a_model_without_an_off_form_offers_no_off_and_refuses_thinking_off() -> TestResult {
    // @step Given the session's model is openai/gpt-5, which advertises the efforts minimal, low, medium and high and no none
    let target = gpt_5()?;
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists minimal, low, medium, high and Model default
    assert_eq!(
        dialog.row_labels(),
        ["minimal", "low", "medium", "high", "Model default"]
    );
    // @step And /thinking off is refused naming openai/gpt-5 and listing minimal, low, medium, high
    let (mut app, mock) = app_for(&target);
    submit(&mut app, "/thinking off").await?;
    let notice = scrollback(&app);
    assert!(
        notice.contains("openai/gpt-5") && notice.contains("minimal, low, medium, high"),
        "the refusal names the model and its levels: {notice}"
    );
    assert_eq!(last_set(&mock), None, "nothing is set");
    Ok(())
}

/// Scenario: A Claude model whose thinking cannot be turned off offers its efforts without Off
#[test]
fn a_claude_model_whose_thinking_cannot_be_turned_off_offers_its_efforts_without_off() -> TestResult
{
    // @step Given the session's model is anthropic/claude-opus-5-5, which advertises the efforts low, medium, high, xhigh and max
    let target = catalog(
        ThinkingWire::Anthropic,
        "anthropic/claude-opus-5-5",
        &[efforts(&["low", "medium", "high", "xhigh", "max"])],
    )?;
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists low, medium, high, xhigh, max and Model default
    assert_eq!(
        dialog.row_labels(),
        ["low", "medium", "high", "xhigh", "max", "Model default"]
    );
    // @step And choosing xhigh sends output_config effort xhigh
    let config = sent(&target, ReasoningLevel::effort("xhigh")?)?;
    assert_eq!(config["output_config"]["effort"], json!("xhigh"));
    Ok(())
}

/// Scenario: A budget-only model takes a token budget at or above its minimum
#[tokio::test]
async fn a_budget_only_model_takes_a_token_budget_at_or_above_its_minimum() -> TestResult {
    // @step Given the session's model is anthropic/claude-haiku-4-5, which advertises only a token budget with a minimum of 1024
    let target = haiku_4_5()?;
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists Off, Token budget 1024 or more and Model default
    assert_eq!(
        dialog.row_labels(),
        ["Off", "Token budget 1024 or more", "Model default"]
    );
    // @step And a budget of 8000 is stored and sent as thinking type enabled with budget_tokens 8000
    let (mut app, mock) = app_for(&target);
    submit(&mut app, "/thinking 8000").await?;
    assert_eq!(last_set(&mock), Some(ThinkingLevel::Budget(8000)));
    assert_eq!(
        sent(&target, ReasoningLevel::Budget(8000))?,
        json!({"thinking": {"type": "enabled", "budget_tokens": 8000}})
    );
    // @step And a budget of 512 is refused naming anthropic/claude-haiku-4-5 and the minimum 1024, and the session keeps its level
    submit(&mut app, "/thinking 512").await?;
    let notice = scrollback(&app);
    assert!(
        notice.contains("anthropic/claude-haiku-4-5") && notice.contains("1024"),
        "the refusal names the model and its minimum: {notice}"
    );
    assert_eq!(last_set(&mock), Some(ThinkingLevel::Budget(8000)));
    Ok(())
}

/// Scenario: The token budget row takes a number in the dialog
#[test]
fn the_token_budget_row_takes_a_number_in_the_dialog() -> TestResult {
    // @step Given the session's model is google/gemini-2.5-pro, which advertises a token budget of 128 to 32768
    let target = catalog(
        ThinkingWire::Gemini,
        "google/gemini-2.5-pro",
        &[budget(Some(128), Some(32768))],
    )?;
    let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step And the thinking dialog is open on the row Token budget 128–32768
    select_row(&mut dialog, "Token budget 128–32768")?;
    // @step When the user presses Enter, types 40000 and presses Enter
    dialog.handle_event(&key(KeyCode::Enter));
    type_text(&mut dialog, "40000");
    dialog.handle_event(&key(KeyCode::Enter));
    // @step Then the number field shows the refusal naming the range 128 to 32768 and stays open
    let (text, refusal) = dialog.number_field().or_fail("the number field is open")?;
    assert_eq!(text, "40000");
    let refusal = refusal.or_fail("the field shows a refusal")?;
    assert!(
        refusal.contains("128") && refusal.contains("32768"),
        "{refusal}"
    );
    assert!(dialog.take_pending_action().is_none(), "nothing is chosen");
    assert!(painted(&mut dialog)?.contains(&refusal));
    // @step When the user clears the field, types 8000 and presses Enter
    for _ in 0..text.len() {
        dialog.handle_event(&key(KeyCode::Backspace));
    }
    type_text(&mut dialog, "8000");
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then the session's level is a token budget of 8000 and the dialog closes
    match dialog.take_pending_action() {
        Some(Action::ThinkingLevelSelected(session, level)) => {
            assert_eq!(session, sid());
            assert_eq!(level, ThinkingLevel::Budget(8000));
        }
        other => return Err(format!("expected a budget of 8000, got {other:?}").into()),
    }
    assert!(
        matches!(result, codelet_fspec_tui::EventResult::Consumed(Some(_))),
        "the dialog closes"
    );
    // @step And pressing Esc in the number field returns to the list without changing the level
    let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    select_row(&mut dialog, "Token budget 128–32768")?;
    dialog.handle_event(&key(KeyCode::Enter));
    type_text(&mut dialog, "9000");
    let result = dialog.handle_event(&key(KeyCode::Esc));
    assert!(dialog.number_field().is_none(), "back on the list");
    assert!(
        matches!(result, codelet_fspec_tui::EventResult::Consumed(None)),
        "the dialog stays open"
    );
    assert!(dialog.take_pending_action().is_none(), "nothing is chosen");
    Ok(())
}

/// Scenario: A local model's numeric efforts are offered and sent unchanged
#[test]
fn a_local_models_numeric_efforts_are_offered_and_sent_unchanged() -> TestResult {
    // @step Given the session's model is a Strata profile model DeepSeek-V4.1 declaring the efforts none, low, medium, high and max and whole-number efforts 1 to 100
    let declared: Vec<String> = ["none", "low", "medium", "high", "max"]
        .iter()
        .map(ToString::to_string)
        .collect();
    let target = local(
        LocalEngine::Strata,
        "strata/DeepSeek-V4.1",
        ReasoningCapability::declared(&declared, Some(whole(1, 100)?), None, None),
    );
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists none, low, medium, high, max, Effort 1–100 and Model default
    assert_eq!(
        dialog.row_labels(),
        [
            "none",
            "low",
            "medium",
            "high",
            "max",
            "Effort 1–100",
            "Model default"
        ]
    );
    // @step And choosing the numeric effort 40 sends reasoning_effort 40
    let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    select_row(&mut dialog, "Effort 1–100")?;
    dialog.handle_event(&key(KeyCode::Enter));
    type_text(&mut dialog, "40");
    dialog.handle_event(&key(KeyCode::Enter));
    let chosen = match dialog.take_pending_action() {
        Some(Action::ThinkingLevelSelected(_, level)) => level,
        other => return Err(format!("expected the effort 40, got {other:?}").into()),
    };
    assert_eq!(chosen, ThinkingLevel::Numeric(Number::from(40)));
    assert_eq!(
        sent(&target, ReasoningLevel::Numeric(Number::from(40)))?,
        json!({"reasoning_effort": 40})
    );
    Ok(())
}

/// Scenario: A toggle-only local model offers Off and On
#[test]
fn a_toggle_only_local_model_offers_off_and_on() -> TestResult {
    // @step Given the session's model is a TensorRT-LLM profile model declaring only the template toggle enable_thinking
    let target = local(
        LocalEngine::TensorRtLlm,
        "trt/qwen3-32b",
        ReasoningCapability::declared(&[], None, None, Some(TemplateToggle::EnableThinking)),
    );
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists Off, On and Model default
    assert_eq!(dialog.row_labels(), ["Off", "On", "Model default"]);
    // @step And choosing On sends chat_template_kwargs enable_thinking true
    assert_eq!(
        sent(&target, ReasoningLevel::On)?,
        json!({"chat_template_kwargs": {"enable_thinking": true}})
    );
    Ok(())
}

/// Scenario: A Z.AI toggle-only model offers only Off
#[tokio::test]
async fn a_zai_toggle_only_model_offers_only_off() -> TestResult {
    // @step Given the session's model is zai/glm-4.6, which advertises only a thinking toggle
    let target = catalog(ThinkingWire::Zai, "zai/glm-4.6", &[ReasoningOption::Toggle])?;
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog lists Off and Model default
    assert_eq!(dialog.row_labels(), ["Off", "Model default"]);
    // @step And /thinking on is refused naming zai/glm-4.6
    let (mut app, mock) = app_for(&target);
    submit(&mut app, "/thinking on").await?;
    assert!(
        scrollback(&app).contains("zai/glm-4.6"),
        "{}",
        scrollback(&app)
    );
    assert_eq!(last_set(&mock), None);
    Ok(())
}

/// Scenario: /thinking sets an advertised effort and refuses an unadvertised one
#[tokio::test]
async fn thinking_sets_an_advertised_effort_and_refuses_an_unadvertised_one() -> TestResult {
    // @step Given the session's model is openai/gpt-5.5
    let (mut app, mock) = app_for(&gpt_5_5()?);
    // @step When the user submits /thinking xhigh
    mock.set_thinking_level(effort("xhigh"));
    submit(&mut app, "/thinking xhigh").await?;
    // @step Then the session's level is xhigh and the header shows xhigh
    assert_eq!(last_set(&mock), Some(effort("xhigh")));
    assert_eq!(
        app.agent_view_store().thinking_level_for(&sid()).cloned(),
        Some(effort("xhigh"))
    );
    let header = header_row(&mut app)?;
    assert!(header.contains("[T:xhigh]"), "{header}");
    // @step When the user submits /thinking med
    let calls = mock.set_thinking_level_calls();
    submit(&mut app, "/thinking med").await?;
    // @step Then the notice says openai/gpt-5.5 does not take med and lists none, low, medium, high, xhigh
    let notice = scrollback(&app);
    assert!(
        notice.contains("openai/gpt-5.5 does not take med")
            && notice.contains("none, low, medium, high, xhigh"),
        "{notice}"
    );
    // @step And the session's level is still xhigh
    assert_eq!(mock.set_thinking_level_calls(), calls, "nothing is set");
    assert_eq!(
        app.agent_view_store().thinking_level_for(&sid()).cloned(),
        Some(effort("xhigh"))
    );
    Ok(())
}

/// Scenario: /thinking with a number takes the one numeric kind the model offers
#[tokio::test]
async fn thinking_with_a_number_takes_the_one_numeric_kind_the_model_offers() -> TestResult {
    // @step Given the session's model is anthropic/claude-haiku-4-5, which takes only a token budget
    let (mut app, mock) = app_for(&haiku_4_5()?);
    // @step When the user submits /thinking 8000
    submit(&mut app, "/thinking 8000").await?;
    // @step Then the session's level is a token budget of 8000
    assert_eq!(last_set(&mock), Some(ThinkingLevel::Budget(8000)));
    // @step Given the session's model is a Strata model declaring whole-number efforts 1 to 100 and a token budget of 128 to 32768
    switch_model(&mock, &strata_numeric_and_budget()?);
    // @step When the user submits /thinking 50
    let calls = mock.set_thinking_level_calls();
    submit(&mut app, "/thinking 50").await?;
    // @step Then the notice says 50 is ambiguous and names the effort range 1 to 100 and the token budget 128 to 32768
    let notice = scrollback(&app);
    assert!(
        notice.contains("50 is ambiguous")
            && notice.contains("1 to 100")
            && notice.contains("128 to 32768"),
        "{notice}"
    );
    assert_eq!(mock.set_thinking_level_calls(), calls, "nothing is set");
    // @step When the user submits /thinking effort 50
    submit(&mut app, "/thinking effort 50").await?;
    // @step Then the session's level is the numeric effort 50
    assert_eq!(
        last_set(&mock),
        Some(ThinkingLevel::Numeric(Number::from(50)))
    );
    // @step When the user submits /thinking budget 4096
    submit(&mut app, "/thinking budget 4096").await?;
    // @step Then the session's level is a token budget of 4096
    assert_eq!(last_set(&mock), Some(ThinkingLevel::Budget(4096)));
    Ok(())
}

/// Scenario: A local profile model without a declaration has no reasoning control
#[tokio::test]
async fn a_local_profile_model_without_a_declaration_has_no_reasoning_control() -> TestResult {
    // @step Given the session's model is a vLLM profile model qwen3-32b that declares no thinking levels
    let target = ThinkingTarget::local_profile(
        Ok(ThinkingWire::LocalOpenAi),
        "vllm/qwen3-32b",
        "qwen3-32b",
        Ok(None),
    );
    // @step When the user opens the thinking dialog
    let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
    // @step Then the dialog says qwen3-32b has no reasoning control and lists only Model default
    assert_eq!(dialog.row_labels(), ["Model default"]);
    let screen = painted(&mut dialog)?;
    assert!(
        screen.contains("qwen3-32b") && screen.contains("no reasoning control"),
        "{screen}"
    );
    // @step And /thinking high is refused with the same reason
    let (mut app, mock) = app_for(&target);
    submit(&mut app, "/thinking high").await?;
    let notice = scrollback(&app);
    assert!(
        notice.contains("qwen3-32b") && notice.contains("no reasoning control"),
        "{notice}"
    );
    assert_eq!(last_set(&mock), None);
    Ok(())
}

/// Scenario: Rhai provider models and models missing from the catalog have no reasoning control
#[tokio::test]
async fn rhai_and_uncatalogued_models_have_no_reasoning_control() -> TestResult {
    // @step Given the session's model is a Rhai custom-provider model or a cloud model missing from models.dev
    let rhai = ThinkingTarget::new(Ok(ThinkingWire::Rhai), "my-rhai/coder", "coder", Ok(None));
    let missing = ThinkingTarget::new(
        Ok(ThinkingWire::Anthropic),
        "anthropic/claude-next",
        "claude-next",
        Ok(None),
    );
    for (target, model) in [(rhai, "my-rhai/coder"), (missing, "anthropic/claude-next")] {
        // @step When the user opens the thinking dialog
        let mut dialog = dialog_for(&target, ThinkingLevel::ModelDefault);
        // @step Then the dialog says the model has no reasoning control and lists only Model default
        assert_eq!(dialog.row_labels(), ["Model default"]);
        let screen = painted(&mut dialog)?;
        assert!(
            screen.contains(model) && screen.contains("no reasoning control"),
            "{screen}"
        );
        // @step And /thinking high is refused with the same reason
        let (mut app, mock) = app_for(&target);
        submit(&mut app, "/thinking high").await?;
        let notice = scrollback(&app);
        assert!(
            notice.contains(model) && notice.contains("no reasoning control"),
            "{notice}"
        );
        assert_eq!(last_set(&mock), None);
    }
    Ok(())
}

/// Scenario: The dialog opened on a level the model does not offer marks no row current
#[test]
fn the_dialog_opened_on_an_unoffered_level_marks_no_row_current() -> TestResult {
    // @step Given the session's level is xhigh and its model is now openai/gpt-5, which does not offer xhigh
    let target = gpt_5()?;
    // @step When the user opens the thinking dialog
    let dialog = dialog_for(&target, effort("xhigh"));
    // @step Then no row is marked current and the selection starts on minimal
    assert_eq!(dialog.current_index(), None);
    assert_eq!(dialog.selected_index(), 0);
    assert_eq!(dialog.row_labels()[0], "minimal");
    // @step And a turn at xhigh is refused naming openai/gpt-5 and listing minimal, low, medium, high
    let refusal = match target.config_for(Some(&TurnLevel::Level(ReasoningLevel::effort("xhigh")?)))
    {
        Ok(config) => return Err(format!("expected a refusal, got {config:?}").into()),
        Err(reason) => reason,
    };
    assert!(
        refusal.contains("openai/gpt-5") && refusal.contains("minimal, low, medium, high"),
        "{refusal}"
    );
    Ok(())
}
