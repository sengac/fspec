use crate::prov159_support::{Fixture, TestResult};
use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use rig::http_client::HttpClientExt;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The ChatGPT account the fixture's OAuth credentials belong to.
pub const ACCOUNT: &str = "acc_12345";
/// Access token seeded for [`ACCOUNT`] (`prov159_support::tokens`).
pub const ACCESS: &str = "access-acc_12345";
/// Access token the mocked OAuth token endpoint rotates to.
pub const ROTATED_ACCESS: &str = "access-acc_12345-rotated";
/// Refresh token the mocked OAuth token endpoint rotates to.
pub const ROTATED_REFRESH: &str = "refresh-acc_12345-rotated";

pub struct Case {
    pub client: RefreshingCodexClient,
    pub issuer: MockServer,
    pub backend: MockServer,
    _fixture: Fixture,
}

pub fn request(url: &str) -> Result<http::Request<Vec<u8>>, http::Error> {
    http::Request::builder()
        .method("POST")
        .uri(url)
        .header("content-type", "application/json")
        .body(Vec::new())
}

impl Case {
    pub async fn new(expiry: Option<u64>) -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = Fixture::new()?;
        fixture.seed(ACCOUNT)?;
        let issuer = MockServer::start().await;
        let backend = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: [DONE]\n\n"),
            )
            .mount(&backend)
            .await;
        let client = RefreshingCodexClient::new_oauth(
            ACCESS.into(),
            format!("refresh-{ACCOUNT}"),
            ACCOUNT.into(),
            expiry,
            issuer.uri(),
        );
        Ok(Self {
            client,
            issuer,
            backend,
            _fixture: fixture,
        })
    }

    pub fn url(&self) -> String {
        format!("{}/v1/models", self.backend.uri())
    }

    pub async fn refresh(&self, status: u16, expiry: Option<u64>) {
        let mut body = serde_json::json!({
            "access_token": ROTATED_ACCESS,
            "refresh_token": ROTATED_REFRESH,
            "id_token": "opaque-id"
        });
        if let Some(expiry) = expiry {
            body["expires_in"] = expiry.into();
        }
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains(format!(
                "refresh_token=refresh-{ACCOUNT}"
            )))
            .and(body_string_contains(format!(
                "client_id={}",
                codelet_providers::codex::codex_oauth::CODEX_CLIENT_ID
            )))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(&self.issuer)
            .await;
    }

    pub async fn send(&self) -> TestResult {
        let response: http::Response<rig::http_client::LazyBody<Vec<u8>>> =
            self.client.send(request(&self.url())?).await?;
        assert_eq!(response.status(), 200);
        Ok(())
    }

    pub async fn backend_requests(
        &self,
    ) -> Result<Vec<wiremock::Request>, Box<dyn std::error::Error>> {
        self.backend
            .received_requests()
            .await
            .ok_or_else(|| "backend recording disabled".into())
    }

    pub async fn issuer_requests(
        &self,
    ) -> Result<Vec<wiremock::Request>, Box<dyn std::error::Error>> {
        self.issuer
            .received_requests()
            .await
            .ok_or_else(|| "issuer recording disabled".into())
    }

    pub async fn assert_headers(&self, access: &str) -> TestResult {
        let requests = self.backend_requests().await?;
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers["authorization"],
            format!("Bearer {access}")
        );
        assert_eq!(requests[0].headers["chatgpt-account-id"], ACCOUNT);
        assert_eq!(requests[0].headers["originator"], "rust");
        Ok(())
    }
}
