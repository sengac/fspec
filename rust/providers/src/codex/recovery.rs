use super::reconnect::NetworkFailure;
use super::refreshing_client::{prepare_oauth_request, RefreshingCodexClient, TokenMode};
use bytes::Bytes;
use futures::StreamExt;
use rig::http_client::{Error, StreamingResponse};
use std::time::Duration;

pub(super) const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RETRIES: usize = 2;
const MAX_BACKOFF: Duration = Duration::from_secs(30);

#[cfg(test)]
#[path = "recovery_authority_tests.rs"]
mod authority_tests;

#[derive(Clone, Debug)]
pub(super) struct AttemptAuthority {
    pub history: super::history::CodexHistory,
    pub generation: u64,
}

pub(super) fn validate_attempt<T>(request: &http::Request<T>) -> Result<(), Error> {
    if let Some(authority) = request.extensions().get::<AttemptAuthority>() {
        authority
            .history
            .ensure_generation(authority.generation)
            .map_err(|error| Error::Instance(Box::new(error)))?;
    }
    Ok(())
}

fn failure(message: &str) -> Error {
    Error::Instance(message.to_owned().into())
}

pub(super) async fn authorize(mode: &TokenMode, headers: &http::HeaderMap) -> Result<(), Error> {
    if let TokenMode::OAuth { token_state } = mode {
        let state = token_state.read().await;
        super::auth_refresh::validate(&state).map_err(|error| failure(&error.to_string()))?;
        if headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            != Some(format!("Bearer {}", state.access_token).as_str())
            || headers
                .get("chatgpt-account-id")
                .and_then(|value| value.to_str().ok())
                != Some(state.account_id.as_str())
        {
            return Err(failure("Codex authentication changed; sign in again"));
        }
    }
    Ok(())
}

fn retry_delay(headers: &http::HeaderMap, attempt: usize) -> Duration {
    let advised = headers
        .get(http::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .parse::<u64>()
                .ok()
                .map(Duration::from_secs)
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc2822(value)
                        .ok()
                        .map(|date| {
                            (date.with_timezone(&chrono::Utc) - chrono::Utc::now())
                                .to_std()
                                .unwrap_or_default()
                        })
                })
        });
    advised
        .unwrap_or_else(|| Duration::from_millis(250 * (1 << attempt)))
        .min(MAX_BACKOFF)
}

pub(super) async fn send(
    inner: reqwest::Client,
    mode: TokenMode,
    request: http::Request<Bytes>,
    idle_timeout: Duration,
) -> Result<StreamingResponse, Error> {
    let mut refreshed = false;
    let mut retries = 0;
    let mut attempts = 0;
    loop {
        let prepared = tokio::time::timeout(idle_timeout, async {
            match &mode {
                TokenMode::OAuth { token_state } => {
                    RefreshingCodexClient::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    Ok(prepare_oauth_request(request.clone(), &state))
                }
                TokenMode::ApiKey => Ok::<_, Error>(request.clone()),
            }
        })
        .await
        .map_err(|_| failure("Codex credential setup deadline exceeded"))??;
        let mut prepared = prepared;
        let routing = super::routing::prepare(&mut prepared)?;
        authorize(&mode, prepared.headers()).await?;
        let rejected_token = prepared.headers().get(http::header::AUTHORIZATION).cloned();
        let redactor = RefreshingCodexClient::diagnostic_redactor(&mode, prepared.headers()).await;
        validate_attempt(&prepared)?;
        let (parts, body) = prepared.into_parts();
        attempts += 1;
        let response = tokio::time::timeout(
            idle_timeout,
            inner
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(body)
                .send(),
        )
        .await
        .map_err(|_| NetworkFailure::SetupDeadline.into_http_error())?
        .map_err(super::network_error::http_setup)?;
        let status = response.status();
        if status.is_success() {
            if let Some(metadata) = request
                .extensions()
                .get::<super::metadata::CodexMetadataHandle>()
            {
                metadata.apply_http_headers(response.headers());
            }
            if let Some(routing) = routing {
                routing.observe_headers(response.headers());
            }
        }
        if status == http::StatusCode::UNAUTHORIZED && !refreshed {
            if let TokenMode::OAuth { token_state } = &mode {
                refreshed = true;
                drop(response);
                tokio::time::timeout(idle_timeout, async {
                    let mut state = token_state.write().await;
                    let current = format!("Bearer {}", state.access_token);
                    if rejected_token
                        .as_ref()
                        .and_then(|value| value.to_str().ok())
                        == Some(current.as_str())
                    {
                        state.expires_at = tokio::time::Instant::now();
                    }
                    drop(state);
                    RefreshingCodexClient::ensure_fresh_token(token_state).await
                })
                .await
                .map_err(|_| failure("Codex credential refresh deadline exceeded"))?
                .map_err(|_| failure("Codex credential refresh failed; sign in again"))?;
                continue;
            }
        }
        if matches!(status.as_u16(), 429 | 502 | 503 | 504) && retries < MAX_RETRIES {
            let delay = retry_delay(response.headers(), retries);
            retries += 1;
            drop(response);
            tokio::time::sleep(delay).await;
            continue;
        }
        if !status.is_success() {
            return Err(super::diagnostics::CodexRejection::http(
                response,
                &redactor,
                attempts,
                idle_timeout,
            )
            .await
            .into_http_error());
        }
        let mut result = http::Response::builder()
            .status(status)
            .version(response.version());
        if let Some(headers) = result.headers_mut() {
            *headers = response.headers().clone();
        }
        let mut network = response.bytes_stream();
        let stream = async_stream::try_stream! {
            loop {
                let next = tokio::time::timeout(idle_timeout, network.next()).await
                    .map_err(|_| NetworkFailure::IdleDeadline.into_http_error())?;
                match next {
                    Some(Ok(bytes)) => yield bytes,
                    Some(Err(_)) => Err(NetworkFailure::Interrupted.into_http_error())?,
                    None => break,
                }
            }
        };
        return result
            .body(Box::pin(stream) as rig::http_client::sse::BoxedStream)
            .map_err(Error::Protocol);
    }
}
