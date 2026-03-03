//! Browser OAuth HTTP Server for PKCE Callback (PROV-013)
//!
//! Implements a local HTTP server on port 1455 that handles the OAuth callback
//! during browser-based PKCE login. The server orchestrates the full flow:
//!
//! 1. Binds to port 1455 using hyper
//! 2. Generates PKCE + state (codex_oauth.rs)
//! 3. Opens browser to authorize URL (open crate)
//! 4. Waits for callback with 5-min timeout
//! 5. Validates state at HTTP layer (CSRF protection)
//! 6. Exchanges code for tokens (exchange_authorization_code)
//! 7. Extracts account_id from JWT (extract_account_id)
//! 8. Persists tokens to auth.json (write_codex_auth)
//! 9. Returns CodexTokens to caller
//!
//! Routes: /auth/callback (main), /cancel (abort), 404 for everything else.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{error, info};

use super::codex_auth::{write_codex_auth, CodexAuthJson, CodexTokens};
use super::codex_oauth::{
    build_authorize_url, exchange_authorization_code, extract_account_id, generate_pkce,
    generate_state, html_error, PkceCodes, CODEX_ISSUER, HTML_CANCELLED,
    HTML_SUCCESS, OAUTH_PORT, OAUTH_TIMEOUT_MS,
};
use crate::oauth_http_utils::{html_response, parse_urlencoded_params};

/// Result from the OAuth callback — success with code+state, cancellation, or error
enum CallbackResult {
    /// Successful callback with authorization code and state (state already validated at HTTP layer)
    Success { code: String, _state: String },
    /// User cancelled via /cancel route
    Cancelled,
    /// Authorization server returned an error (e.g. access_denied)
    AuthError { error: String, description: String },
    /// CSRF validation failed - state mismatch
    CsrfError { expected: String, received: String },
}

/// Configuration for `browser_oauth_login_inner` to support both production and test use.
pub struct OAuthServerConfig {
    /// The issuer URL (e.g. "https://auth.openai.com" or a wiremock URL)
    pub issuer_url: String,
    /// Pre-bound TCP listener (tests bind to port 0; production binds to OAUTH_PORT)
    pub listener: TcpListener,
    /// Whether to open the browser automatically
    pub open_browser: bool,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Optional pre-generated PKCE codes (tests inject known values; production generates fresh)
    pub pkce: Option<PkceCodes>,
    /// Optional pre-generated state parameter (tests inject known values; production generates fresh)
    pub state: Option<String>,
}

/// Public entry point: run the full browser OAuth login flow.
///
/// Orchestrates: start server → generate PKCE/state → open browser → await
/// callback → validate state → exchange code → extract account_id → persist
/// tokens → stop server.
///
/// Returns `CodexTokens` on success.
pub async fn browser_oauth_login() -> Result<CodexTokens> {
    let addr = SocketAddr::from(([127, 0, 0, 1], OAUTH_PORT));
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        anyhow!(
            "Failed to bind OAuth server to port {OAUTH_PORT}: {e}. \
             Is port {OAUTH_PORT} already in use?"
        )
    })?;

    let config = OAuthServerConfig {
        issuer_url: CODEX_ISSUER.to_string(),
        listener,
        open_browser: true,
        timeout_ms: OAUTH_TIMEOUT_MS,
        pkce: None,
        state: None,
    };

    browser_oauth_login_inner(config).await
}

/// Inner implementation that accepts configuration for testability.
///
/// The listener is pre-bound so callers control the port. Tests pass a
/// port-0 listener and `open_browser: false`.
pub async fn browser_oauth_login_inner(config: OAuthServerConfig) -> Result<CodexTokens> {
    let local_addr = config.listener.local_addr()?;
    let port = local_addr.port();

    // 1. Generate PKCE + state (or use injected values for testing)
    let pkce = config.pkce.unwrap_or_else(generate_pkce);
    let expected_state = config.state.unwrap_or_else(generate_state);
    let redirect_uri = format!("http://localhost:{port}/auth/callback");

    // 2. Build authorize URL
    let auth_url = build_authorize_url(&redirect_uri, &pkce, &expected_state);

    info!("OAuth callback server listening on http://localhost:{port}");

    // 3. Open browser (skipped in tests)
    if config.open_browser {
        if let Err(e) = open::that(&auth_url) {
            error!("Failed to open browser: {e}. Please open this URL manually:\n{auth_url}");
        }
    }

    // 4. Await callback with timeout
    let (tx, rx) = oneshot::channel::<CallbackResult>();
    let server_state = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let callback_result = tokio::select! {
        result = serve_until_done(config.listener, server_state, expected_state.clone()) => {
            match result {
                Ok(()) => rx.await.map_err(|_| anyhow!("Callback channel closed without result"))?,
                Err(e) => return Err(anyhow!("OAuth server error: {e}")),
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_millis(config.timeout_ms)) => {
            return Err(anyhow!(
                "OAuth login timed out after {} seconds. No callback received.",
                config.timeout_ms / 1000
            ));
        }
    };

    // 5. Process the callback result
    match callback_result {
        CallbackResult::Cancelled => Err(anyhow!("Login cancelled by user")),
        CallbackResult::AuthError { error, description } => {
            Err(anyhow!("Authorization failed: {error} - {description}"))
        }
        CallbackResult::CsrfError { expected, received } => {
            Err(anyhow!(
                "Invalid state parameter - potential CSRF attack. Expected: {expected}, Got: {received}"
            ))
        }
        CallbackResult::Success { code, _state: _ } => {
            // State was already validated at the HTTP layer

            // 6. Exchange code for tokens
            let token_response = exchange_authorization_code(
                &config.issuer_url,
                &code,
                &pkce.verifier,
                Some(&redirect_uri),
            )
            .await?;

            // 7. Extract account_id from JWT
            let account_id = extract_account_id(
                Some(&token_response.id_token),
                Some(&token_response.access_token),
            )
            .ok_or_else(|| anyhow!("Could not extract account_id from token response"))?;

            // 8. Build CodexTokens and persist
            let tokens = CodexTokens {
                id_token: token_response.id_token,
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                account_id,
            };

            let auth = CodexAuthJson {
                openai_api_key: None,
                tokens: Some(tokens.clone()),
                last_refresh: None,
            };

            write_codex_auth(&auth)?;

            info!("OAuth login successful — tokens persisted to auth.json");
            Ok(tokens)
        }
    }
}

/// Accept connections until a terminal route (/auth/callback or /cancel) fires.
///
/// Each connection is spawned independently so that non-terminal requests
/// (e.g. /favicon.ico, pre-connect) never block the accept loop. A shared
/// `done` flag tells the loop when to stop.
async fn serve_until_done(
    listener: TcpListener,
    callback_state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<CallbackResult>>>>,
    expected_state: String,
) -> Result<()> {
    // Shared flag: set to true when a terminal route fires
    let done = Arc::new(tokio::sync::Notify::new());

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _addr) = accept_result?;
                let io = TokioIo::new(stream);
                let state_clone = Arc::clone(&callback_state);
                let done_clone = Arc::clone(&done);
                let expected_state_clone = expected_state.clone();

                let svc = service_fn(move |req: Request<Incoming>| {
                    let st = Arc::clone(&state_clone);
                    let dn = Arc::clone(&done_clone);
                    let exp_state = expected_state_clone.clone();
                    async move { handle_request(req, st, dn, exp_state).await }
                });

                // Spawn connection — never blocks the accept loop
                tokio::spawn(async move {
                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, svc)
                        .await
                    {
                        // Connection errors are expected when browser closes early
                        error!("HTTP connection error: {e}");
                    }
                });
            }
            // A terminal route notified us — stop accepting
            () = done.notified() => {
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single HTTP request
async fn handle_request(
    req: Request<Incoming>,
    state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<CallbackResult>>>>,
    done: Arc<tokio::sync::Notify>,
    expected_state: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or_default().to_string();

    match path.as_str() {
        "/auth/callback" => {
            let params = parse_urlencoded_params(&query);
            let code = params.get("code").cloned();
            let callback_state = params.get("state").cloned();
            let error_param = params.get("error").cloned();

            // Check for error from the authorization server (e.g. access_denied)
            if let Some(err) = error_param {
                let error_desc = params
                    .get("error_description")
                    .cloned()
                    .unwrap_or_else(|| err.clone());
                
                // Send error through channel BEFORE notifying done
                {
                    let mut guard = state.lock().await;
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(CallbackResult::AuthError {
                            error: err.clone(),
                            description: error_desc.clone(),
                        });
                    }
                }
                
                done.notify_one();
                let html = html_error(&error_desc);
                return Ok(html_response(StatusCode::BAD_REQUEST, &html));
            }

            match (code, callback_state) {
                (Some(code), Some(received_state)) => {
                    // Validate state at HTTP layer for proper UX
                    if received_state != expected_state {
                        // Send CSRF error through channel BEFORE notifying done
                        {
                            let mut guard = state.lock().await;
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(CallbackResult::CsrfError {
                                    expected: expected_state.clone(),
                                    received: received_state.clone(),
                                });
                            }
                        }
                        
                        done.notify_one();
                        let html = html_error(
                            "CSRF validation failed. State mismatch detected."
                        );
                        return Ok(html_response(StatusCode::BAD_REQUEST, &html));
                    }

                    // Send success through channel BEFORE notifying done
                    {
                        let mut guard = state.lock().await;
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(CallbackResult::Success {
                                code,
                                _state: received_state,
                            });
                        }
                    }
                    
                    done.notify_one();
                    Ok(html_response(StatusCode::OK, HTML_SUCCESS))
                }
                _ => {
                    // Missing required parameters - send nothing through channel,
                    // but also don't notify done so server stays alive for retry
                    let html = html_error("Missing code or state parameter in callback");
                    Ok(html_response(StatusCode::BAD_REQUEST, &html))
                }
            }
        }
        "/cancel" => {
            // Send cancellation through channel BEFORE notifying done
            {
                let mut guard = state.lock().await;
                if let Some(tx) = guard.take() {
                    let _ = tx.send(CallbackResult::Cancelled);
                }
            }
            
            done.notify_one();
            Ok(html_response(StatusCode::OK, HTML_CANCELLED))
        }
        _ => {
            // 404 for everything else — don't shut down the server
            let html = html_error("Not found");
            Ok(html_response(StatusCode::NOT_FOUND, &html))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::oauth_http_utils::{html_response, parse_urlencoded_params, urlencoded_decode};
    use hyper::StatusCode;

    #[test]
    fn test_parse_query_params_basic() {
        let params = parse_urlencoded_params("code=abc123&state=xyz");
        assert_eq!(params.get("code"), Some(&"abc123".to_string()));
        assert_eq!(params.get("state"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_parse_query_params_empty() {
        let params = parse_urlencoded_params("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_parse_query_params_encoded() {
        let params = parse_urlencoded_params("redirect_uri=http%3A%2F%2Flocalhost%3A1455");
        assert_eq!(
            params.get("redirect_uri"),
            Some(&"http://localhost:1455".to_string())
        );
    }

    #[test]
    fn test_urlencoded_decode_plus_as_space() {
        assert_eq!(urlencoded_decode("hello+world"), "hello world");
    }

    #[test]
    fn test_html_response_sets_content_type() {
        let resp = html_response(StatusCode::OK, "<h1>Test</h1>");
        assert_eq!(
            resp.headers()
                .get("Content-Type")
                .map(|v| v.to_str().unwrap_or_default()),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
