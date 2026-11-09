use base64::Engine;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_test_helpers::{setup_temp_directory, OrFailErr};
use std::sync::Arc;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct HomeGuard(Option<std::ffi::OsString>);

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("CODEX_HOME", value),
            None => std::env::remove_var("CODEX_HOME"),
        }
    }
}

fn service(root: &std::path::Path) -> Result<FspecServiceImpl, Box<dyn std::error::Error>> {
    std::fs::create_dir(root.join("spec"))?;
    std::fs::write(root.join("spec/work-units.json"), "{\"workUnits\":{}}")?;
    Ok(FspecServiceImpl::new(Arc::new(SharedFspecService::new(
        Arc::new(WorkUnitsWatcher::new(root)?),
    ))))
}

#[tokio::test]
#[serial_test::serial]
async fn daemon_exchanges_code_and_commits_tokens() -> TestResult {
    // @step Given a daemon with isolated credentials and a local token issuer
    let root = setup_temp_directory()?;
    let home = setup_temp_directory()?;
    let _home_guard = HomeGuard(std::env::var_os("CODEX_HOME"));
    std::env::set_var("CODEX_HOME", home.path());
    let issuer = MockServer::start().await;
    let _issuer_guard = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    let service = service(root.path())?;
    let start = service
        .clone()
        .oauth_headless_start(tarpc::context::current(), "codex".into())
        .await?;
    assert!(service
        .clone()
        .oauth_headless_complete(
            tarpc::context::current(),
            "codex".into(),
            "wrong-code#wrong-verifier".into(),
            "wrong-verifier".into(),
        )
        .await
        .is_err());
    assert!(issuer
        .received_requests()
        .await
        .ok_or("request recording disabled")?
        .is_empty());
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"chatgpt_account_id":"account-browser"}"#);
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("code=browser-code"))
        .and(body_string_contains(
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("code_verifier", &start.pkce_verifier)
                .finish(),
        ))
        .and(body_string_contains(
            "redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id_token": format!("e30.{payload}.signature"),
            "access_token": "access-browser", "refresh_token": "refresh-browser", "expires_in": 3600
        })))
        .expect(1)
        .mount(&issuer)
        .await;
    // @step When Codex completion receives a code with matching verifier state
    service
        .oauth_headless_complete(
            tarpc::context::current(),
            "codex".into(),
            format!("browser-code#{}", start.pkce_verifier),
            start.pkce_verifier,
        )
        .await?;
    // @step Then the issuer receives the code and fixed localhost callback redirect
    issuer.verify().await;
    // @step And the daemon persists tokens and their expiry through the login lifecycle
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.path().join("auth.json"))?)?;
    assert_eq!(document["tokens"]["access_token"], "access-browser");
    assert_eq!(document["tokens"]["account_id"], "account-browser");
    assert!(document["fspec_expires_at"].is_i64());
    assert!(document["fspec_generation"].is_string());
    Ok(())
}

#[tokio::test(start_paused = true)]
#[serial_test::serial]
async fn authorization_expires_at_original_deadline() -> TestResult {
    // @step Given a daemon has started Codex browser authorization
    let root = setup_temp_directory()?;
    let home = setup_temp_directory()?;
    let _home_guard = HomeGuard(std::env::var_os("CODEX_HOME"));
    std::env::set_var("CODEX_HOME", home.path());
    let issuer = MockServer::start().await;
    let _issuer_guard = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    let daemon = service(root.path())?;
    let start = daemon
        .clone()
        .oauth_headless_start(tarpc::context::current(), "codex".into())
        .await?;
    // @step When five minutes pass without a callback
    tokio::time::advance(std::time::Duration::from_secs(300)).await;
    // @step Then completion is rejected without token exchange and another login can start
    assert!(daemon
        .clone()
        .oauth_headless_complete(
            tarpc::context::current(),
            "codex".into(),
            format!("late#{}", start.pkce_verifier),
            start.pkce_verifier,
        )
        .await
        .is_err());
    assert!(issuer
        .received_requests()
        .await
        .ok_or("request recording disabled")?
        .is_empty());
    daemon
        .clone()
        .oauth_headless_start(tarpc::context::current(), "codex".into())
        .await?;
    daemon
        .oauth_clear_tokens(tarpc::context::current(), "codex".into())
        .await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn daemon_rejects_invalid_state_before_exchange() -> TestResult {
    // @step Given a daemon with isolated credentials and a local token issuer
    let root = setup_temp_directory()?;
    let home = setup_temp_directory()?;
    let _home_guard = HomeGuard(std::env::var_os("CODEX_HOME"));
    std::env::set_var("CODEX_HOME", home.path());
    let issuer = MockServer::start().await;
    let _issuer_guard = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    let service = service(root.path())?;
    // @step When Codex completion receives missing or mismatched verifier state
    for code in ["code", "code#wrong", "#verifier"] {
        let error = service
            .clone()
            .oauth_headless_complete(
                tarpc::context::current(),
                "codex".into(),
                code.into(),
                "verifier".into(),
            )
            .await
            .or_fail_err("invalid completion must fail")?;
        assert!(error.contains("state") || error.contains("code"), "{error}");
        assert!(
            !error.contains("unsupported"),
            "Codex completion must be implemented"
        );
    }
    // @step Then no token request or credential write occurs
    assert!(issuer
        .received_requests()
        .await
        .ok_or("request recording disabled")?
        .is_empty());
    assert_eq!(std::fs::read_dir(home.path())?.count(), 0);
    Ok(())
}
