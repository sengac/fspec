//! MODEL-014: Codex thinking levels come from the Codex /models endpoint.
//!
//! Feature: spec/features/codex-thinking-levels-from-codex-models.feature
//!
//! These scenarios drive a real `SessionManager` rooted at a throwaway data
//! directory. Its models.dev cache is the MODEL-012 fixture. Its Codex home
//! holds ChatGPT tokens (or only an API key). A wiremock server answers the
//! Codex `/models` request: the test-support redirect points the Codex
//! backend base at it, and the path and query are kept. The responses are
//! cut from the live capture in spec/attachments/MODEL-014. A session is
//! created, resumed or switched exactly as the daemon does it, so the fetch,
//! the thinking options and the provider snapshot all come from production
//! code. Tests share process-global state (data directory, CODEX_HOME,
//! redirects), so `GUARD` serialises them.

use std::sync::Arc;

use tokio::sync::{Mutex, MutexGuard};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::models_catalog::{self, CODEX_MODELS_CLIENT_VERSION};
use codelet_providers::codex::oauth_test_support::{self, OAuthTestConfig, OAuthTestGuard};
use codelet_providers::codex::request_options::RequestOptionsError;
use codelet_providers::models::ReasoningLevel;
use codelet_providers::thinking_wire::{ThinkingTarget, TurnLevel};
use codelet_rpc_types::{
    NotificationSeverity, SessionId, StreamChunk, ThinkingChoice, ThinkingLevel, ThinkingOptions,
};
use codelet_sessions::SessionManager;
use codelet_test_helpers::{OrFail, OrFailErr, TestError, TestResult};
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Offline models.dev catalog (no network while the registry loads).
const MODELS_DEV_FIXTURE: &str = include_str!("fixtures/model012_models.json");
/// A Codex /models response cut from the 2026-11-05 live capture.
const CODEX_MODELS: &str = include_str!("fixtures/model014_codex_models.json");

static GUARD: Mutex<()> = Mutex::const_new(());

const LUNA: &str = "codex/gpt-6-luna";
const SOL: &str = "codex/gpt-6-sol";
const SOL_61: &str = "codex/gpt-6.1-sol";
const ACCOUNT: &str = "acc_model014";
const ACCESS: &str = "access-acc_model014";
const ROTATED_ACCESS: &str = "access-acc_model014-rotated";
const LUNA_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// How Codex is signed in.
enum Auth {
    /// ChatGPT tokens; `expired` makes the stored access token expired.
    ChatGpt { expired: bool },
    /// Only an OpenAI API key, no ChatGPT tokens.
    ApiKeyOnly,
}

/// A session manager with a Codex home and a mocked Codex backend.
///
/// Fields drop in order, so the redirects are cleared before the guard lets
/// the next test set its own.
struct Codex {
    manager: Arc<SessionManager>,
    backend: MockServer,
    issuer: MockServer,
    _redirect: models_catalog::test_support::Redirect,
    _oauth: OAuthTestGuard,
    _tmp: tempfile::TempDir,
    _guard: MutexGuard<'static, ()>,
}

/// The models.dev fixture with the Codex /models slugs added to the OpenAI
/// catalog (copies of its gpt-5.5 entry): PROV-176 refuses a Codex model
/// whose limits models.dev does not know.
fn models_dev() -> Result<String, TestError> {
    let mut catalog: Value = serde_json::from_str(MODELS_DEV_FIXTURE)?;
    let openai = catalog["openai"]["models"]
        .as_object_mut()
        .or_fail("the fixture's OpenAI models")?;
    let template = openai.get("gpt-5.5").cloned().or_fail("gpt-5.5")?;
    for id in [
        "gpt-6.1-sol",
        "gpt-6-sol",
        "gpt-6-luna",
        "codex-auto-review",
    ] {
        let mut model = template.clone();
        model["id"] = json!(id);
        openai.insert(id.to_string(), model);
    }
    Ok(catalog.to_string())
}

fn codex_models() -> Result<Value, TestError> {
    Ok(serde_json::from_str(CODEX_MODELS)?)
}

fn ok(body: &Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

impl Codex {
    async fn start(
        auth: Auth,
        models: ResponseTemplate,
        config: Option<Value>,
    ) -> Result<Self, TestError> {
        let guard = GUARD.lock().await;
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("data");
        std::fs::create_dir_all(dir.join("cache"))?;
        std::fs::write(dir.join("cache").join("models.json"), models_dev()?)?;
        if let Some(config) = config {
            std::fs::write(dir.join("fspec-config.json"), config.to_string())?;
        }
        let codex_home = tmp.path().join("codex-home");
        std::fs::create_dir_all(&codex_home)?;
        let document = match auth {
            Auth::ChatGpt { expired } => {
                let refreshed = if expired {
                    chrono::Utc::now() - chrono::Duration::hours(3)
                } else {
                    chrono::Utc::now()
                };
                json!({
                    "tokens": {
                        "id_token": format!("id-{ACCOUNT}"),
                        "access_token": ACCESS,
                        "refresh_token": format!("refresh-{ACCOUNT}"),
                        "account_id": ACCOUNT
                    },
                    "last_refresh": refreshed.to_rfc3339()
                })
            }
            Auth::ApiKeyOnly => json!({ "OPENAI_API_KEY": "sk-model014-api-key" }),
        };
        std::fs::write(codex_home.join("auth.json"), document.to_string())?;
        std::env::set_var("CODEX_HOME", &codex_home);
        std::env::remove_var("CODEX_MODEL");

        let backend = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(models)
            .mount(&backend)
            .await;
        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": ROTATED_ACCESS,
                "refresh_token": format!("refresh-{ACCOUNT}-rotated"),
                "id_token": "opaque-id",
                "expires_in": 3600
            })))
            .mount(&issuer)
            .await;
        let redirect = models_catalog::test_support::redirect(&backend.uri());
        let oauth = oauth_test_support::configure(OAuthTestConfig {
            issuer: issuer.uri(),
            callback_address: "127.0.0.1:0".parse()?,
            state: "model014".into(),
        });

        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(dir.clone())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = SessionManager::new();
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace))?;
        Ok(Self {
            manager,
            backend,
            issuer,
            _redirect: redirect,
            _oauth: oauth,
            _tmp: tmp,
            _guard: guard,
        })
    }

    /// Signed in with ChatGPT, /models answering with the capture.
    async fn signed_in() -> Result<Self, TestError> {
        Self::start(Auth::ChatGpt { expired: false }, ok(&codex_models()?), None).await
    }

    fn handle(&self) -> &dyn SessionManagerHandle {
        self.manager.as_ref()
    }

    /// A new session on `model` (`codex/<slug>`).
    fn session_on(&self, model: &str) -> Result<SessionId, TestError> {
        self.handle().set_default_model(model)?;
        Ok(self.handle().create_session_checked(None)?)
    }

    /// Switch `session` to `model` (`codex/<slug>`).
    fn switch(&self, session: &SessionId, model: &str) -> Result<(), TestError> {
        let (provider, id) = model.split_once('/').or_fail("a provider/model name")?;
        Ok(self.handle().set_model(session, provider, id)?)
    }

    fn options(&self, session: &SessionId) -> Result<ThinkingOptions, TestError> {
        Ok(self.handle().get_thinking_options(session)?)
    }

    /// The efforts the session's model offers, in order.
    fn efforts(&self, session: &SessionId) -> Result<Vec<String>, TestError> {
        match self.options(session)? {
            ThinkingOptions::Levels { choices, .. } => Ok(choices
                .into_iter()
                .filter_map(|choice| match choice {
                    ThinkingChoice::Effort(name) => Some(name),
                    _ => None,
                })
                .collect()),
            other => Err(format!("expected levels, got {other:?}").into()),
        }
    }

    /// Why the session's model has no reasoning control.
    fn no_control(&self, session: &SessionId) -> Result<String, TestError> {
        match self.options(session)? {
            ThinkingOptions::NoReasoningControl { reason, .. } => Ok(reason),
            other => Err(format!("expected no reasoning control, got {other:?}").into()),
        }
    }

    fn choose(&self, session: &SessionId, level: &str) -> Result<(), String> {
        self.handle()
            .set_thinking_level(session, ThinkingLevel::Effort(level.to_string()))
    }

    fn level(&self, session: &SessionId) -> Result<ThinkingLevel, TestError> {
        Ok(self.handle().get_thinking_level(session)?)
    }

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

    /// The /models requests the backend received.
    async fn models_requests(&self) -> Result<Vec<Request>, TestError> {
        Ok(self
            .backend
            .received_requests()
            .await
            .or_fail("backend recording")?
            .into_iter()
            .filter(|request| request.url.path() == "/models")
            .collect())
    }

    /// Answer /models with `template` from now on.
    async fn respond(&self, template: ResponseTemplate) {
        self.backend.reset().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(template)
            .mount(&self.backend)
            .await;
    }

    /// The session's thinking target, as a turn resolves it.
    async fn target(&self, session: &SessionId) -> Result<ThinkingTarget, TestError> {
        let background = self.manager.get_session(&session.value)?;
        let inner = background.inner.lock().await;
        Ok(ThinkingTarget::for_provider_manager(
            inner.provider_manager(),
        ))
    }

    /// The wire body the session's Codex provider prepares for a request
    /// carrying `options`, built by the session's own provider manager.
    async fn prepared(
        &self,
        session: &SessionId,
        options: Option<Value>,
    ) -> Result<Result<Value, CompletionError>, TestError> {
        let background = self.manager.get_session(&session.value)?;
        let inner = background.inner.lock().await;
        let model = inner.provider_manager().get_codex()?.completion_model();
        let mut request = CompletionRequestBuilder::new(model.clone(), "Hello");
        if let Some(options) = options {
            request = request.additional_params(options);
        }
        Ok(model
            .prepare_request(request.build())
            .and_then(|bytes| Ok(serde_json::from_slice::<Value>(&bytes)?)))
    }
}

fn effort(name: &str) -> Result<TurnLevel, TestError> {
    Ok(TurnLevel::Level(ReasoningLevel::effort(name)?))
}

/// Scenario: The thinking dialog offers the levels Codex /models advertises
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_dialog_offers_the_levels_codex_models_advertises() -> TestResult {
    // @step Given Codex /models lists gpt-6-luna with levels low, medium, high, xhigh, max and default medium
    let codex = Codex::signed_in().await?;
    // @step When a session starts on codex/gpt-6-luna
    let session = codex.session_on(LUNA)?;
    // @step Then the thinking options are low, medium, high, xhigh, max in that order
    assert_eq!(codex.efforts(&session)?, LUNA_LEVELS);
    // @step And none is not offered
    assert!(!codex.efforts(&session)?.contains(&"none".to_string()));
    assert!(matches!(
        codex.options(&session)?,
        ThinkingOptions::Levels {
            takes_off: false,
            ..
        }
    ));
    Ok(())
}

/// Scenario: A level Codex does not advertise is refused and an advertised one is sent unchanged
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unadvertised_level_is_refused_and_an_advertised_one_is_sent() -> TestResult {
    // @step Given a session on codex/gpt-6-luna whose /models entry lists low, medium, high, xhigh, max
    let codex = Codex::signed_in().await?;
    let session = codex.session_on(LUNA)?;
    // @step When the user chooses none
    let refusal = codex
        .choose(&session, "none")
        .or_fail_err("none is refused")?;
    // @step Then it is refused naming codex/gpt-6-luna and listing low, medium, high, xhigh, max
    assert!(
        refusal.contains(LUNA) && refusal.contains("low, medium, high, xhigh, max"),
        "{refusal}"
    );
    // @step When the user chooses xhigh
    codex.choose(&session, "xhigh")?;
    assert_eq!(
        codex.level(&session)?,
        ThinkingLevel::Effort("xhigh".into())
    );
    let config = codex
        .target(&session)
        .await?
        .config_for(Some(&effort("xhigh")?))?
        .or_fail("xhigh sends reasoning")?;
    // @step Then the Codex request carries reasoning effort xhigh
    let body = codex.prepared(&session, Some(config)).await??;
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    Ok(())
}

/// Scenario: Ultra is not offered on Codex
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ultra_is_not_offered_on_codex() -> TestResult {
    // @step Given Codex /models lists gpt-6.1-sol with levels low, medium, high, xhigh, max, ultra
    let codex = Codex::signed_in().await?;
    // @step When a session starts on codex/gpt-6.1-sol
    let session = codex.session_on(SOL_61)?;
    // @step Then the thinking options are low, medium, high, xhigh, max in that order
    assert_eq!(codex.efforts(&session)?, LUNA_LEVELS);
    // @step And choosing ultra is refused naming codex/gpt-6.1-sol and listing low, medium, high, xhigh, max
    let refusal = codex
        .choose(&session, "ultra")
        .or_fail_err("ultra is refused")?;
    assert!(
        refusal.contains(SOL_61) && refusal.contains("low, medium, high, xhigh, max"),
        "{refusal}"
    );
    let snapshot = codex
        .prepared(&session, Some(json!({"reasoning": {"effort": "ultra"}})))
        .await?;
    assert!(
        matches!(&snapshot, Err(CompletionError::RequestError(source))
            if source.downcast_ref::<RequestOptionsError>().is_some()),
        "{snapshot:?}"
    );
    Ok(())
}

/// Scenario: A Codex model that /models does not list has no reasoning control
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_model_models_does_not_list_has_no_reasoning_control() -> TestResult {
    // @step Given the Codex /models response does not list gpt-5.5
    let codex = Codex::signed_in().await?;
    // @step When a session starts on codex/gpt-5.5
    let session = codex.session_on("codex/gpt-5.5")?;
    // @step Then it has no reasoning control because Codex /models does not list gpt-5.5
    let reason = codex.no_control(&session)?;
    assert!(
        reason.contains("Codex /models does not list gpt-5.5"),
        "{reason}"
    );
    assert!(codex.choose(&session, "none").is_err());
    Ok(())
}

/// Scenario: A failed /models request leaves no reasoning control until the next selection retries
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_request_leaves_no_control_until_the_next_selection() -> TestResult {
    // @step Given the Codex /models request fails with status 401
    let codex = Codex::start(
        Auth::ChatGpt { expired: false },
        ResponseTemplate::new(401).set_body_json(json!({"detail": "Unauthorized"})),
        None,
    )
    .await?;
    // @step When a session starts on codex/gpt-6-luna
    let session = codex.session_on(LUNA)?;
    // @step Then it has no reasoning control and the reason names the failed Codex /models request
    let reason = codex.no_control(&session)?;
    assert!(reason.contains("Codex /models request failed"), "{reason}");
    // @step When /models answers again and the session switches to codex/gpt-6-luna
    codex.respond(ok(&codex_models()?)).await;
    codex.switch(&session, LUNA)?;
    // @step Then the thinking options are low, medium, high, xhigh, max in that order
    assert_eq!(codex.efforts(&session)?, LUNA_LEVELS);
    Ok(())
}

/// Scenario: Codex signed in with an API key only has no reasoning control
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_key_codex_has_no_reasoning_control() -> TestResult {
    // @step Given Codex is signed in with an API key and no ChatGPT tokens
    let codex = Codex::start(Auth::ApiKeyOnly, ok(&codex_models()?), None).await?;
    // @step When a session starts on codex/gpt-6-luna
    let session = codex.session_on(LUNA)?;
    // @step Then it has no reasoning control because Codex thinking levels need ChatGPT sign-in
    let reason = codex.no_control(&session)?;
    assert!(reason.contains("ChatGPT sign-in"), "{reason}");
    // @step And no /models request is sent
    assert!(codex.models_requests().await?.is_empty());
    Ok(())
}

/// Scenario: The /models request carries the pinned client version and ChatGPT auth
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_request_carries_the_pinned_client_version_and_chatgpt_auth() -> TestResult {
    // @step Given Codex is signed in with ChatGPT as account acc_model014
    let codex = Codex::signed_in().await?;
    // @step When a session starts on codex/gpt-6-luna
    codex.session_on(LUNA)?;
    // @step Then one GET /models request is sent with client_version 0.165.0, the pinned Codex client version
    assert_eq!(CODEX_MODELS_CLIENT_VERSION, "0.165.0");
    let requests = codex.models_requests().await?;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[0].url.query(), Some("client_version=0.165.0"));
    // @step And it carries the Bearer access token and ChatGPT-Account-Id acc_model014
    assert_eq!(
        requests[0].headers["authorization"],
        format!("Bearer {ACCESS}")
    );
    assert_eq!(requests[0].headers["chatgpt-account-id"], ACCOUNT);
    drop(codex);

    // @step When the access token has expired
    let expired = Codex::start(Auth::ChatGpt { expired: true }, ok(&codex_models()?), None).await?;
    let session = expired.session_on(LUNA)?;
    // @step Then the token is refreshed before the /models request is sent
    let refreshes = expired
        .issuer
        .received_requests()
        .await
        .or_fail("issuer recording")?;
    assert_eq!(refreshes.len(), 1);
    let requests = expired.models_requests().await?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers["authorization"],
        format!("Bearer {ROTATED_ACCESS}")
    );
    assert_eq!(expired.efforts(&session)?, LUNA_LEVELS);
    Ok(())
}

/// Scenario: An entry fspec cannot use fails only that model's capability
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unusable_entry_fails_only_that_model() -> TestResult {
    // @step Given a /models response listing codex-auto-review with default turbo outside its levels, gpt-6-sol with an effort named Extra High, and gpt-6-luna
    let mut models = codex_models()?;
    for entry in models["models"].as_array_mut().or_fail("models array")? {
        match entry["slug"].as_str() {
            Some("codex-auto-review") => entry["default_reasoning_level"] = json!("turbo"),
            Some("gpt-6-sol") => {
                entry["supported_reasoning_levels"]
                    .as_array_mut()
                    .or_fail("levels array")?
                    .push(json!({"effort": "Extra High", "description": "x"}));
            }
            _ => {}
        }
    }
    let codex = Codex::start(Auth::ChatGpt { expired: false }, ok(&models), None).await?;
    // @step When the Codex capabilities are resolved
    let review = codex.session_on("codex/codex-auto-review")?;
    let sol = codex.session_on(SOL)?;
    let luna = codex.session_on(LUNA)?;
    // @step Then codex/codex-auto-review has no reasoning control with a reason naming it and turbo
    let reason = codex.no_control(&review)?;
    assert!(
        reason.contains("codex-auto-review") && reason.contains("turbo"),
        "{reason}"
    );
    // @step And codex/gpt-6-sol has no reasoning control with a reason naming it and Extra High
    let reason = codex.no_control(&sol)?;
    assert!(
        reason.contains("gpt-6-sol") && reason.contains("Extra High"),
        "{reason}"
    );
    // @step And codex/gpt-6-luna offers low, medium, high, xhigh, max
    assert_eq!(codex.efforts(&luna)?, LUNA_LEVELS);
    Ok(())
}

/// Scenario: A saved level applies only when Codex advertises it
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_level_applies_only_when_codex_advertises_it() -> TestResult {
    let saved = |level: &str| Some(json!({"tui": {"thinkingLevels": {LUNA: level}}}));
    // @step Given the saved thinking level for codex/gpt-6-luna is xhigh
    let codex = Codex::start(
        Auth::ChatGpt { expired: false },
        ok(&codex_models()?),
        saved("effort:xhigh"),
    )
    .await?;
    // @step When a session starts on codex/gpt-6-luna
    let session = codex.session_on(LUNA)?;
    // @step Then its thinking level is xhigh
    assert_eq!(
        codex.level(&session)?,
        ThinkingLevel::Effort("xhigh".into())
    );
    drop(codex);
    // @step Given the saved thinking level for codex/gpt-6-luna is none
    let codex = Codex::start(
        Auth::ChatGpt { expired: false },
        ok(&codex_models()?),
        saved("effort:none"),
    )
    .await?;
    // @step When a session starts on codex/gpt-6-luna
    let session = codex.session_on(LUNA)?;
    // @step Then its thinking level is not set and a warning says codex/gpt-6-luna does not offer none
    assert_eq!(codex.level(&session)?, ThinkingLevel::ModelDefault);
    let warnings = codex.warnings(&session)?;
    assert!(
        warnings.iter().any(|warning| warning.contains(LUNA)
            && warning.contains("none")
            && warning.contains("not offered")),
        "{warnings:?}"
    );
    Ok(())
}

/// Scenario: With no level set the request carries the advertised default effort
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_no_level_set_the_advertised_default_is_sent() -> TestResult {
    // @step Given the Codex provider is built for codex/gpt-6-sol, whose /models default is medium
    let codex = Codex::signed_in().await?;
    let session = codex.session_on(SOL)?;
    assert_eq!(codex.level(&session)?, ThinkingLevel::ModelDefault);
    // @step When a request is prepared with no thinking level
    let body = codex.prepared(&session, None).await??;
    // @step Then it carries reasoning effort medium
    assert_eq!(body["reasoning"]["effort"], "medium");
    // @step When a request is prepared at low
    let config = codex
        .target(&session)
        .await?
        .config_for(Some(&effort("low")?))?
        .or_fail("low sends reasoning")?;
    let body = codex.prepared(&session, Some(config)).await??;
    // @step Then it carries reasoning effort low
    assert_eq!(body["reasoning"]["effort"], "low");
    Ok(())
}

/// Scenario: The provider snapshot comes from /models and leaves unlisted controls unvalidated
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_provider_snapshot_comes_from_models() -> TestResult {
    // @step Given the Codex provider is built for codex/gpt-6-luna from its /models entry
    let codex = Codex::signed_in().await?;
    let session = codex.session_on(LUNA)?;
    // @step When a request option asks for reasoning effort none
    let refused = codex
        .prepared(&session, Some(json!({"reasoning": {"effort": "none"}})))
        .await?;
    // @step Then the provider refuses reasoning.effort none and lists low, medium, high, xhigh, max
    let error = match refused {
        Err(CompletionError::RequestError(source)) => source
            .downcast_ref::<RequestOptionsError>()
            .map(|error| match error {
                RequestOptionsError::Unsupported {
                    option, supported, ..
                } => (option.clone(), supported.clone()),
                other => (other.to_string(), Vec::new()),
            })
            .or_fail("a request option error")?,
        other => return Err(format!("expected a refusal, got {other:?}").into()),
    };
    assert_eq!(error.0, "reasoning.effort");
    assert_eq!(error.1, LUNA_LEVELS.map(|level| json!(level)).to_vec());
    // @step When a request asks for reasoning effort xhigh with summary auto
    let body = codex
        .prepared(
            &session,
            Some(json!({"reasoning": {"effort": "xhigh", "summary": "auto"}})),
        )
        .await??;
    // @step Then the request carries effort xhigh and summary auto
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(body["reasoning"]["summary"], "auto");
    Ok(())
}

/// Scenario: /models is fetched once per model selection and not per turn
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_is_fetched_once_per_selection_not_per_turn() -> TestResult {
    // @step Given a session on codex/gpt-6-luna has fetched Codex /models once
    let codex = Codex::signed_in().await?;
    let session = codex.session_on(LUNA)?;
    assert_eq!(codex.models_requests().await?.len(), 1);
    // @step When the session prepares a turn
    {
        let background = codex.manager.get_session(&session.value)?;
        let mut inner = background.inner.lock().await;
        background.refresh_runtime_settings_for_turn(&mut inner)?;
    }
    // @step Then no further /models request is sent
    assert_eq!(codex.models_requests().await?.len(), 1);
    assert_eq!(codex.efforts(&session)?, LUNA_LEVELS);
    // @step When the session switches to codex/gpt-6-sol
    codex.switch(&session, SOL)?;
    // @step Then a second /models request is sent and gpt-6-sol's levels are offered
    assert_eq!(codex.models_requests().await?.len(), 2);
    assert_eq!(codex.efforts(&session)?, LUNA_LEVELS);
    assert_eq!(
        codex.target(&session).await?.model(),
        SOL,
        "the session's target follows the switch"
    );
    Ok(())
}
