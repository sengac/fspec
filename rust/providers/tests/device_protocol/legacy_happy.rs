use crate::prov159_support::{Fixture, TestResult};
use crate::support::*;
use codelet_providers::codex::codex_auth::CodexTokens;
use codelet_providers::codex::codex_device_auth::device_auth_login;
use serial_test::serial;
use std::sync::{Arc, Mutex};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn test_successful_device_auth_login_completes_end_to_end() -> TestResult {
    // @step Given no Codex credentials exist in auth.json
    let fixture = Fixture::new()?;
    let auth = fixture.directory.path().join("auth.json");
    assert!(!auth.exists());
    let server = MockServer::start().await;
    // @step When the user initiates device auth login
    // @step Then a device authorization request should be POST-ed to the usercode endpoint with client_id
    // @step And the response should contain a device_auth_id, user_code, and polling interval
    official_start(&server, usercode()).await;
    let displayed = Arc::new(Mutex::new(Vec::new()));
    let captured = displayed.clone();
    let mut configuration = config(&server, 2000);
    // @step And the user should see the user_code and the verification URL to visit
    configuration.display_fn = Some(Box::new(move |code, url| {
        captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((code.to_owned(), url.to_owned()));
    }));
    // @step When the system polls the token endpoint at the specified interval
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = attempts.clone();
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(move |request: &wiremock::Request| {
            if !is_poll_request(request) {
                return ResponseTemplate::new(415);
            }
            if counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error":"authorization_pending"}))
            } else {
                ResponseTemplate::new(200).set_body_json(authorized())
            }
        })
        .mount(&server)
        .await;
    // @step And the user completes authorization on the external device
    // @step Then the polling endpoint should return an authorization_code and code_verifier
    // @step And the authorization_code should be exchanged for tokens using the device callback redirect_uri
    official_exchange(&server).await;
    let tokens = device_auth_login(configuration).await?;
    // @step And the account_id should be extracted from the JWT id_token claims
    assert_eq!(tokens.account_id, "SYNTHETIC-account");
    assert_eq!(tokens.access_token, "SYNTHETIC-access");
    assert_eq!(tokens.refresh_token, "SYNTHETIC-refresh");
    assert!(!tokens.id_token.is_empty());
    assert_eq!(
        *displayed.lock().map_err(|_| "display lock poisoned")?,
        vec![(
            USER_CODE.to_owned(),
            format!("{}/codex/device", server.uri())
        )]
    );
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
    // @step And the tokens should be persisted to auth.json with refresh_token, access_token, and account_id
    let persisted: serde_json::Value = serde_json::from_slice(&std::fs::read(auth)?)?;
    assert_eq!(persisted["tokens"]["access_token"], tokens.access_token);
    assert_eq!(persisted["tokens"]["refresh_token"], tokens.refresh_token);
    assert_eq!(persisted["tokens"]["account_id"], tokens.account_id);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_exchange_uses_correct_parameters_with_device_callback() -> TestResult {
    // @step Given a device auth login received a successful polling response
    // @step And the response contains authorization_code and code_verifier
    let _fixture = Fixture::new()?;
    let server = MockServer::start().await;
    official_start(&server, usercode()).await;
    official_poll(&server).await;
    // @step When the authorization_code is exchanged at the token endpoint
    // @step Then the exchange should POST grant_type authorization_code, code, code_verifier, and client_id
    // @step And the exchange should include the issuer deviceauth callback redirect_uri
    official_exchange(&server).await;
    let tokens = device_auth_login(config(&server, 2000)).await?;
    // @step And the response should contain id_token, access_token, and refresh_token
    assert_eq!(tokens.access_token, "SYNTHETIC-access");
    assert_eq!(tokens.refresh_token, "SYNTHETIC-refresh");
    assert!(!tokens.id_token.is_empty());
    assert_eq!(count(&server, "/oauth/token").await, 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_device_auth_produces_same_codex_tokens_output_as_browser_oauth() -> TestResult {
    // @step Given a device auth login completes successfully
    let _fixture = Fixture::new()?;
    let server = MockServer::start().await;
    official_start(&server, usercode()).await;
    official_poll(&server).await;
    official_exchange(&server).await;
    // @step When the tokens are returned
    let tokens: CodexTokens = device_auth_login(config(&server, 2000)).await?;
    // @step Then the output should be a CodexTokens with id_token, access_token, refresh_token, and account_id
    assert!(!tokens.id_token.is_empty());
    assert_eq!(tokens.access_token, "SYNTHETIC-access");
    assert_eq!(tokens.refresh_token, "SYNTHETIC-refresh");
    assert_eq!(tokens.account_id, "SYNTHETIC-account");
    // @step And the output should be identical in structure to browser OAuth login output
    let serialized = serde_json::to_value(&tokens)?;
    for field in ["id_token", "access_token", "refresh_token", "account_id"] {
        assert!(serialized.get(field).is_some());
    }
    let restored: CodexTokens = serde_json::from_value(serialized)?;
    assert_eq!(restored.account_id, tokens.account_id);
    assert_eq!(restored.refresh_token, tokens.refresh_token);
    Ok(())
}
