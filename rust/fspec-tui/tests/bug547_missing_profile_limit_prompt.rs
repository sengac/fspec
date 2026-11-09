//! BUG-547 — the TUI asks for a local profile model's missing limit when the
//! model is selected, saves it to the profile and continues; cancelling
//! refuses the model; provider settings show a profile missing a limit.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature
//!
//! The daemon scenarios drive a real `codelet_sessions::SessionManager`
//! through the embedded transport against a temp `fspec-config.json`
//! (`FSPEC_USER_DIR`). The session-creation scenarios without the TUI are in
//! rust/sessions/tests/bug547_missing_profile_limit_resolved_at_selection.rs.

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::components::missing_profile_limit_dialog::MISSING_PROFILE_LIMIT_DIALOG_ID;
use codelet_fspec_tui::views::provider_settings::nav_item::{NavItemKind, ProviderDisplayInfo};
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{ProfileDefinition, ProviderProfileConfig, ProviderSettingsConfig};
use codelet_sessions::SessionManager;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

const STRATA_MODEL: &str = "qwen3.8-flash-next-iq3_s";
const MODELS_FIXTURE: &str = include_str!("../../sessions/tests/fixtures/prov101_models.json");

fn press(app: &mut App, code: KeyCode) {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

async fn drain(app: &mut App) -> TestResult {
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

fn scrollback_text(app: &App, session: &codelet_rpc_types::SessionId) -> String {
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

fn profiles() -> Value {
    json!({
        "Strata": {
            "baseUrl": "http://127.0.0.1:9/v1",
            "apiKey": "offline",
            "engine": "strata",
            "contextWindow": 200000,
            "customModels": [{"id": STRATA_MODEL}]
        },
        "Full": {
            "baseUrl": "http://127.0.0.1:9/v1",
            "apiKey": "offline",
            "contextWindow": 32768,
            "maxOutputTokens": 4096,
            "customModels": [{"id": "full"}]
        }
    })
}

/// A user directory with the profiles, a data directory with a models
/// cache and a workspace, with the environment restored on drop.
struct Daemon {
    user: TempDir,
    workspace: TempDir,
    manager: Arc<SessionManager>,
    backend: Arc<dyn FspecBackend>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        codelet_core::persistence::reset_stores_for_tests();
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

impl Daemon {
    fn new() -> Result<Self, codelet_test_helpers::TestError> {
        let saved = [
            "FSPEC_USER_DIR",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_MAX_OUTPUT_TOKENS",
            "OPENAI_STREAMING",
        ]
        .into_iter()
        .map(|key| (key, std::env::var(key).ok()))
        .collect();
        let user = codelet_test_helpers::setup_temp_directory()?;
        std::fs::write(
            user.path().join("fspec-config.json"),
            serde_json::to_string_pretty(
                &json!({"providers": {"openai": {"profiles": profiles()}}}),
            )?,
        )?;
        std::env::set_var("FSPEC_USER_DIR", user.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        let data = user.path().join("data");
        std::fs::create_dir_all(data.join("cache"))?;
        std::fs::write(data.join("cache/models.json"), MODELS_FIXTURE)?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        codelet_sessions::credentials::reset_credential_store();
        let workspace = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(workspace.path().join("spec"))?;
        std::fs::write(
            workspace.path().join("spec/work-units.json"),
            r#"{"workUnits":{}}"#,
        )?;
        let watcher = Arc::new(WorkUnitsWatcher::new(workspace.path())?);
        let manager = SessionManager::new();
        manager.set_default_project(workspace.path().to_path_buf())?;
        let handle: Arc<dyn SessionManagerHandle> = manager.clone();
        let service = Arc::new(
            SharedFspecService::with_session_manager(watcher, handle)
                .with_cwd(workspace.path().to_path_buf()),
        );
        let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
            tokio::runtime::Handle::current(),
            service,
        ));
        Ok(Self {
            user,
            workspace,
            manager,
            backend,
            saved,
        })
    }

    fn config(&self) -> Result<Value, codelet_test_helpers::TestError> {
        Ok(serde_json::from_slice(&std::fs::read(
            self.user.path().join("fspec-config.json"),
        )?)?)
    }

    fn config_path(&self) -> std::path::PathBuf {
        self.user.path().join("fspec-config.json")
    }

    fn workspace(&self) -> &Path {
        self.workspace.path()
    }
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_tui_asks_for_a_missing_limit_saves_it_and_continues() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given the Strata profile declares contextWindow 200000 and no maxOutputTokens
    let daemon = Daemon::new()?;
    let mut app = App::new(daemon.backend.clone());

    // @step When the user selects openai:Strata/qwen3.8-flash-next-iq3_s in the TUI and enters 10000 for Max Output Tokens
    app.dispatch(Action::ModelSelected(
        None,
        "openai:Strata".into(),
        STRATA_MODEL.into(),
    ));
    drain(&mut app).await?;
    assert!(
        app.compositor().contains(MISSING_PROFILE_LIMIT_DIALOG_ID),
        "the TUI asks for the missing limit"
    );
    assert!(
        daemon.manager.session_count().await == 0,
        "no session exists before the limit is given"
    );
    type_text(&mut app, "10000");
    press(&mut app, KeyCode::Enter);
    drain(&mut app).await?;

    // @step Then providers.openai.profiles.Strata.maxOutputTokens is 10000 in fspec-config.json
    let config = daemon.config()?;
    let strata = &config["providers"]["openai"]["profiles"]["Strata"];
    assert_eq!(strata["maxOutputTokens"], json!(10000));
    assert_eq!(strata["contextWindow"], json!(200000));
    assert_eq!(strata["engine"], json!("strata"));
    assert_eq!(strata["customModels"], json!([{"id": STRATA_MODEL}]));

    // @step And the session is created and the provider its first turn uses caps output at 10000
    assert!(!app.compositor().contains(MISSING_PROFILE_LIMIT_DIALOG_ID));
    let session = app
        .agent_view_store()
        .current_session()
        .cloned()
        .or_fail("the session is created")?;
    let model = daemon.manager.get_session_model(&session)?;
    assert_eq!(model.max_output_tokens, Some(10000));
    assert_eq!(model.context_window, Some(200000));
    let background = daemon.manager.get_session(&session.value)?;
    let inner = background.inner.lock().await;
    let provider_manager = inner.provider_manager();
    assert_eq!(provider_manager.max_output_tokens(), Some(10000));
    provider_manager.get_openai(uuid::Uuid::parse_str(&session.value)?)?;
    Ok(())
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_the_prompt_refuses_the_model() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given the Strata profile declares contextWindow 200000 and no maxOutputTokens
    let daemon = Daemon::new()?;
    daemon.manager.set_default_model("openai:Full/full")?;
    let session = SessionManagerHandle::create_session_checked(daemon.manager.as_ref(), None)?;
    let before = daemon.config()?;
    let mut app = App::new(daemon.backend.clone());
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::OpenAgentView(Some(session.clone())));
    drain(&mut app).await?;

    // @step When the user selects openai:Strata/qwen3.8-flash-next-iq3_s in the TUI and cancels the prompt
    app.dispatch(Action::ModelSelected(
        Some(session.clone()),
        "openai:Strata".into(),
        STRATA_MODEL.into(),
    ));
    drain(&mut app).await?;
    assert!(app.compositor().contains(MISSING_PROFILE_LIMIT_DIALOG_ID));
    press(&mut app, KeyCode::Esc);
    drain(&mut app).await?;

    // @step Then the selection is refused with the message naming maxOutputTokens, the Strata profile and where to set it
    assert!(!app.compositor().contains(MISSING_PROFILE_LIMIT_DIALOG_ID));
    let scrollback = scrollback_text(&app, &session);
    let expected = format!(
        "openai:Strata/{STRATA_MODEL} has no max output tokens: set maxOutputTokens on the Strata profile in {} (providers.openai.profiles.Strata) or in the TUI provider settings",
        daemon.config_path().display()
    );
    assert!(
        scrollback.replace('\n', " ").contains(&expected),
        "the refusal is shown:\n{scrollback}"
    );
    assert_eq!(
        daemon.manager.get_session_model(&session)?.model_id,
        "full",
        "the model does not change"
    );
    // @step And fspec-config.json is unchanged
    assert_eq!(daemon.config()?, before);
    let _ = daemon.workspace();
    Ok(())
}

#[test]
fn the_provider_settings_show_a_profile_missing_a_limit() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given the Strata profile declares no maxOutputTokens
    let strata = ProfileDefinition {
        base_url: "http://127.0.0.1:9/v1".into(),
        context_window: Some(200000),
        ..ProfileDefinition::default()
    };
    let full = ProfileDefinition {
        base_url: "http://127.0.0.1:9/v1".into(),
        context_window: Some(32768),
        max_output_tokens: Some(4096),
        ..ProfileDefinition::default()
    };
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![ProviderDisplayInfo {
        id: "openai".into(),
        name: "OpenAI".into(),
        configured: true,
        credential_type: "api-key".into(),
        ..ProviderDisplayInfo::default()
    }]);

    // @step When the TUI provider settings list the profiles
    view.set_provider_settings_configs(vec![ProviderSettingsConfig {
        provider_id: "openai".into(),
        settings: ProfileDefinition::default(),
        profiles: vec![
            ProviderProfileConfig {
                name: "Strata".into(),
                definition: strata,
            },
            ProviderProfileConfig {
                name: "Full".into(),
                definition: full,
            },
        ],
    }]);
    view.toggle_expansion("openai");

    // @step Then the Strata profile is shown as missing max output tokens
    let labels: Vec<String> = view
        .nav_items
        .iter()
        .filter_map(|item| match &item.kind {
            NavItemKind::Profile { profile_name } => Some(profile_name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        vec![
            "Full → http://127.0.0.1:9/v1".to_string(),
            "Strata → http://127.0.0.1:9/v1 — missing max output tokens".to_string(),
        ]
    );
    Ok(())
}
