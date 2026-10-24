use super::auth_refresh;
use super::auth_storage::{self, AuthError};
use super::codex_auth::CodexTokens;
use super::codex_oauth::CODEX_ISSUER;
use super::refreshing_client::{token_expiry, TokenState};

pub(super) fn state(refresh_token: &str) -> Result<TokenState, AuthError> {
    let tokens = auth_storage::read_codex_auth()?
        .and_then(|auth| auth.tokens)
        .ok_or(AuthError::Changed)?;
    if tokens.refresh_token != refresh_token {
        return Err(AuthError::Changed);
    }
    let binding = auth_refresh::bind(&tokens.access_token, &tokens.refresh_token)?
        .ok_or(AuthError::Changed)?;
    let issuer_url = CODEX_ISSUER.to_owned();
    #[cfg(feature = "test-support")]
    let issuer_url = super::oauth_test_support::config().map_or(issuer_url, |config| config.issuer);
    let lifetime = auth_refresh::stored_lifetime(&tokens.access_token)?;
    Ok(TokenState {
        binding: Ok(Some(binding)),
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        account_id: tokens.account_id,
        expires_at: token_expiry(lifetime),
        issuer_url,
    })
}

pub async fn refresh_codex_tokens(refresh_token: &str) -> Result<CodexTokens, AuthError> {
    let mut state = state(refresh_token)?;
    auth_refresh::ensure(&mut state)
        .await
        .map_err(|error| AuthError::OAuth(error.to_string()))?;
    let binding = state
        .binding
        .as_ref()
        .map_err(|_| AuthError::Changed)?
        .as_ref()
        .ok_or(AuthError::Changed)?;
    let document = auth_refresh::current(&state, binding)?;
    Ok(serde_json::from_value(document["tokens"].clone())?)
}
