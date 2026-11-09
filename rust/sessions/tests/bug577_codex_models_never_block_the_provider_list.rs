//! BUG-577: a slow, failing or empty Codex /models never blocks the provider
//! list or replaces the saved model.
//!
//! Feature: spec/features/codex-models-never-block-the-provider-list-or-replace-the-saved-model.feature
//!
//! A real `SessionManager` is rooted at a throwaway data directory (the
//! MODEL-012 models.dev fixture) with a Codex home holding ChatGPT tokens (or
//! only an API key). The Codex `/models` request goes, through the MODEL-014
//! test-support redirect, to a small HTTP server in this file that answers
//! each request with a fixed body, either at once or only when the test
//! releases it, and counts the requests. Tests share process-global state
//! (data directory, CODEX_HOME, FSPEC_USER_DIR, redirects, the in-flight
//! fetch), so `GUARD` serialises them.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex as StdMutex, PoisonError};
use std::time::Duration;

use tokio::sync::{Mutex, MutexGuard};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::models_catalog::test_support;
use codelet_providers::codex::oauth_test_support::{self, OAuthTestConfig, OAuthTestGuard};
use codelet_rpc_types::{ModelEntry, ProviderInfo};
use codelet_sessions::startup_model_resolution::resolve_startup_model;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{
    deadlock_guard_blocking, wait_until_blocking, OrFail, TestError, TestResult,
};
use serde_json::{json, Value};

const MODELS_DEV_FIXTURE: &str = include_str!("fixtures/model012_models.json");
const ACCOUNT: &str = "acc_bug577";

static GUARD: Mutex<()> = Mutex::const_new(());

enum Auth {
    ChatGpt,
    ApiKeyOnly,
}

/// When the server answers `/models`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    Now,
    WhenReleased,
}

/// The server's shared state: requests seen, and whether answers may go.
struct Gate {
    requests: AtomicUsize,
    released: StdMutex<bool>,
    opened: Condvar,
}

/// A local HTTP server for the Codex `/models` request.
struct ModelsServer {
    uri: String,
    gate: Arc<Gate>,
}

impl ModelsServer {
    fn start(body: Value, answer: Answer) -> Result<Self, TestError> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let uri = format!("http://{}", listener.local_addr()?);
        let gate = Arc::new(Gate {
            requests: AtomicUsize::new(0),
            released: StdMutex::new(answer == Answer::Now),
            opened: Condvar::new(),
        });
        let serving = Arc::clone(&gate);
        let body = body.to_string();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let gate = Arc::clone(&serving);
                let body = body.clone();
                std::thread::spawn(move || serve(stream, &gate, &body));
            }
        });
        Ok(Self { uri, gate })
    }

    fn requests(&self) -> usize {
        self.gate.requests.load(Ordering::SeqCst)
    }

    fn release(&self) {
        *self
            .gate
            .released
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        self.gate.opened.notify_all();
    }
}

fn serve(stream: TcpStream, gate: &Gate, body: &str) {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
        if line == "\r\n" {
            break;
        }
        line.clear();
    }
    gate.requests.fetch_add(1, Ordering::SeqCst);
    let released = gate.released.lock().unwrap_or_else(PoisonError::into_inner);
    drop(
        gate.opened
            .wait_while(released, |released| !*released)
            .unwrap_or_else(PoisonError::into_inner),
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = &stream;
    let _ = stream.write_all(response.as_bytes());
}

/// A /models entry.
fn entry(slug: &str) -> Value {
    json!({
        "slug": slug,
        "display_name": slug,
        "visibility": "list",
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [{"effort": "low"}, {"effort": "medium"}]
    })
}

fn models(slugs: &[&str]) -> Value {
    json!({ "models": slugs.iter().map(|slug| entry(slug)).collect::<Vec<_>>() })
}

/// A session manager with a Codex home and the local `/models` server.
/// Fields drop in order, so the redirects are cleared before the guard is
/// released.
struct Codex {
    manager: Arc<SessionManager>,
    server: ModelsServer,
    _redirect: test_support::Redirect,
    _oauth: OAuthTestGuard,
    _tmp: tempfile::TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Codex {
    async fn start(auth: Auth, body: Value, answer: Answer) -> Result<Self, TestError> {
        let guard = GUARD.lock().await;
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("data");
        std::fs::create_dir_all(dir.join("cache"))?;
        std::fs::write(dir.join("cache").join("models.json"), MODELS_DEV_FIXTURE)?;
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
            Auth::ApiKeyOnly => json!({ "OPENAI_API_KEY": "sk-bug577-api-key" }),
        };
        std::fs::write(codex_home.join("auth.json"), document.to_string())?;
        std::env::set_var("CODEX_HOME", &codex_home);
        std::env::remove_var("CODEX_MODEL");
        std::env::remove_var("OPENAI_API_KEY");

        let server = ModelsServer::start(body, answer)?;
        let redirect = test_support::redirect(&server.uri);
        let oauth = oauth_test_support::configure(OAuthTestConfig {
            issuer: "http://127.0.0.1:1".into(),
            callback_address: "127.0.0.1:0".parse()?,
            state: "bug577".into(),
        });
        codelet_common::set_data_directory(dir)?;
        codelet_core::persistence::reset_stores_for_tests();
        let manager = SessionManager::new();
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace))?;
        Ok(Self {
            manager,
            server,
            _redirect: redirect,
            _oauth: oauth,
            _tmp: tmp,
            _guard: guard,
        })
    }

    /// The provider list, built on its own thread (it blocks).
    fn providers(&self) -> Result<Vec<ProviderInfo>, TestError> {
        let manager = Arc::clone(&self.manager);
        Ok(deadlock_guard_blocking("the provider list", move || {
            manager.as_ref().list_providers()
        })??)
    }

    /// Answer every waiting request and wait until no fetch is in flight,
    /// so the next test never joins this test's fetch.
    fn finish(&self) -> TestResult {
        self.server.release();
        Ok(wait_until_blocking("the Codex fetch to end", || {
            !test_support::listing_in_flight()
        })?)
    }
}

impl Drop for Codex {
    fn drop(&mut self) {
        self.server.release();
        std::env::remove_var("FSPEC_USER_DIR");
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

fn reason(section: &ProviderInfo) -> Result<&str, TestError> {
    Ok(section.unavailable_reason.as_deref().or_fail("a reason")?)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_codex_model_is_kept_when_codex_models_lists_no_models() -> TestResult {
    // @step Given my saved model is codex/gpt-6.1-terra and Codex /models answers with no models
    let codex = Codex::start(Auth::ChatGpt, models(&[]), Answer::Now).await?;
    let saved = "codex/gpt-6.1-terra";

    // @step When the provider list is built and the startup model is resolved
    let providers = codex.providers()?;
    let resolved = resolve_startup_model(&providers, Some(saved));

    // @step Then the Codex section is shown with no models and the reason that Codex /models listed none
    let section = codex_section(&providers)?;
    assert!(section.models.is_empty(), "{:?}", ids(section));
    assert!(reason(section)?.contains("listed no models"), "{section:?}");

    // @step And the startup model is codex/gpt-6.1-terra
    assert_eq!(
        resolved.map(|resolved| resolved.model_string).as_deref(),
        Some(saved)
    );
    codex.finish()
}

#[test]
fn a_saved_codex_model_that_codex_models_no_longer_lists_gives_way() {
    // @step Given my saved model is codex/gpt-retired and the Codex section lists only gpt-6.1-sol
    let model = |id: &str| ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(200_000),
        supports_reasoning: true,
        supports_vision: false,
        is_custom: false,
    };
    let section = |key: &str, ids: &[&str]| ProviderInfo {
        key: key.to_string(),
        display_name: key.to_string(),
        models: ids.iter().map(|id| model(id)).collect(),
        provider_id: key.to_string(),
        ..ProviderInfo::default()
    };
    let providers = vec![
        section("codex", &["gpt-6.1-sol"]),
        section("anthropic", &["claude-opus-5-5"]),
    ];

    // @step When the startup model is resolved
    let resolved = resolve_startup_model(&providers, Some("codex/gpt-retired"));

    // @step Then the first available model is chosen
    assert_eq!(
        resolved.map(|resolved| resolved.model_string).as_deref(),
        Some("codex/gpt-6.1-sol")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_provider_list_does_not_wait_past_the_list_budget_for_codex_models() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and Codex /models does not answer
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&["gpt-6.1-sol"]),
        Answer::WhenReleased,
    )
    .await?;
    let budget = test_support::list_budget(Duration::ZERO);

    // @step When the provider list is built
    let providers = codex.providers()?;
    drop(budget);

    // @step Then the provider list is returned with the Codex section showing no models
    let section = codex_section(&providers)?;
    assert!(section.models.is_empty(), "{:?}", ids(section));

    // @step And the reason says the Codex model list did not arrive in time
    assert!(
        reason(section)?.contains("did not arrive in time"),
        "{section:?}"
    );
    codex.finish()
}

#[tokio::test(flavor = "multi_thread")]
async fn provider_lists_built_during_one_codex_models_request_share_it() -> TestResult {
    // @step Given Codex is signed in with ChatGPT and Codex /models answers only when released
    let codex = Codex::start(
        Auth::ChatGpt,
        models(&["gpt-6.1-sol", "gpt-6-luna"]),
        Answer::WhenReleased,
    )
    .await?;

    // @step When two provider lists are built while the request is in flight and the answer is then released
    let lists = [Arc::clone(&codex.manager), Arc::clone(&codex.manager)]
        .map(|manager| std::thread::spawn(move || manager.as_ref().list_providers()));
    let gate = Arc::clone(&codex.server.gate);
    wait_until_blocking("both lists to wait on the request", move || {
        let requests = gate.requests.load(Ordering::SeqCst);
        requests >= 2 || (requests == 1 && test_support::waiting_listings() == 2)
    })?;
    codex.server.release();
    let mut sections = Vec::new();
    for list in lists {
        let providers = deadlock_guard_blocking("a provider list", move || list.join())?
            .map_err(|_| "a provider list panicked")??;
        sections.push(codex_section(&providers)?.clone());
    }

    // @step Then Codex /models was requested once
    assert_eq!(codex.server.requests(), 1);

    // @step And both Codex sections list the models it returned
    for section in &sections {
        assert_eq!(ids(section), vec!["gpt-6.1-sol", "gpt-6-luna"]);
    }
    codex.finish()
}

#[tokio::test(flavor = "multi_thread")]
async fn without_chatgpt_sign_in_the_codex_section_says_where_to_sign_in() -> TestResult {
    // @step Given Codex is signed in with an API key and not with ChatGPT
    let codex = Codex::start(Auth::ApiKeyOnly, models(&["gpt-6.1-sol"]), Answer::Now).await?;

    // @step When the provider list is built
    let providers = codex.providers()?;

    // @step Then the Codex section's reason says Codex models need ChatGPT sign-in under /provider
    let section = codex_section(&providers)?;
    let reason = reason(section)?;
    assert!(reason.contains("ChatGPT sign-in"), "{reason}");
    assert!(reason.contains("/provider"), "{reason}");
    assert!(!reason.contains("thinking levels"), "{reason}");
    assert_eq!(codex.server.requests(), 0);
    codex.finish()
}
