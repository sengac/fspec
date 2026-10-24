use super::codex_oauth::{TokenRefreshResponse, CODEX_CLIENT_ID};
use rig::http_client::Error;

pub(super) fn http_client() -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .dns_resolver(std::sync::Arc::new(super::network_error::NetworkResolver))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Error::Instance("Codex HTTP client initialization failed".into()))
}

pub(super) async fn refresh(
    issuer: &str,
    refresh_token: &str,
) -> Result<TokenRefreshResponse, Error> {
    let response = http_client()?
        .post(format!("{issuer}/oauth/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CODEX_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(super::network_error::http_setup)?;
    if !response.status().is_success() {
        return Err(Error::InvalidStatusCodeWithMessage(
            response.status(),
            "Codex credential refresh rejected; sign in again".into(),
        ));
    }
    response.json().await.map_err(|_| {
        Error::Instance("Codex credential refresh returned invalid credentials".into())
    })
}
