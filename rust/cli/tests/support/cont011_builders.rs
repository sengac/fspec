use super::{
    native::{Case, TestResult},
    wire::Kind,
};
use codelet_core::RigAgent;
use codelet_providers::{AuthMode, ClaudeProvider, GeminiProvider, ZAIProvider};

pub fn assert_rejected_endpoints() -> TestResult {
    use codelet_providers::claude_refreshing_client::RefreshingClaudeClient;
    for endpoint in [
        "https://127.0.0.1",
        "http://localhost",
        "http://192.0.2.1",
        "http://user@127.0.0.1",
        "http://127.0.0.1?unowned=value",
        "http://127.0.0.1#fragment",
    ] {
        let claude = rig::providers::anthropic::Client::<RefreshingClaudeClient>::builder()
            .api_key("synthetic-loopback-only")
            .base_url(endpoint)
            .http_client(RefreshingClaudeClient::new_api_key())
            .build()?;
        assert!(ClaudeProvider::from_test_client(claude, "synthetic", AuthMode::ApiKey).is_err());
        let gemini = rig::providers::gemini::Client::builder()
            .api_key("synthetic-loopback-only")
            .base_url(endpoint)
            .build()?;
        assert!(GeminiProvider::from_test_client(gemini, "synthetic").is_err());
        let zai = rig::providers::openai::CompletionsClient::builder()
            .api_key("synthetic-loopback-only")
            .base_url(endpoint)
            .build()?;
        assert!(ZAIProvider::from_test_client(zai, "synthetic").is_err());
    }
    Ok(())
}

pub async fn run(fixture: &mut Case) -> TestResult {
    match fixture.kind {
        Kind::Claude | Kind::OAuth => {
            use codelet_providers::claude_refreshing_client::RefreshingClaudeClient;
            let oauth = matches!(fixture.kind, Kind::OAuth);
            let http = if oauth {
                RefreshingClaudeClient::new_oauth(
                    "sk-ant-oat-synthetic-only".into(),
                    "synthetic-refresh".into(),
                    Some(3600),
                    fixture.server.uri(),
                )
            } else {
                RefreshingClaudeClient::new_api_key()
            };
            let client = rig::providers::anthropic::Client::<RefreshingClaudeClient>::builder()
                .api_key(if oauth {
                    "sk-ant-oat-synthetic-only"
                } else {
                    "synthetic-loopback-only"
                })
                .base_url(fixture.server.uri())
                .http_client(http)
                .build()?;
            let provider = ClaudeProvider::from_test_client(
                client,
                "claude-sonnet-4-5",
                if oauth {
                    AuthMode::OAuth
                } else {
                    AuthMode::ApiKey
                },
            )?;
            fixture
                .drive(RigAgent::new(
                    provider.create_rig_agent(
                        fixture.session_id,
                        Some("Local synthetic verification"),
                        None,
                    ),
                    8,
                ))
                .await
        }
        Kind::Gemini => {
            let client = rig::providers::gemini::Client::builder()
                .api_key("synthetic-loopback-only")
                .base_url(fixture.server.uri())
                .build()?;
            let provider = GeminiProvider::from_test_client(client, "gemini-2.5-pro")?;
            fixture
                .drive(RigAgent::new(
                    provider.create_rig_agent(
                        fixture.session_id,
                        Some("Local synthetic verification"),
                        None,
                    ),
                    8,
                ))
                .await
        }
        Kind::Zai => {
            let client = rig::providers::openai::CompletionsClient::builder()
                .api_key("synthetic-loopback-only")
                .base_url(fixture.server.uri())
                .build()?;
            let provider = ZAIProvider::from_test_client(client, "glm-4.7")?;
            fixture
                .drive(RigAgent::new(
                    provider.create_rig_agent(
                        fixture.session_id,
                        Some("Local synthetic verification"),
                        None,
                    ),
                    8,
                ))
                .await
        }
    }
}
