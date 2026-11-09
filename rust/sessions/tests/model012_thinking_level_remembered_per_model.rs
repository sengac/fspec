//! MODEL-012: the thinking level is remembered per model.
//!
//! Feature: spec/features/thinking-level-remembered-per-model.feature
//!
//! The daemon scenarios drive a real `SessionManager` rooted at a throwaway
//! data directory whose models.dev cache is seeded with models that
//! advertise their reasoning options (anthropic/claude-opus-5-5,
//! openai/gpt-5.5, openai/gpt-5, google/gemini-2.5-pro). The saved levels
//! live in that directory's user `fspec-config.json` under
//! `tui.thinkingLevels`. The process-global data directory is serialised by
//! `DATA_DIR_GUARD`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::local_reasoning::{LocalEngine, LocalReasoning};
use codelet_providers::models::{NumericKind, NumericRange, ReasoningCapability};
use codelet_providers::thinking_wire::{ThinkingTarget, ThinkingWire};
use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk, ThinkingLevel};
use codelet_sessions::default_thinking_level_persistence::load_default_thinking_level_with_dirs;
use codelet_sessions::thinking_options::{saved_level_for, turn_level};
use codelet_sessions::SessionManager;
use codelet_test_helpers::{OrFail, OrFailErr, TestError, TestResult};
use serde_json::{json, Number, Value};

/// Offline models.dev catalog whose models carry `reasoning_options`.
const MODELS_FIXTURE: &str = include_str!("fixtures/model012_models.json");

/// Serialises tests that set the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

const OPUS: &str = "anthropic/claude-opus-5-5";
const GPT_5_5: &str = "openai/gpt-5.5";
const GPT_5: &str = "openai/gpt-5";
const GEMINI: &str = "google/gemini-2.5-pro";

fn effort(name: &str) -> ThinkingLevel {
    ThinkingLevel::Effort(name.to_string())
}

/// A session manager rooted at a fresh data directory.
struct Daemon {
    _guard: MutexGuard<'static, ()>,
    _tmp: tempfile::TempDir,
    dir: PathBuf,
    manager: Arc<SessionManager>,
}

impl Daemon {
    /// Root a daemon whose user `fspec-config.json` is `config` (none when
    /// `None`).
    fn start(config: Option<Value>) -> Result<Self, TestError> {
        let guard = DATA_DIR_GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
        std::env::set_var("OPENAI_API_KEY", "sk-test-dummy-key");
        std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("cache"))?;
        std::fs::write(dir.join("cache").join("models.json"), MODELS_FIXTURE)?;
        if let Some(config) = config {
            std::fs::write(dir.join("fspec-config.json"), config.to_string())?;
        }
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(dir.clone())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = SessionManager::new();
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace))?;
        Ok(Self {
            _guard: guard,
            _tmp: tmp,
            dir,
            manager,
        })
    }

    /// The daemon with these saved levels in `tui.thinkingLevels`.
    fn with_saved(levels: &[(&str, &str)]) -> Result<Self, TestError> {
        let map: serde_json::Map<String, Value> = levels
            .iter()
            .map(|(model, level)| ((*model).to_string(), json!(level)))
            .collect();
        Self::start(Some(json!({ "tui": { "thinkingLevels": map } })))
    }

    fn handle(&self) -> &dyn SessionManagerHandle {
        self.manager.as_ref()
    }

    /// A new session on `model` (`provider/model`).
    fn session_on(&self, model: &str) -> Result<SessionId, TestError> {
        self.handle().set_default_model(model)?;
        Ok(self.handle().create_session_checked(None)?)
    }

    fn level(&self, session: &SessionId) -> Result<ThinkingLevel, TestError> {
        Ok(self.handle().get_thinking_level(session)?)
    }

    fn user_config(&self) -> Result<Value, TestError> {
        let text = std::fs::read_to_string(self.dir.join("fspec-config.json"))?;
        Ok(serde_json::from_str(&text)?)
    }

    /// The saved entry for `model`, as stored.
    fn saved(&self, model: &str) -> Result<Option<Value>, TestError> {
        Ok(self.user_config()?["tui"]["thinkingLevels"]
            .get(model)
            .cloned())
    }

    /// The warning notices the session has shown.
    fn warnings(&self, session: &SessionId) -> Result<Vec<String>, TestError> {
        Ok(self
            .handle()
            .get_buffered_output(session, 1000)?
            .into_iter()
            .filter_map(|chunk| match chunk {
                StreamChunk::UserNotification {
                    message,
                    severity: NotificationSeverity::Warning,
                } => Some(message),
                _ => None,
            })
            .collect())
    }

    /// Every notice the session has shown.
    fn notices(&self, session: &SessionId) -> Result<Vec<String>, TestError> {
        Ok(self
            .handle()
            .get_buffered_output(session, 1000)?
            .into_iter()
            .filter_map(|chunk| match chunk {
                StreamChunk::UserNotification { message, .. } => Some(message),
                _ => None,
            })
            .collect())
    }
}

fn split(model: &str) -> Result<(&str, &str), TestError> {
    Ok(model.split_once('/').or_fail("a provider/model name")?)
}

/// Scenario: D saves the level for the current model only
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d_saves_the_level_for_the_current_model_only() -> TestResult {
    // @step Given the saved thinking level for openai/gpt-5.5 is none
    let daemon = Daemon::with_saved(&[(GPT_5_5, "effort:none")])?;
    // @step And the session's model is anthropic/claude-opus-5-5
    let session = daemon.session_on(OPUS)?;
    // @step When the user saves xhigh with D
    daemon
        .handle()
        .set_thinking_level_default(&session, effort("xhigh"))?;
    // @step Then tui.thinkingLevels holds effort:xhigh for anthropic/claude-opus-5-5 and still holds effort:none for openai/gpt-5.5
    assert_eq!(daemon.saved(OPUS)?, Some(json!("effort:xhigh")));
    assert_eq!(daemon.saved(GPT_5_5)?, Some(json!("effort:none")));
    // @step And a new session on anthropic/claude-opus-5-5 starts at xhigh
    let next = daemon.session_on(OPUS)?;
    assert_eq!(daemon.level(&next)?, effort("xhigh"));
    Ok(())
}

/// Scenario: A model with no saved level starts with no level set
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_model_with_no_saved_level_starts_with_no_level_set() -> TestResult {
    // @step Given the saved thinking levels are xhigh for anthropic/claude-opus-5-5 and none for openai/gpt-5.5
    let daemon = Daemon::with_saved(&[(OPUS, "effort:xhigh"), (GPT_5_5, "effort:none")])?;
    // @step When a new session starts on openai/gpt-5.5
    let gpt = daemon.session_on(GPT_5_5)?;
    // @step Then its thinking level is none
    assert_eq!(daemon.level(&gpt)?, effort("none"));
    // @step When a new session starts on google/gemini-2.5-pro
    let gemini = daemon.session_on(GEMINI)?;
    // @step Then its thinking level is not set
    assert_eq!(daemon.level(&gemini)?, ThinkingLevel::ModelDefault);
    assert_eq!(daemon.warnings(&gemini)?, Vec::<String>::new());
    Ok(())
}

/// Scenario: D on Model default removes only the current model's saved level
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d_on_model_default_removes_only_the_current_models_saved_level() -> TestResult {
    // @step Given the saved thinking levels are none for openai/gpt-5.5 and xhigh for anthropic/claude-opus-5-5
    let daemon = Daemon::with_saved(&[(GPT_5_5, "effort:none"), (OPUS, "effort:xhigh")])?;
    // @step And the session's model is openai/gpt-5.5
    let session = daemon.session_on(GPT_5_5)?;
    // @step When the user saves Model default with D
    daemon
        .handle()
        .set_thinking_level_default(&session, ThinkingLevel::ModelDefault)?;
    // @step Then tui.thinkingLevels has no entry for openai/gpt-5.5
    assert_eq!(daemon.saved(GPT_5_5)?, None);
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    // @step And tui.thinkingLevels still holds effort:xhigh for anthropic/claude-opus-5-5
    assert_eq!(daemon.saved(OPUS)?, Some(json!("effort:xhigh")));
    Ok(())
}

/// Scenario: Saving a level the model does not offer is refused
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_level_the_model_does_not_offer_is_refused() -> TestResult {
    // @step Given the session's model is openai/gpt-5
    let daemon = Daemon::with_saved(&[])?;
    let session = daemon.session_on(GPT_5)?;
    // @step When set_thinking_level_default is called with xhigh
    let refusal = daemon
        .handle()
        .set_thinking_level_default(&session, effort("xhigh"))
        .or_fail_err("xhigh is refused on openai/gpt-5")?;
    // @step Then it is refused naming openai/gpt-5 and listing minimal, low, medium, high
    assert!(
        refusal.contains(GPT_5) && refusal.contains("minimal, low, medium, high"),
        "{refusal}"
    );
    // @step And tui.thinkingLevels has no entry for openai/gpt-5
    assert_eq!(daemon.saved(GPT_5)?, None);
    Ok(())
}

/// Scenario: A saved level the model does not offer leaves the level not set and tells the user why
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_level_the_model_does_not_offer_is_not_set_and_explained() -> TestResult {
    // @step Given the saved thinking level for openai/gpt-5 is xhigh
    let daemon = Daemon::with_saved(&[(GPT_5, "effort:xhigh")])?;
    // @step When a new session starts on openai/gpt-5
    let session = daemon.session_on(GPT_5)?;
    // @step Then its thinking level is not set
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    // @step And the session warns that the saved thinking level xhigh for openai/gpt-5 is not offered, listing minimal, low, medium, high
    let warnings = daemon.warnings(&session)?.join("\n");
    assert!(
        warnings.contains("saved thinking level xhigh for openai/gpt-5 is not offered")
            && warnings.contains("minimal, low, medium, high"),
        "{warnings}"
    );
    // @step And tui.thinkingLevels still holds effort:xhigh for openai/gpt-5
    assert_eq!(daemon.saved(GPT_5)?, Some(json!("effort:xhigh")));
    Ok(())
}

/// Scenario: A saved level that cannot be read is reported and never loaded as Off
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_level_that_cannot_be_read_is_reported() -> TestResult {
    // @step Given tui.thinkingLevels holds the bare number 3 for openai/gpt-5
    let daemon = Daemon::start(Some(json!({ "tui": { "thinkingLevels": { GPT_5: 3 } } })))?;
    // @step When a new session starts on openai/gpt-5
    let session = daemon.session_on(GPT_5)?;
    // @step Then its thinking level is not set
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    // @step And the session warns that the saved thinking level for openai/gpt-5 in tui.thinkingLevels cannot be read
    let warnings = daemon.warnings(&session)?.join("\n");
    assert!(
        warnings
            .contains("saved thinking level for openai/gpt-5 in tui.thinkingLevels cannot be read"),
        "{warnings}"
    );
    Ok(())
}

/// Scenario: The old tui.defaultThinkingLevel is ignored and left in the file
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_old_default_thinking_level_is_ignored_and_left_in_the_file() -> TestResult {
    // @step Given the user config has tui.defaultThinkingLevel 3 and no tui.thinkingLevels
    let daemon = Daemon::start(Some(json!({ "tui": { "defaultThinkingLevel": 3 } })))?;
    // @step When a new session starts on anthropic/claude-opus-5-5
    let session = daemon.session_on(OPUS)?;
    // @step Then its thinking level is not set and the session shows no notice
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    assert_eq!(daemon.notices(&session)?, Vec::<String>::new());
    // @step When the user saves xhigh with D on anthropic/claude-opus-5-5
    daemon
        .handle()
        .set_thinking_level_default(&session, effort("xhigh"))?;
    // @step Then the user config still has tui.defaultThinkingLevel 3
    let config = daemon.user_config()?;
    assert_eq!(config["tui"]["defaultThinkingLevel"], json!(3));
    assert_eq!(config["tui"]["thinkingLevels"][OPUS], json!("effort:xhigh"));
    Ok(())
}

/// The Strata profile model DeepSeek-V4.1 declaring whole-number efforts
/// 1 to 100 (MODEL-016).
fn strata_deepseek() -> Result<ThinkingTarget, TestError> {
    let numeric = NumericRange::new(NumericKind::Whole, Number::from(1), Number::from(100))?;
    Ok(ThinkingTarget::local_profile(
        Ok(ThinkingWire::LocalOpenAi),
        "openai:strata/DeepSeek-V4.1",
        "DeepSeek-V4.1",
        Ok(Some(LocalReasoning {
            profile: "strata".to_string(),
            engine: LocalEngine::Strata,
            capability: ReasoningCapability::declared(&[], Some(numeric), None, None),
        })),
    ))
}

fn write_config(dir: &Path, config: &Value) -> TestResult {
    std::fs::write(dir.join("fspec-config.json"), config.to_string())?;
    Ok(())
}

/// Scenario: A profile model's numeric effort is remembered under its profile-qualified name
#[test]
fn a_profile_models_numeric_effort_is_remembered_under_its_profile_name() -> TestResult {
    // @step Given the saved thinking level for openai:strata/DeepSeek-V4.1 is the numeric effort 40
    let tmp = tempfile::tempdir()?;
    write_config(
        tmp.path(),
        &json!({ "tui": { "thinkingLevels": { "openai:strata/DeepSeek-V4.1": "numeric:40" } } }),
    )?;
    // @step And the Strata profile model DeepSeek-V4.1 declares whole-number efforts 1 to 100
    let target = strata_deepseek()?;
    // @step When its saved level is applied
    let saved = load_default_thinking_level_with_dirs(tmp.path(), tmp.path(), target.model());
    let applied = saved_level_for(&target, saved);
    // @step Then its thinking level is the numeric effort 40
    assert_eq!(applied.level, ThinkingLevel::Numeric(Number::from(40)));
    assert_eq!(applied.notice, None);
    // @step And its turn sends reasoning_effort 40
    let turn = turn_level(&applied.level)?;
    let sent = target
        .config_for(turn.as_ref())?
        .or_fail("the turn sends a thinking parameter")?;
    assert_eq!(sent["reasoning_effort"], json!(40), "{sent}");
    Ok(())
}

/// Scenario: Switching model takes the new model's saved level
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_model_takes_the_new_models_saved_level() -> TestResult {
    // @step Given the session on anthropic/claude-opus-5-5 is at xhigh
    // @step And the saved thinking levels are none for openai/gpt-5.5 and xhigh for openai/gpt-5
    let daemon = Daemon::with_saved(&[(GPT_5_5, "effort:none"), (GPT_5, "effort:xhigh")])?;
    let session = daemon.session_on(OPUS)?;
    daemon
        .handle()
        .set_thinking_level(&session, effort("xhigh"))?;
    assert_eq!(daemon.level(&session)?, effort("xhigh"));
    // @step When the user switches the session to openai/gpt-5.5
    let (provider, model) = split(GPT_5_5)?;
    daemon.handle().set_model(&session, provider, model)?;
    // @step Then its thinking level is none
    assert_eq!(daemon.level(&session)?, effort("none"));
    // @step When the user switches the session to openai/gpt-5
    let (provider, model) = split(GPT_5)?;
    daemon.handle().set_model(&session, provider, model)?;
    // @step Then its thinking level is not set and the session warns that the saved thinking level xhigh for openai/gpt-5 is not offered
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    let warnings = daemon.warnings(&session)?.join("\n");
    assert!(
        warnings.contains("saved thinking level xhigh for openai/gpt-5 is not offered"),
        "{warnings}"
    );
    // @step When the user switches the session to google/gemini-2.5-pro
    daemon
        .handle()
        .set_thinking_level(&session, effort("low"))?;
    let (provider, model) = split(GEMINI)?;
    daemon.handle().set_model(&session, provider, model)?;
    // @step Then its thinking level is not set
    assert_eq!(daemon.level(&session)?, ThinkingLevel::ModelDefault);
    Ok(())
}

/// Scenario: Choosing a level does not save it
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn choosing_a_level_does_not_save_it() -> TestResult {
    // @step Given the session's model is anthropic/claude-opus-5-5 with no saved level
    let daemon = Daemon::with_saved(&[])?;
    let session = daemon.session_on(OPUS)?;
    // @step When the user chooses high with /thinking
    daemon
        .handle()
        .set_thinking_level(&session, effort("high"))?;
    // @step Then the session's thinking level is high
    assert_eq!(daemon.level(&session)?, effort("high"));
    // @step And tui.thinkingLevels has no entry for anthropic/claude-opus-5-5
    assert_eq!(daemon.saved(OPUS)?, None);
    Ok(())
}
