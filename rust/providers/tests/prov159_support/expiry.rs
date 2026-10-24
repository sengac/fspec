use super::*;
use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexProvider};
use wiremock::{matchers::method, Mock, ResponseTemplate};

pub async fn unrepresentable_lifetimes() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let explicit = RefreshingCodexClient::new_oauth(
        "access-A".into(),
        "refresh-A".into(),
        "A".into(),
        Some(u64::MAX),
        "http://127.0.0.1:1".into(),
    );
    let path = fixture.directory.path().join("auth.json");
    let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    document["fspec_expires_at"] = u64::MAX.into();
    std::fs::write(&path, document.to_string())?;
    let previous_model = std::env::var_os("CODEX_MODEL");
    std::env::set_var("CODEX_MODEL", "test-model");
    let persisted = CodexProvider::new();
    match previous_model {
        Some(previous) => std::env::set_var("CODEX_MODEL", previous),
        None => std::env::remove_var("CODEX_MODEL"),
    }
    let persisted = persisted?;
    let issuer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id_token": "id-A", "access_token": "renewed-A",
            "refresh_token": "renewed-refresh-A", "expires_in": u64::MAX
        })))
        .expect(1)
        .mount(&issuer)
        .await;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    let refreshed = client(&issuer.uri(), 0);
    bounded(send(&refreshed, &api.uri())).await??;
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(3601)).await;
    let expired = (
        explicit.is_token_expired().await,
        persisted.client().http_client().is_token_expired().await,
        refreshed.is_token_expired().await,
    );
    tokio::time::resume();
    assert_eq!(expired, (true, true, true));
    Ok(())
}
