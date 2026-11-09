//! MODEL-012: the thinking level is remembered per model (TUI side).
//!
//! Feature: spec/features/thinking-level-remembered-per-model.feature
//!
//! Each model is a real `ThinkingTarget` whose offer the daemon's
//! `options_for` computes, as in the MODEL-011 tests. The saved levels live
//! in a throwaway process-global data directory's `fspec-config.json` under
//! `tui.thinkingLevels`; `DATA_DIR_GUARD` serialises the tests that set it.

mod common;

use std::sync::Arc;

use crate::common::TaskOutcome;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, Component, FspecBackend, ThinkingLevelDialog};
use codelet_providers::models::{BudgetRange, ReasoningCapability, ReasoningOption};
use codelet_providers::thinking_wire::{ThinkingTarget, ThinkingWire};
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_sessions::default_thinking_level_persistence::save_default_thinking_level_with_dirs;
use codelet_sessions::thinking_options::options_for;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use common::MockBackend;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serde_json::{json, Value};
use tokio::sync::Mutex;

/// Serialises tests that set the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

fn sid() -> SessionId {
    SessionId::new("s-1")
}

// ───────────────────────── models ─────────────────────────────────────────

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

fn gemini_2_5_pro() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::Gemini,
        "google/gemini-2.5-pro",
        &[budget(Some(128), Some(32768))],
    )
}

fn haiku_4_5() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::Anthropic,
        "anthropic/claude-haiku-4-5",
        &[budget(Some(1024), None)],
    )
}

fn opus_5_5() -> Result<ThinkingTarget, TestError> {
    catalog(
        ThinkingWire::Anthropic,
        "anthropic/claude-opus-5-5",
        &[efforts(&["low", "medium", "high", "xhigh", "max"])],
    )
}

// ───────────────────────── dialog ─────────────────────────────────────────

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
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

// ───────────────────────── app ────────────────────────────────────────────

/// Root a throwaway process-global data directory whose user
/// `fspec-config.json` is `config`.
fn data_dir_with(config: &Value) -> Result<tempfile::TempDir, TestError> {
    let tmp = tempfile::tempdir()?;
    std::fs::write(tmp.path().join("fspec-config.json"), config.to_string())?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf())?;
    Ok(tmp)
}

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

/// The App's screen, one line per row.
fn screen(app: &mut App) -> Result<Vec<String>, TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 40)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    Ok((0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
        .collect())
}

/// The App's thinking dialog rows that carry the `(default)` marker.
async fn default_rows_on(target: &ThinkingTarget) -> Result<Vec<String>, TestError> {
    let (mut app, _mock) = app_for(target);
    drain(&mut app).await?;
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Thinking,
    ));
    drain(&mut app).await?;
    let rows = screen(&mut app)?;
    let labels = options_rows(target);
    assert!(
        rows.iter().any(|row| row.contains("Model default")),
        "the thinking dialog is open: {rows:#?}"
    );
    Ok(rows
        .into_iter()
        .filter(|row| row.contains("(default)"))
        .map(|row| {
            labels
                .iter()
                .find(|label| row.contains(&format!("{label} - ")))
                .cloned()
                .unwrap_or(row)
        })
        .collect())
}

fn options_rows(target: &ThinkingTarget) -> Vec<String> {
    ThinkingLevelDialog::new(sid(), ThinkingLevel::ModelDefault, options_for(target)).row_labels()
}

// ───────────────────────── scenarios ──────────────────────────────────────

/// Scenario: D on a numeric row saves the value the session holds
#[test]
fn d_on_a_numeric_row_saves_the_value_the_session_holds() -> TestResult {
    // @step Given the session's model is anthropic/claude-haiku-4-5 and its thinking level is a token budget of 8000
    let haiku = haiku_4_5()?;
    let mut dialog =
        ThinkingLevelDialog::new(sid(), ThinkingLevel::Budget(8000), options_for(&haiku));
    // @step When the user presses D on the token budget row
    select_row(&mut dialog, "Token budget 1024 or more")?;
    dialog.handle_event(&key(KeyCode::Char('d')));
    // @step Then the dialog asks to save a token budget of 8000
    let level = match dialog.take_pending_action() {
        Some(Action::SetThinkingLevelDefault(session, level)) if session == sid() => level,
        other => return Err(format!("expected SetThinkingLevelDefault, got {other:?}").into()),
    };
    assert_eq!(level, ThinkingLevel::Budget(8000));
    // @step And tui.thinkingLevels holds budget:8000 for anthropic/claude-haiku-4-5
    let tmp = tempfile::tempdir()?;
    save_default_thinking_level_with_dirs(tmp.path(), tmp.path(), haiku.model(), level)?;
    let config: Value = serde_json::from_str(&std::fs::read_to_string(
        tmp.path().join("fspec-config.json"),
    )?)?;
    assert_eq!(
        config["tui"]["thinkingLevels"]["anthropic/claude-haiku-4-5"],
        json!("budget:8000")
    );
    // @step Given the session's model is google/gemini-2.5-pro and its thinking level is not set
    let gemini = gemini_2_5_pro()?;
    let mut dialog =
        ThinkingLevelDialog::new(sid(), ThinkingLevel::ModelDefault, options_for(&gemini));
    // @step When the user presses D on the token budget row
    select_row(&mut dialog, "Token budget 128–32768")?;
    dialog.handle_event(&key(KeyCode::Char('d')));
    // @step Then D is refused with a notice to choose a token budget first and nothing is saved
    match dialog.take_pending_action() {
        Some(Action::ThinkingLevelRefused(_, reason)) => {
            assert!(reason.contains("choose a token budget first"), "{reason}");
        }
        other => return Err(format!("expected a refusal, got {other:?}").into()),
    }
    assert_eq!(dialog.default_index(), None, "the marker does not move");
    Ok(())
}

/// Scenario: The dialog marks only the current model's saved level
#[tokio::test]
async fn the_dialog_marks_only_the_current_models_saved_level() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given the saved thinking levels are none for openai/gpt-5.5 and xhigh for openai/gpt-5
    let _tmp = data_dir_with(&json!({ "tui": { "thinkingLevels": {
        "openai/gpt-5.5": "effort:none",
        "openai/gpt-5": "effort:xhigh",
    } } }))?;
    // @step When the user opens the thinking dialog on openai/gpt-5.5
    let marked = default_rows_on(&gpt_5_5()?).await?;
    // @step Then none carries the (default) marker
    assert_eq!(marked, ["none"]);
    // @step When the user opens the thinking dialog on openai/gpt-5
    let marked = default_rows_on(&gpt_5()?).await?;
    // @step Then no row carries the (default) marker
    assert_eq!(marked, Vec::<String>::new());
    // @step When the user opens the thinking dialog on google/gemini-2.5-pro
    let marked = default_rows_on(&gemini_2_5_pro()?).await?;
    // @step Then no row carries the (default) marker
    assert_eq!(marked, Vec::<String>::new());
    Ok(())
}

/// Scenario: The TUI applies no saved level itself
#[tokio::test]
async fn the_tui_applies_no_saved_level_itself() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given the saved thinking level for anthropic/claude-opus-5-5 is xhigh
    let _tmp = data_dir_with(&json!({ "tui": {
        "defaultThinkingLevel": 3,
        "thinkingLevels": { "anthropic/claude-opus-5-5": "effort:xhigh" },
    } }))?;
    // @step And the daemon's session on anthropic/claude-opus-5-5 is at high
    let (mut app, mock) = app_for(&opus_5_5()?);
    mock.set_thinking_level(ThinkingLevel::Effort("high".to_string()));
    // @step When the TUI starts with that session active and the session regains focus
    drain(&mut app).await?;
    app.dispatch(Action::SessionCreated(sid()));
    drain(&mut app).await?;
    // @step Then the TUI sends no set_thinking_level
    assert_eq!(mock.set_thinking_level_calls(), 0);
    // @step And the header shows high from get_thinking_level
    assert_eq!(
        app.agent_view_store().thinking_level_for(&sid()),
        Some(&ThinkingLevel::Effort("high".to_string()))
    );
    let header = screen(&mut app)?.into_iter().next().unwrap_or_default();
    assert!(header.contains("high"), "{header}");
    Ok(())
}
