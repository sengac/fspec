use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use super::form::{build_form_html, HTML_CAPTURED, HTML_SUCCESS_CLAUDE};
use super::ClaudeBrowserAuthorizationError as Error;
use crate::claude_auth::{write_claude_auth, ClaudeAuthJson};
use crate::claude_oauth::{
    calculate_expiry, exchange_authorization_code, parse_authorization_code,
};
use crate::codex::codex_oauth::{html_error, HTML_CANCELLED};
use crate::oauth_http_utils::{html_response, parse_urlencoded_params};

pub(super) enum Submission {
    Captured(String),
    Exchanged(ClaudeAuthJson),
}

#[derive(Clone)]
pub(super) enum SubmissionMode {
    Capture,
    Exchange(String),
}

type Outcome = Option<Result<Submission, Error>>;

enum ConnectionResult {
    Completed(Outcome),
    Rejected {
        error: hyper::Error,
        terminal: Outcome,
    },
}

pub(super) async fn serve(
    listener: TcpListener,
    authorize_url: String,
    verifier: String,
    mode: SubmissionMode,
) -> Result<Submission, Error> {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let authorize_url = authorize_url.clone();
                let verifier = verifier.clone();
                let mode = mode.clone();
                connections.spawn(async move {
                    let outcome = Arc::new(Mutex::new(None));
                    let result = Arc::clone(&outcome);
                    let service = service_fn(move |request| {
                        handle_request(request, Arc::clone(&outcome), authorize_url.clone(),
                            verifier.clone(), mode.clone())
                    });
                    let connection = http1::Builder::new().keep_alive(false)
                        .serve_connection(TokioIo::new(stream), service).await;
                    let submitted = result.lock().await.take();
                    match connection {
                        Ok(()) => ConnectionResult::Completed(submitted),
                        Err(error) => ConnectionResult::Rejected { error, terminal: submitted },
                    }
                });
            }
            completed = connections.join_next(), if !connections.is_empty() => {
                if let Some(completed) = completed {
                    match completed? {
                        ConnectionResult::Completed(Some(outcome)) => return outcome,
                        ConnectionResult::Completed(None) => {}
                        ConnectionResult::Rejected { error, terminal } => {
                            tracing::error!(%error, cause = ?error, "Rejected Claude OAuth HTTP connection");
                            if let Some(outcome) = terminal {
                                return outcome;
                            }
                            continue;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_request(
    request: Request<Incoming>,
    outcome: Arc<Mutex<Outcome>>,
    authorize_url: String,
    verifier: String,
    mode: SubmissionMode,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let response = match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => html_response(StatusCode::OK, &build_form_html(&authorize_url)),
        (&Method::GET, "/cancel") => {
            *outcome.lock().await = Some(Err(Error::Cancelled));
            html_response(StatusCode::OK, HTML_CANCELLED)
        }
        (&Method::POST, "/submit") => match submit(request, &verifier, mode).await {
            Ok(Some(submission)) => {
                let html = match &submission {
                    Submission::Captured(_) => HTML_CAPTURED,
                    Submission::Exchanged(_) => HTML_SUCCESS_CLAUDE,
                };
                *outcome.lock().await = Some(Ok(submission));
                html_response(StatusCode::OK, html)
            }
            Ok(None) => html_response(
                StatusCode::BAD_REQUEST,
                &html_error("No authorization code submitted"),
            ),
            Err(error) => {
                let status = if matches!(error, Error::Persistence(_)) {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::BAD_REQUEST
                };
                let html = html_error(&error.to_string());
                *outcome.lock().await = Some(Err(error));
                html_response(status, &html)
            }
        },
        _ => html_response(StatusCode::NOT_FOUND, &html_error("Not found")),
    };
    Ok(response)
}

async fn submit(
    request: Request<Incoming>,
    verifier: &str,
    mode: SubmissionMode,
) -> Result<Option<Submission>, Error> {
    let body = request.into_body().collect().await?.to_bytes();
    let params = parse_urlencoded_params(&String::from_utf8_lossy(&body));
    let raw_code = match params.get("code") {
        Some(code) if !code.trim().is_empty() => code,
        _ => return Ok(None),
    };
    let (code, state) = parse_authorization_code(raw_code);
    let state = state.ok_or(Error::MissingState)?;
    if state != verifier {
        return Err(Error::StateMismatch);
    }
    if code.is_empty() {
        return Ok(None);
    }
    match mode {
        SubmissionMode::Capture => Ok(Some(Submission::Captured(format!("{code}#{state}")))),
        SubmissionMode::Exchange(endpoint) => {
            let tokens = exchange_authorization_code(&endpoint, &code, &state, verifier)
                .await
                .map_err(|error| Error::Exchange(error.to_string()))?;
            let auth = ClaudeAuthJson {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires: calculate_expiry(tokens.expires_in),
            };
            write_claude_auth(&auth)
                .await
                .map_err(|error| Error::Persistence(error.to_string()))?;
            Ok(Some(Submission::Exchanged(auth)))
        }
    }
}
