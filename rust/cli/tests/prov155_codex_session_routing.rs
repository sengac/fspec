//! Feature: spec/features/codex-session-cache-routing.feature
use codelet_cli::session::Session;
use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use codelet_providers::codex::session::CodexSession;
use rig::http_client::HttpClientExt;
use serde_json::{json, Value};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial_test::serial]
async fn session_routing_persists_across_provider_reconstruction_and_resets_with_history(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a session with Codex routing and a server turn state
    let directory = tempfile::tempdir()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-codex-turn-state", "server-state")
                .set_body_string(""),
        )
        .mount(&server)
        .await;
    let mut session =
        Session::from_provider_manager(codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::Codex,
            Some(272_000),
            Some(4096),
        ));
    let client = RefreshingCodexClient::new_api_key();
    let routing = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("routing missing")?
        .routing
        .clone();
    for handle in [routing.clone(), routing.clone()] {
        let mut request =
            http::Request::post(server.uri()).body(json!({"input":[]}).to_string())?;
        request.extensions_mut().insert(handle);
        client.send_streaming(request).await?;
    }
    // @step When a new turn starts and the conversation is replaced
    session.begin_provider_turn();
    let mut request = http::Request::post(server.uri()).body(json!({"input":[]}).to_string())?;
    request.extensions_mut().insert(routing.clone());
    client.send_streaming(request).await?;
    session.reset_provider_state()?;
    let mut request = http::Request::post(server.uri()).body(json!({"input":[]}).to_string())?;
    request.extensions_mut().insert(routing.clone());
    client.send_streaming(request).await?;
    // @step Then the new turn keeps affinity without turn state and replacement rotates affinity
    let requests = server.received_requests().await.ok_or("requests missing")?;
    let key = |index: usize| -> Result<Value, serde_json::Error> {
        Ok(requests[index].body_json::<Value>()?["prompt_cache_key"].clone())
    };
    assert_eq!(key(0)?, key(1)?);
    assert_eq!(key(0)?, key(2)?);
    assert_ne!(key(0)?, key(3)?);
    assert!(requests[1].headers.contains_key("x-codex-turn-state"));
    assert!(!requests[2].headers.contains_key("x-codex-turn-state"));
    assert!(!requests[3].headers.contains_key("x-codex-turn-state"));
    // @step And another session and an ordinary provider share no routing
    let other = Session::from_provider_manager(codelet_providers::ProviderManager::for_testing(
        codelet_providers::ProviderType::Codex,
        Some(272_000),
        Some(4096),
    ));
    let mut request = http::Request::post(server.uri()).body(json!({"input":[]}).to_string())?;
    request.extensions_mut().insert(
        other
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("other routing missing")?
            .routing
            .clone(),
    );
    client.send_streaming(request).await?;
    client
        .send_streaming(http::Request::post(server.uri()).body(json!({"input":[]}).to_string())?)
        .await?;
    let requests = server.received_requests().await.ok_or("requests missing")?;
    assert_ne!(
        requests[3].body_json::<Value>()?["prompt_cache_key"],
        requests[4].body_json::<Value>()?["prompt_cache_key"]
    );
    assert!(requests[5]
        .body_json::<Value>()?
        .get("prompt_cache_key")
        .is_none());
    Ok(())
}
