//! Claude Browser OAuth HTTP Server (PROV-021)
//!
//! Implements a local HTTP server that shows a form page for users to paste
//! their authorization code from Anthropic's remote callback page.
//!
//! Unlike Codex (PROV-013) which receives a direct redirect callback,
//! Anthropic's redirect_uri is remote (console.anthropic.com), so users
//! must manually copy the code#state string and paste it into a local form.
//!
//! Routes:
//! - GET /       → Form page with authorize URL link and code paste input
//! - POST /submit → Receives code, validates state, exchanges tokens
//! - GET /cancel  → Abort flow
//! - _            → 404 (does not shut down server)
//!
//! Entry point: `claude_browser_oauth_login()` / `claude_browser_oauth_login_inner()`

use std::convert::Infallible;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use crate::claude_auth::{write_claude_auth, ClaudeAuthJson};
use crate::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code, parse_authorization_code,
};
use crate::codex::codex_oauth::{html_error, HTML_CANCELLED};
use crate::oauth_crypto::{generate_pkce, PkceCodes};
use crate::oauth_http_utils::{html_response, parse_urlencoded_params};

/// Default timeout for Claude OAuth login (5 minutes)
pub const CLAUDE_OAUTH_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Result from the form submission handler
enum SubmitResult {
    /// Token exchange succeeded — contains persisted auth data
    Success(ClaudeAuthJson),
    /// User cancelled via /cancel route
    Cancelled,
    /// CSRF validation failed — state mismatch
    CsrfError { expected: String, received: String },
    /// Missing state in submitted code (no # separator)
    MissingState,
    /// Token exchange failed after valid state validation
    ExchangeError(String),
}

/// Configuration for `claude_browser_oauth_login_inner` to support both production and test use.
pub struct ClaudeOAuthServerConfig {
    /// Base URL for the token endpoint (tests point at wiremock; production uses real URL)
    pub token_endpoint_base: String,
    /// Pre-bound TCP listener (tests bind to port 0; production binds to ephemeral port)
    pub listener: TcpListener,
    /// Whether to open the browser automatically
    pub open_browser: bool,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Optional pre-generated PKCE codes (tests inject known values; production generates fresh)
    pub pkce: Option<PkceCodes>,
}

/// Public entry point: run the full Claude browser OAuth login flow.
///
/// Orchestrates: start server → generate PKCE → open browser → show form →
/// await code paste → validate state → exchange code → persist tokens →
/// stop server.
///
/// Returns `ClaudeAuthJson` on success.
pub async fn claude_browser_oauth_login() -> Result<ClaudeAuthJson> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
        anyhow!("Failed to bind Claude OAuth server: {e}")
    })?;

    let config = ClaudeOAuthServerConfig {
        token_endpoint_base: "https://console.anthropic.com".to_string(),
        listener,
        open_browser: true,
        timeout_ms: CLAUDE_OAUTH_TIMEOUT_MS,
        pkce: None,
    };

    claude_browser_oauth_login_inner(config).await
}

/// Inner implementation that accepts configuration for testability.
///
/// The listener is pre-bound so callers control the port. Tests pass a
/// port-0 listener and `open_browser: false`.
pub async fn claude_browser_oauth_login_inner(
    config: ClaudeOAuthServerConfig,
) -> Result<ClaudeAuthJson> {
    let local_addr = config.listener.local_addr()?;
    let port = local_addr.port();

    // 1. Generate PKCE (or use injected values for testing)
    //    State = verifier for Anthropic (unlike Codex which uses separate state)
    let pkce = config.pkce.unwrap_or_else(generate_pkce);
    let expected_state = pkce.verifier.clone();

    // 2. Build authorize URL
    let auth_url = build_authorize_url(&pkce);

    info!("Claude OAuth server listening on http://localhost:{port}");
    info!("Authorize URL: {auth_url}");

    // 3. Open browser (skipped in tests)
    if config.open_browser {
        let form_url = format!("http://localhost:{port}/");
        if let Err(e) = open::that(&form_url) {
            warn!(
                "Failed to open browser: {e}. Please open this URL manually:\n{form_url}"
            );
        }
    }

    // 4. Await form submission with timeout
    let (tx, rx) = oneshot::channel::<SubmitResult>();
    let server_state = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let submit_result = tokio::select! {
        result = serve_until_done(
            config.listener,
            server_state,
            expected_state.clone(),
            auth_url.clone(),
            config.token_endpoint_base.clone(),
            pkce.verifier.clone(),
        ) => {
            match result {
                Ok(()) => rx.await.map_err(|_| anyhow!("Submit channel closed without result"))?,
                Err(e) => return Err(anyhow!("Claude OAuth server error: {e}")),
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_millis(config.timeout_ms)) => {
            return Err(anyhow!(
                "Claude OAuth login timed out after {} seconds. No code submitted.",
                config.timeout_ms / 1000
            ));
        }
    };

    // 5. Process the submit result
    match submit_result {
        SubmitResult::Cancelled => Err(anyhow!("Login cancelled by user")),
        SubmitResult::MissingState => {
            Err(anyhow!("Missing state in authorization code — code must be in 'code#state' format"))
        }
        SubmitResult::CsrfError { expected, received } => {
            Err(anyhow!(
                "CSRF validation failed — state mismatch. Expected verifier: {expected}, Got: {received}"
            ))
        }
        SubmitResult::ExchangeError(msg) => Err(anyhow!("{msg}")),
        SubmitResult::Success(auth) => {
            info!("Claude OAuth login successful — tokens persisted to claude_auth.json");
            Ok(auth)
        }
    }
}

/// Accept connections until a terminal route (/submit or /cancel) fires.
async fn serve_until_done(
    listener: TcpListener,
    callback_state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<SubmitResult>>>>,
    expected_state: String,
    auth_url: String,
    token_endpoint_base: String,
    pkce_verifier: String,
) -> Result<()> {
    let done = Arc::new(tokio::sync::Notify::new());

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _addr) = accept_result?;
                let io = TokioIo::new(stream);
                let state_clone = Arc::clone(&callback_state);
                let done_clone = Arc::clone(&done);
                let expected_state_clone = expected_state.clone();
                let auth_url_clone = auth_url.clone();
                let token_base_clone = token_endpoint_base.clone();
                let verifier_clone = pkce_verifier.clone();

                let svc = service_fn(move |req: Request<Incoming>| {
                    let st = Arc::clone(&state_clone);
                    let dn = Arc::clone(&done_clone);
                    let exp_state = expected_state_clone.clone();
                    let url = auth_url_clone.clone();
                    let token_base = token_base_clone.clone();
                    let verifier = verifier_clone.clone();
                    async move {
                        handle_request(req, st, dn, exp_state, url, token_base, verifier).await
                    }
                });

                tokio::spawn(async move {
                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, svc)
                        .await
                    {
                        error!("HTTP connection error: {e}");
                    }
                });
            }
            () = done.notified() => {
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single HTTP request.
///
/// The /submit handler performs the full flow: parse → validate state →
/// exchange code for tokens → persist. This means the user sees
/// a success or error page that reflects the *actual* outcome.
async fn handle_request(
    req: Request<Incoming>,
    state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<SubmitResult>>>>,
    done: Arc<tokio::sync::Notify>,
    expected_state: String,
    auth_url: String,
    token_endpoint_base: String,
    pkce_verifier: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        (hyper::Method::GET, "/") => {
            let form_html = build_form_html(&auth_url);
            Ok(html_response(StatusCode::OK, &form_html))
        }
        (hyper::Method::POST, "/submit") => {
            handle_submit(
                req, state, done, expected_state, token_endpoint_base, pkce_verifier,
            )
            .await
        }
        (hyper::Method::GET, "/cancel") => {
            send_result_and_notify(&state, &done, SubmitResult::Cancelled).await;
            Ok(html_response(StatusCode::OK, HTML_CANCELLED))
        }
        _ => {
            let html = html_error("Not found");
            Ok(html_response(StatusCode::NOT_FOUND, &html))
        }
    }
}

/// Handle the POST /submit route: parse code, validate state, exchange, persist.
async fn handle_submit(
    req: Request<Incoming>,
    state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<SubmitResult>>>>,
    done: Arc<tokio::sync::Notify>,
    expected_state: String,
    token_endpoint_base: String,
    pkce_verifier: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let body_bytes = read_body(req).await;
    let body_str = String::from_utf8_lossy(&body_bytes);
    let params = parse_urlencoded_params(&body_str);
    let raw_code = params.get("code").cloned().unwrap_or_default();

    if raw_code.is_empty() {
        let html = html_error("No authorization code submitted");
        return Ok(html_response(StatusCode::BAD_REQUEST, &html));
    }

    // Parse code#state format
    let (code, maybe_state) = parse_authorization_code(&raw_code);

    let received_state = match maybe_state {
        None => {
            send_result_and_notify(&state, &done, SubmitResult::MissingState).await;
            let html = html_error(
                "Missing state in authorization code. The code should be in 'code#state' format.",
            );
            return Ok(html_response(StatusCode::BAD_REQUEST, &html));
        }
        Some(s) => s,
    };

    if received_state != expected_state {
        send_result_and_notify(
            &state,
            &done,
            SubmitResult::CsrfError {
                expected: expected_state,
                received: received_state,
            },
        )
        .await;
        let html =
            html_error("CSRF validation failed — state mismatch detected. Please try again.");
        return Ok(html_response(StatusCode::BAD_REQUEST, &html));
    }

    // State validated — exchange code for tokens
    let exchange_result = exchange_authorization_code(
        &token_endpoint_base,
        &code,
        &received_state,
        &pkce_verifier,
    )
    .await;

    match exchange_result {
        Err(e) => {
            let msg = format!("{e}");
            send_result_and_notify(
                &state,
                &done,
                SubmitResult::ExchangeError(msg.clone()),
            )
            .await;
            let html = html_error(&format!("Token exchange failed: {msg}"));
            Ok(html_response(StatusCode::BAD_REQUEST, &html))
        }
        Ok(token_response) => {
            let expires = calculate_expiry(token_response.expires_in);
            let auth = ClaudeAuthJson {
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                expires,
            };

            if let Err(e) = write_claude_auth(&auth).await {
                let msg = format!("Failed to persist tokens: {e}");
                send_result_and_notify(
                    &state,
                    &done,
                    SubmitResult::ExchangeError(msg.clone()),
                )
                .await;
                let html = html_error(&msg);
                return Ok(html_response(StatusCode::INTERNAL_SERVER_ERROR, &html));
            }

            send_result_and_notify(&state, &done, SubmitResult::Success(auth)).await;
            Ok(html_response(StatusCode::OK, HTML_SUCCESS_CLAUDE))
        }
    }
}

/// Send a `SubmitResult` through the oneshot channel and notify the server to shut down.
async fn send_result_and_notify(
    state: &Arc<tokio::sync::Mutex<Option<oneshot::Sender<SubmitResult>>>>,
    done: &Arc<tokio::sync::Notify>,
    result: SubmitResult,
) {
    {
        let mut guard = state.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(result);
        }
    }
    done.notify_one();
}

/// Read the full request body into bytes
async fn read_body(req: Request<Incoming>) -> Vec<u8> {
    match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes().to_vec(),
        Err(_) => Vec::new(),
    }
}

/// Build the form HTML page for code paste entry.
///
/// Uses relative URLs for form action and cancel link so the page works
/// regardless of host header or proxy configuration.
fn build_form_html(auth_url: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>fspec - Claude OAuth Login</title>
    <style>
      body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }}
      .container {{ text-align: center; padding: 2rem; max-width: 600px; }}
      h1 {{ color: #f1ecec; margin-bottom: 1rem; }}
      p {{ color: #b7b1b1; margin-bottom: 1.5rem; }}
      a {{ color: #7ab5ff; text-decoration: none; word-break: break-all; }}
      a:hover {{ text-decoration: underline; }}
      .form-group {{ margin: 1.5rem 0; }}
      input[type="text"] {{ width: 100%; padding: 0.75rem; border: 1px solid #444; border-radius: 0.5rem; background: #1a1717; color: #f1ecec; font-size: 1rem; box-sizing: border-box; font-family: monospace; }}
      input[type="text"]:focus {{ outline: none; border-color: #7ab5ff; }}
      button {{ padding: 0.75rem 2rem; border: none; border-radius: 0.5rem; background: #2563eb; color: white; font-size: 1rem; cursor: pointer; margin-top: 0.5rem; }}
      button:hover {{ background: #1d4ed8; }}
      .cancel {{ margin-top: 1rem; }}
      .cancel a {{ color: #888; font-size: 0.9rem; }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Claude OAuth Login</h1>
      <p>1. Click the link below to authorize with Claude:</p>
      <p><a href="{auth_url}" target="_blank">{auth_url}</a></p>
      <p>2. After authorizing, copy the code from the callback page and paste it below:</p>
      <form method="POST" action="/submit">
        <div class="form-group">
          <input type="text" name="code" placeholder="Paste authorization code here (code#state)" autofocus />
        </div>
        <button type="submit">Submit Code</button>
      </form>
      <div class="cancel">
        <a href="/cancel">Cancel login</a>
      </div>
    </div>
  </body>
</html>"#
    )
}

/// HTML page shown on successful Claude OAuth authorization
const HTML_SUCCESS_CLAUDE: &str = r#"<!doctype html>
<html>
  <head>
    <title>fspec - Authorization Successful</title>
    <style>
      body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #f1ecec; margin-bottom: 1rem; }
      p { color: #b7b1b1; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to fspec.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000)</script>
  </body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_form_html_contains_auth_url() {
        let html = build_form_html("https://claude.ai/oauth/authorize?test=1");
        assert!(html.contains("https://claude.ai/oauth/authorize?test=1"));
        assert!(html.contains("href="));
        assert!(html.contains("<form"));
    }

    #[test]
    fn test_build_form_html_uses_relative_urls() {
        let html = build_form_html("https://example.com");
        assert!(
            html.contains(r#"action="/submit""#),
            "Form action should be relative: {html}"
        );
        assert!(
            html.contains(r#"href="/cancel""#),
            "Cancel link should be relative: {html}"
        );
        assert!(
            !html.contains("localhost"),
            "Should not contain localhost in URLs: {html}"
        );
    }
}
