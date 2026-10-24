use std::convert::Infallible;
use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex, Notify};
use tokio::task::JoinSet;

use crate::codex::codex_oauth::{html_error, HTML_CANCELLED, HTML_SUCCESS};
use crate::oauth_http_utils::{html_response, parse_urlencoded_params};

pub(super) enum CallbackResult {
    Success { code: String },
    Cancelled,
    AuthError { error: String, description: String },
}

type CallbackState = Arc<Mutex<Option<oneshot::Sender<CallbackResult>>>>;

pub(super) async fn serve_until_done(
    listener: TcpListener,
    state: CallbackState,
    expected_state: String,
) -> Result<()> {
    let done = Arc::new(Notify::new());
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let state = state.clone();
                let completed = state.clone();
                let done = done.clone();
                let expected = expected_state.clone();
                connections.spawn(async move {
                    let service = service_fn(move |request| {
                        handle_request(request, state.clone(), expected.clone())
                    });
                    if let Err(error) = http1::Builder::new().keep_alive(false)
                        .serve_connection(TokioIo::new(stream), service).await {
                        tracing::debug!(%error, "OAuth callback connection closed");
                    }
                    if completed.lock().await.is_none() {
                        done.notify_one();
                    }
                });
            }
            () = done.notified() => return Ok(()),
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    }
}

async fn handle_request(
    request: Request<Incoming>,
    state: CallbackState,
    expected: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (result, status, html) = match request.uri().path() {
        "/auth/callback" => {
            let params = parse_urlencoded_params(request.uri().query().unwrap_or_default());
            if let Some(error) = params.get("error") {
                let description = params.get("error_description").unwrap_or(error).clone();
                (
                    Some(CallbackResult::AuthError {
                        error: error.clone(),
                        description: description.clone(),
                    }),
                    StatusCode::BAD_REQUEST,
                    html_error(&description),
                )
            } else {
                match (params.get("code"), params.get("state")) {
                    (Some(code), Some(received)) if *received == expected => (
                        Some(CallbackResult::Success { code: code.clone() }),
                        StatusCode::OK,
                        HTML_SUCCESS.to_owned(),
                    ),
                    (Some(_), Some(_)) => (
                        None,
                        StatusCode::BAD_REQUEST,
                        html_error("CSRF validation failed. State mismatch detected."),
                    ),
                    _ => (
                        None,
                        StatusCode::BAD_REQUEST,
                        html_error("Missing code or state parameter in callback"),
                    ),
                }
            }
        }
        "/cancel" => (
            Some(CallbackResult::Cancelled),
            StatusCode::OK,
            HTML_CANCELLED.to_owned(),
        ),
        _ => (None, StatusCode::NOT_FOUND, html_error("Not found")),
    };
    if let Some(result) = result {
        if let Some(sender) = state.lock().await.take() {
            let _ = sender.send(result);
        }
    }
    Ok(html_response(status, &html))
}
