//! Feature: spec/features/custom-model-writes-under-an-openai-profile-section.feature
//!
//! BUG-325 — custom-model writes from an OpenAI profile section must name the
//! base provider ("openai"), not the daemon's section key
//! ("openai:<profile>"), and a write the daemon cannot apply must reach the
//! user. Sections are built with the daemon's own builder
//! (`codelet_sessions::profile_sections::build_profile_provider_info`) so the
//! fixture shape cannot drift from what `list_providers` sends. The daemon
//! scenarios drive a real `codelet_sessions::SessionManager` through the
//! embedded transport against a temp `fspec-config.json` (`FSPEC_USER_DIR`).

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::views::{ModelSelectorEvent, ModelSelectorView};
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId};
use codelet_sessions::profile_sections::build_profile_provider_info;
use codelet_sessions::SessionManager;
use common::MockBackend;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn custom(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(128_000),
        is_custom: true,
        ..ModelEntry::default()
    }
}

/// The section exactly as the daemon lists it for profile `my-profile`.
fn daemon_section(models: Vec<ModelEntry>) -> ProviderInfo {
    build_profile_provider_info("openai", "my-profile", models, false)
}

fn selector(section: ProviderInfo) -> ModelSelectorView {
    let mut view = ModelSelectorView::new();
    view.set_session(Some(SessionId::new("s-1")));
    view.set_providers(vec![section]);
    view
}

fn emitted(event: ModelSelectorEvent) -> Result<Action, codelet_test_helpers::TestError> {
    Ok(match event {
        ModelSelectorEvent::Emit(action) => *action,
        other => return Err(format!("expected an emitted write action, got {other:?}").into()),
    })
}

/// Press `a` on the focused section header, type `id`, submit.
fn add_from_header(
    view: &mut ModelSelectorView,
    id: &str,
) -> Result<Action, codelet_test_helpers::TestError> {
    view.handle_key(key(KeyCode::Char('a')));
    for c in id.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    emitted(view.handle_key(key(KeyCode::Enter)))
}

fn seed_config(dir: &Path, profile: Value) -> Result<(), codelet_test_helpers::TestError> {
    let root = json!({"providers": {"openai": {"profiles": {"my-profile": profile}}}});
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&root).or_fail("expected a value")?,
    )
    .or_fail("expected a value")?;
    std::env::set_var("FSPEC_USER_DIR", dir);
    Ok(())
}

fn custom_model_ids(dir: &Path) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let root: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("fspec-config.json")).or_fail("expected a value")?,
    )
    .or_fail("expected a value")?;
    Ok(
        root["providers"]["openai"]["profiles"]["my-profile"]["customModels"]
            .as_array()
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| model["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// The embedded transport over a real daemon `SessionManager`.
fn daemon_backend(
    workspace: &Path,
) -> Result<Arc<dyn FspecBackend>, codelet_test_helpers::TestError> {
    std::fs::create_dir_all(workspace.join("spec")).or_fail("expected a value")?;
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("expected a value")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).or_fail("expected a value")?);
    let handle: Arc<dyn SessionManagerHandle> = SessionManager::new();
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, handle).with_cwd(workspace.to_path_buf()),
    );
    Ok(Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    )))
}

// Scenario: Custom-model writes from a profile section name the base provider
#[test]
fn scenario_custom_model_writes_name_the_base_provider() -> TestResult {
    // @step Given the daemon lists the OpenAI profile section for "my-profile" with the custom model "mycustom"
    let mut view = selector(daemon_section(vec![custom("mycustom")]));

    // @step When I add, edit and delete a custom model from that section in the model selector
    let added = add_from_header(&mut view, "newmodel")?;
    view.handle_key(key(KeyCode::Right));
    view.handle_key(key(KeyCode::Down));
    view.handle_key(key(KeyCode::Char('e')));
    let edited = emitted(view.handle_key(key(KeyCode::Enter)))?;
    view.handle_key(key(KeyCode::Char('d')));
    let deleted = emitted(view.handle_key(key(KeyCode::Char('y'))))?;

    // @step Then each write is sent with provider id "openai" and profile "my-profile"
    let targets: Vec<(String, String)> = [added, edited, deleted]
        .into_iter()
        .map(|action| match action {
            Action::AddCustomModel {
                provider_id,
                profile_name,
                ..
            }
            | Action::EditCustomModel {
                provider_id,
                profile_name,
                ..
            }
            | Action::DeleteCustomModel {
                provider_id,
                profile_name,
                ..
            } => Ok((provider_id, profile_name)),
            other => Err(format!("expected a custom-model write, got {other:?}")),
        })
        .collect::<Result<_, String>>()?;
    let expected = ("openai".to_string(), "my-profile".to_string());
    assert_eq!(targets, vec![expected.clone(), expected.clone(), expected]);
    Ok(())
}

// Scenario: A custom model added from a profile section is saved to that profile
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_added_custom_model_is_saved_to_the_profile() -> TestResult {
    let user_dir = TempDir::new().or_fail("expected a value")?;
    let workspace = TempDir::new().or_fail("expected a value")?;
    // @step Given an OpenAI profile "my-profile" in the user config
    seed_config(user_dir.path(), json!({"baseUrl": "http://127.0.0.1:9"}))?;
    let backend = daemon_backend(workspace.path())?;

    // @step And the model selector shows the daemon's section for that profile
    let mut view = selector(daemon_section(Vec::new()));

    // @step When I add the custom model "newmodel" from that section
    let Action::AddCustomModel {
        provider_id,
        profile_name,
        definition,
    } = add_from_header(&mut view, "newmodel")?
    else {
        return Err("expected AddCustomModel".into());
    };
    let result = backend
        .add_custom_model(provider_id, profile_name, definition)
        .await;

    // @step Then the profile "my-profile" lists the custom model "newmodel"
    assert!(result.is_ok(), "add must reach the profile: {result:?}");
    assert_eq!(custom_model_ids(user_dir.path())?, vec!["newmodel"]);
    Ok(())
}

// Scenario: Deleting a custom model with a section key as provider id is refused
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_delete_with_section_key_is_refused() -> TestResult {
    let user_dir = TempDir::new().or_fail("expected a value")?;
    let workspace = TempDir::new().or_fail("expected a value")?;
    // @step Given an OpenAI profile "my-profile" with the custom model "mycustom"
    seed_config(
        user_dir.path(),
        json!({"baseUrl": "http://127.0.0.1:9", "customModels": [{"id": "mycustom"}]}),
    )?;
    let backend = daemon_backend(workspace.path())?;

    // @step When a client calls delete_custom_model with provider id "openai:my-profile"
    let result = backend
        .delete_custom_model(
            "openai:my-profile".into(),
            "my-profile".into(),
            "mycustom".into(),
        )
        .await;

    // @step Then the call returns an error naming the provider id "openai:my-profile"
    let error = result
        .or_fail_err("a section key is not a provider id")?
        .to_string();
    assert!(error.contains("openai:my-profile"), "error: {error}");
    // @step And the profile "my-profile" still lists the custom model "mycustom"
    assert_eq!(custom_model_ids(user_dir.path())?, vec!["mycustom"]);
    Ok(())
}

fn scrollback_text(app: &App, session: &SessionId) -> String {
    app.agent_view_store()
        .session_context_for(session)
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
        .collect::<Vec<_>>()
        .join("\n")
}

fn selector_screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    app.navigator_mut().model_selector.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn drain(app: &mut App) -> codelet_test_helpers::TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    Ok(())
}

// Scenario: A failed custom-model write is shown to the user
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_failed_write_is_shown_to_the_user() -> TestResult {
    let session = SessionId::new("s-1");
    let mock = Arc::new(MockBackend::new());
    let section = daemon_section(vec![custom("mycustom")]);
    mock.seed_providers(vec![section.clone()]);
    mock.fail_custom_model_writes("custom model 'missing' not found in profile 'my-profile'");
    let mut app = App::new(mock.clone());
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::OpenAgentView(Some(session.clone())));
    drain(&mut app).await?;
    // @step Given the model selector is open for a session on the OpenAI profile section for "my-profile"
    app.dispatch(Action::OpenModelSelectorView);
    drain(&mut app).await?;

    // @step When deleting the custom model "missing" fails because the profile does not have it
    app.dispatch(Action::DeleteCustomModel {
        provider_id: "openai".into(),
        profile_name: "my-profile".into(),
        model_id: "missing".into(),
    });
    drain(&mut app).await?;

    // @step Then the model selector shows the error
    let screen = selector_screen(&mut app);
    assert!(
        screen.contains("custom model 'missing' not found"),
        "model selector must show the failure:\n{screen}"
    );
    // @step And the session's scrollback shows the error
    let scrollback = scrollback_text(&app, &session);
    assert!(
        scrollback.contains("custom model 'missing' not found"),
        "scrollback must show the failure:\n{scrollback}"
    );
    Ok(())
}
