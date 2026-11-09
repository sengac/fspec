use super::codex_auth::CodexTokens;
use super::codex_browser_authorization::BROWSER_REDIRECT_URI;
use super::codex_oauth::{exchange_authorization_code, extract_account_id};

pub struct BrowserLoginStart {
    pub authorize_url: String,
    pub pkce_verifier: String,
}

pub async fn start_browser_login() -> Result<BrowserLoginStart, BrowserCompletionError> {
    Ok(super::browser_session::start().await?)
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserCompletionError {
    #[error("missing code or state — completion must be 'code#state'")]
    MissingCodeOrState,
    #[error("CSRF validation failed — state mismatch")]
    StateMismatch,
    #[error("Codex credential lifecycle failed: {0}")]
    Storage(#[from] super::auth_storage::AuthError),
    #[error("Codex token exchange failed: {0}")]
    Exchange(String),
    #[error("Could not extract account_id from Codex token response")]
    MissingAccount,
    #[error("Codex login cancelled")]
    Cancelled,
    #[error("Codex login timed out")]
    Timeout,
}

pub async fn complete_browser_login(
    code_with_state: &str,
    verifier: &str,
) -> Result<(), BrowserCompletionError> {
    let (code, state) = code_with_state
        .split_once('#')
        .filter(|(code, state)| !code.is_empty() && !state.is_empty())
        .ok_or(BrowserCompletionError::MissingCodeOrState)?;
    if state != verifier {
        return Err(BrowserCompletionError::StateMismatch);
    }
    let pending = super::browser_session::take(verifier)?;
    let attempt = &pending.attempt;
    let deadline = pending.deadline;
    let issuer = &pending.issuer;
    tokio::select! {
        biased;
        () = attempt.cancelled() => Err(BrowserCompletionError::Cancelled),
        result = tokio::time::timeout_at(deadline, async {
            let response = exchange_authorization_code(
                issuer, code, verifier, Some(BROWSER_REDIRECT_URI),
            ).await.map_err(|error| BrowserCompletionError::Exchange(error.to_string()))?;
            let account_id = extract_account_id(Some(&response.id_token), Some(&response.access_token))
                .ok_or(BrowserCompletionError::MissingAccount)?;
            let tokens = CodexTokens {
                id_token: response.id_token,
                access_token: response.access_token,
                refresh_token: response.refresh_token,
                account_id,
            };
            attempt.commit_with_expiry(&tokens, response.expires_in).await?;
            Ok(())
        }) => result.map_err(|_| BrowserCompletionError::Timeout)?,
    }
}
