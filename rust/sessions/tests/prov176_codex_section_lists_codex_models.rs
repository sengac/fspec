//! PROV-176: the Codex section lists only the models Codex /models returns.
//!
//! Feature: spec/features/the-codex-section-lists-only-the-models-codex-models-returns.feature
//!
//! A real `SessionManager` is rooted at a throwaway data directory whose
//! models.dev cache is the MODEL-012 fixture with gpt-4o and gpt-6-luna added
//! to the OpenAI catalog. Its Codex home holds ChatGPT tokens (or only an API
//! key), and a wiremock server answers the Codex `/models` request through
//! the MODEL-014 test-support redirect. The provider list is built and models
//! are selected exactly as the model selector does it. Tests share
//! process-global state (data directory, CODEX_HOME, FSPEC_USER_DIR,
//! redirects), so `GUARD` serialises them.

use std::sync::Arc;

use tokio::sync::{Mutex, MutexGuard};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::models_catalog;
use codelet_providers::codex::oauth_test_support::{self, OAuthTestConfig, OAuthTestGuard};
use codelet_rpc_types::ProviderInfo;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODELS_DEV_FIXTURE: &str = include_str!("fixtures/model012_models.json");

static GUARD: Mutex<()> = Mutex::const_new(());

const ACCOUNT: &str = "acc_prov176";

enum Auth {
    ChatGpt,
    ApiKeyOnly,
}

/// A /models entry.
fn entry(slug: &str, name: &str, visibility: &str) -> Value {
    json!({
        "slug": slug,
        "display_name": name,
        "visibility": visibility,
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            {"effort": "low"}, {"effort": "medium"}, {"effort": "high"}
        ]
    })
}

fn models(entries: &[Value]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "models": entries }))
}

/// The MODEL-012 models.dev fixture with gpt-4o and gpt-6-luna added to the
/// OpenAI catalog (copies of its gpt-5.5 entry).
fn models_dev() -> Result<String, TestError> {
    let mut catalog: Value = serde_json::from_str(MODELS_DEV_FIXTURE)?;
    let openai = catalog["openai"]["models"]
        .as_object_mut()
        .or_fail("the fixture's OpenAI models")?;
    let template = openai.get("gpt-5.5").cloned().or_fail("gpt-5.5")?;
    for (id, name) in [("gpt-4o", "GPT-4o"), ("gpt-6-luna", "GPT-6 Luna")] {
        let mut model = template.clone();
        model["id"] = json!(id);
        model["name"] = json!(name);
        openai.insert(id.to_string(), model);
    }
    Ok(catalog.to_string())
}

/// A session manager with a Codex home and a mocked Codex backend. Fields
/// drop in order, so the redirects are cleared before the guard is released.
struct Codex {
    manager: Arc<SessionManager>,
    backend: MockServer,
    _redirect: models_catalog::test_support::Redirect,
    _oauth: OAuthTestGuard,
    _tmp: tempfile::TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Codex {
    async fn start(auth: Auth, response: ResponseTemplate) -> Result<Self, TestError> {
        let guard = GUARD.lock().await;
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("data");
        std::fs::create_dir_all(dir.join("cache"))?;
        std::fs::write(dir.join("cache").join("models.json"), models_dev()?)?;
        let user_dir = tmp.path().join("user");
        std::fs::create_dir_all(&user_dir)?;
        std::env::set_var("FSPEC_USER_DIR", &user_dir);
        let codex_home = tmp.path().join("codex-home");
        std::fs::create_dir_all(&codex_home)?;
        let document = match auth {
            Auth::ChatGpt => json!({
                "tokens": {
                    "id_token": format!("id-{ACCOUNT}"),
                    "access_token": format!("access-{ACCOUNT}"),
                    "refresh_token": format!("refresh-{ACCOUNT}"),
                    "account_id": ACCOUNT
                },
                "last_refresh": chrono::Utc::now().to_rfc3339()
            }),
            Auth::ApiKeyOnly => json!({ "OPENAI_API_KEY": "sk-prov176-api-key" }),
        };
        std::fs::write(codex_home.join("auth.json"), document.to_string())?;
        std::env::set_var("CODEX_HOME", &codex_home);
        std::env::remove_var("CODEX_MODEL");
        std::env::remove_var("OPENAI_API_KEY");

        let backend = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(response)
            .mount(&backend)
            .await;
        let issuer = MockServer::start().await;
        let redirect = models_catalog::test_support::redirect(&backend.uri());
        let oauth = oauth_test_support::configure(OAuthTestConfig {
            issuer: issuer.uri(),
            callback_address: "127.0.0.1:0".parse()?,
            state: "prov176".into(),
        });
        codelet_common::set_data_directory(dir)?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = SessionManager::new();
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace))?;
        Ok(Self {
            manager,
            backend,
            _redirect: redirect,
            _oauth: oauth,
            _tmp: tmp,
            _guard: guard,
        })
    }

    fn handle(&self) -> &dyn SessionManagerHandle {
        self.manager.as_ref()
    }

    /// The provider list, as the model selector builds it.
    fn providers(&self) -> Result<Vec<ProviderInfo>, TestError> {
        Ok(self.handle().list_providers()?)
    }

    async fn models_requests(&self) -> Result<usize, TestError> {
        Ok(self
            .backend
            .received_requests()
            .await
            .or_fail("backend recording")?
            .iter()
            .filter(|request| request.url.path() == "/models")
            .count())
    }
}

impl Drop for Codex {
    fn drop(&mut self) {
        std::env::remove_var("FSPEC_USER_DIR");
        std::env::remove_var("OPENAI_API_KEY");
    }
}

fn codex_section(providers: &[ProviderInfo]) -> Result<&ProviderInfo, TestError> {
    Ok(providers
        .iter()
        .find(|section| section.key == "codex" && section.profile_name.is_none())
        .or_fail("a Codex section")?)
}

fn ids(section: &ProviderInfo) -> Vec<&str> {
    section
        .models
        .iter()
        .map(|model| model.id.as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_codex_section_lists_the_models_codex_models_returns() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol and gpt-6-luna with visibility list and gpt-reserve with visibility hide
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&[
            entry("gpt-6.1-sol", "GPT-6.1-Sol", "list"),
            entry("gpt-reserve", "GPT-Reserve", "hide"),
            entry("gpt-6-luna", "GPT-6-Luna", "list"),
        ]),
    )
    .await?;

    // @step When the provider list is built
    let providers = codex.providers()?;

    // @step Then the Codex section lists gpt-6.1-sol and gpt-6-luna
    let section = codex_section(&providers)?;
    assert_eq!(ids(section), vec!["gpt-6.1-sol", "gpt-6-luna"]);
    assert_eq!(section.models[0].display_name, "GPT-6.1-Sol");
    assert_eq!(section.unavailable_reason, None);
    // gpt-6-luna is known to models.dev: its context window comes from there.
    assert!(section.models[1].context_window.is_some());
    assert!(section.models[1].supports_reasoning);

    // @step And gpt-reserve, gpt-5.5 and gpt-4o are not listed under Codex
    for absent in ["gpt-reserve", "gpt-5.5", "gpt-4o"] {
        assert!(!ids(section).contains(&absent), "{absent} is listed");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_openai_provider_keeps_its_sections() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and an OpenAI API key is set
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&[entry("gpt-6-luna", "GPT-6-Luna", "list")]),
    )
    .await?;
    std::env::set_var("OPENAI_API_KEY", "sk-prov176-openai");

    // @step When the provider list is built
    let providers = codex.providers()?;

    // @step Then no standalone OpenAI cloud section is shown, as PROV-146 decided
    assert!(
        !providers
            .iter()
            .any(|section| section.key == "openai" && section.profile_name.is_none()),
        "{:?}",
        providers
            .iter()
            .map(|section| &section.key)
            .collect::<Vec<_>>()
    );

    // @step And no section lists the models.dev OpenAI catalog's gpt-5.5 or gpt-4o
    for section in &providers {
        for absent in ["gpt-5.5", "gpt-4o"] {
            assert!(
                !ids(section).contains(&absent),
                "{} lists {absent}",
                section.key
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn codex_models_is_fetched_every_time_the_provider_list_is_built() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&[
            entry("gpt-6.1-sol", "GPT-6.1-Sol", "list"),
            entry("gpt-6-luna", "GPT-6-Luna", "list"),
        ]),
    )
    .await?;
    // The first request is answered without gpt-6-luna.
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(models(&[entry("gpt-6.1-sol", "GPT-6.1-Sol", "list")]))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&codex.backend)
        .await;

    // @step When the provider list is built, /models then also returns gpt-6-luna, and the provider list is built again
    let first = codex.providers()?;
    let second = codex.providers()?;
    assert_eq!(ids(codex_section(&first)?), vec!["gpt-6.1-sol"]);

    // @step Then /models was requested twice
    assert_eq!(codex.models_requests().await?, 2);

    // @step And the second Codex section lists gpt-6.1-sol and gpt-6-luna
    assert_eq!(
        ids(codex_section(&second)?),
        vec!["gpt-6.1-sol", "gpt-6-luna"]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_models_leaves_the_codex_section_empty_with_the_reason() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and /models answers 401
    let codex = Codex::start(Auth::ChatGpt, ResponseTemplate::new(401)).await?;

    // @step When the provider list is built
    let providers = codex.providers()?;

    // @step Then the Codex section is shown with no models and the reason the Codex /models request failed with status 401
    let section = codex_section(&providers)?;
    assert!(section.models.is_empty(), "{:?}", ids(section));
    let reason = section.unavailable_reason.as_deref().or_fail("a reason")?;
    assert!(reason.contains("Codex /models request failed"), "{reason}");
    assert!(reason.contains("401"), "{reason}");

    // @step And no models.dev OpenAI model is listed under Codex
    for absent in ["gpt-5", "gpt-5.5", "gpt-4o", "gpt-6-luna"] {
        assert!(!ids(section).contains(&absent));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn codex_signed_in_with_an_api_key_only_lists_no_codex_models() -> TestResult {
    // @step Given Codex is signed in with an API key and not with ChatGPT
    let codex = Codex::start(
        Auth::ApiKeyOnly,
        models(&[entry("gpt-6-luna", "GPT-6-Luna", "list")]),
    )
    .await?;

    // @step When the provider list is built
    let providers = codex.providers()?;

    // @step Then the Codex section is shown with no models and the reason that Codex models need ChatGPT sign-in
    let section = codex_section(&providers)?;
    assert!(section.models.is_empty(), "{:?}", ids(section));
    let reason = section.unavailable_reason.as_deref().or_fail("a reason")?;
    assert!(reason.contains("ChatGPT sign-in"), "{reason}");
    assert_eq!(codex.models_requests().await?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_listed_model_models_dev_does_not_know_is_refused_at_selection() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol, which models.dev does not know
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&[entry("gpt-6.1-sol", "GPT-6.1-Sol", "list")]),
    )
    .await?;

    // @step When the provider list is built and the user selects codex/gpt-6.1-sol
    let providers = codex.providers()?;
    let selected = codex
        .handle()
        .set_default_model("codex/gpt-6.1-sol")
        .and_then(|()| codex.handle().create_session_checked(None));

    // @step Then gpt-6.1-sol is listed under Codex
    let section = codex_section(&providers)?;
    assert_eq!(ids(section), vec!["gpt-6.1-sol"]);
    assert_eq!(section.models[0].context_window, None);

    // @step And the selection is refused with an error naming its missing context window and output limit
    let error = match selected {
        Ok(session) => return Err(format!("selection succeeded: {session:?}").into()),
        Err(error) => error,
    };
    assert!(error.contains("gpt-6.1-sol"), "{error}");
    assert!(error.contains("context window"), "{error}");
    assert!(error.contains("output limit"), "{error}");
    Ok(())
}
