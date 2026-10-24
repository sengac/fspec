use super::auth_refresh;
use super::auth_storage::{self, AuthError};
use super::codex_oauth::{TokenRefreshResponse, CODEX_CLIENT_ID};
use serde::Deserialize;

#[derive(Deserialize)]
struct ExchangeResponse {
    access_token: String,
}

pub(super) fn api_key() -> Result<String, AuthError> {
    let auth = auth_storage::read_codex_auth()?.ok_or(AuthError::Changed)?;
    if let Some(api_key) = auth.openai_api_key {
        return Ok(api_key);
    }
    let tokens = auth.tokens.ok_or(AuthError::Changed)?;
    let mut state = super::credential_refresh::state(&tokens.refresh_token)?;
    let binding = state
        .binding
        .clone()
        .map_err(|_| AuthError::Changed)?
        .ok_or(AuthError::Changed)?;
    let rotation = auth_storage::lock(&binding.home, ".fspec-refresh.lock")?;
    rotation.lock()?;
    let document = auth_refresh::current(&state, &binding)?;
    auth_refresh::reload(&mut state, &document)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(network_error)?;
    if tokio::time::Instant::now() + std::time::Duration::from_secs(30) >= state.expires_at {
        let response: TokenRefreshResponse = client
            .post(format!("{}/oauth/token", state.issuer_url))
            .form(&[
                ("client_id", CODEX_CLIENT_ID),
                ("grant_type", "refresh_token"),
                ("refresh_token", state.refresh_token.as_str()),
                ("scope", "openid profile email offline_access"),
            ])
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(network_error)?
            .json()
            .map_err(network_error)?;
        auth_storage::transaction(&binding.home, || {
            auth_refresh::persist(&state, Some(&binding), &state.refresh_token, &response)
        })?;
        state.access_token = response.access_token;
        state.refresh_token = response.refresh_token;
    }
    let document = auth_refresh::current(&state, &binding)?;
    let id_token = document["tokens"]["id_token"]
        .as_str()
        .ok_or(AuthError::Changed)?;
    let exchange: ExchangeResponse = client
        .post(format!("{}/oauth/token", state.issuer_url))
        .form(&[
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", id_token),
            (
                "subject_token_type",
                "urn:ietf:params:oauth:token-type:id_token",
            ),
        ])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(network_error)?
        .json()
        .map_err(network_error)?;
    auth_storage::transaction(&binding.home, || {
        let mut document = auth_refresh::current(&state, &binding)?;
        if document["tokens"]["refresh_token"] != state.refresh_token {
            return Err(AuthError::Changed);
        }
        document["OPENAI_API_KEY"] = exchange.access_token.clone().into();
        auth_storage::replace(&binding.home, &document)
    })?;
    Ok(exchange.access_token)
}

fn network_error(_: reqwest::Error) -> AuthError {
    AuthError::OAuth("Credential exchange failed; sign in again".to_owned())
}
