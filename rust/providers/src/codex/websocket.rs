use super::reconnect::NetworkFailure;
use super::refreshing_client::{prepare_oauth_request, RefreshingCodexClient, TokenMode};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use rig::http_client::{Error, StreamingResponse};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_tungstenite::{
    tungstenite::{client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect(
    request: http::Request<()>,
) -> Result<
    (
        Socket,
        tokio_tungstenite::tungstenite::handshake::client::Response,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    use tokio_tungstenite::tungstenite::{error::UrlError, Error};
    let host = request
        .uri()
        .host()
        .ok_or(Error::Url(UrlError::NoHostName))?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = request
        .uri()
        .port_u16()
        .unwrap_or(if request.uri().scheme_str() == Some("wss") {
            443
        } else {
            80
        });
    let addresses = super::network_error::resolve(host, port).await?;
    let socket = tokio::net::TcpStream::connect(addresses.as_slice()).await?;
    tokio_tungstenite::client_async_tls_with_config(request, socket, None, None).await
}

#[derive(Default)]
struct State {
    socket: Option<Socket>,
    identity: Option<(String, http::HeaderMap)>,
    auth_identity: Option<super::auth_refresh::Identity>,
    generation: u64,
    active: bool,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "Codex WebSocket upgrade rejected with status {status}; selected transport was not changed"
)]
pub struct WebSocketUpgradeRejected {
    pub status: u16,
}

#[derive(Clone, Default)]
pub struct CodexWebSocketHandle(Arc<Mutex<State>>);

impl std::fmt::Debug for CodexWebSocketHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexWebSocketHandle(<private>)")
    }
}

struct Lease(CodexWebSocketHandle, u64);

impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0 .0.lock() {
            if state.generation == self.1 {
                state.active = false;
            }
        }
    }
}

pub(super) fn failure(message: &str) -> Error {
    Error::Instance(message.to_string().into())
}

impl CodexWebSocketHandle {
    pub async fn reset(&self) {
        if let Ok(mut state) = self.0.lock() {
            let generation = state.generation.wrapping_add(1);
            *state = State {
                generation,
                ..State::default()
            };
        }
    }

    pub(super) async fn send(
        &self,
        mode: &TokenMode,
        request: &http::Request<Bytes>,
        timeout: Duration,
    ) -> Result<StreamingResponse, Error> {
        let mut request = tokio::time::timeout(timeout, async {
            match mode {
                TokenMode::OAuth { token_state } => {
                    RefreshingCodexClient::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    Ok(prepare_oauth_request(request.clone(), &state))
                }
                TokenMode::ApiKey => Ok::<_, Error>(request.clone()),
            }
        })
        .await
        .map_err(|_| failure("Codex socket credential deadline exceeded"))??;
        let routing = super::routing::prepare(&mut request)?;
        let url = request.uri().to_string();
        let url = if let Some(suffix) = url.strip_prefix("https://") {
            format!("wss://{suffix}")
        } else if let Some(suffix) = url.strip_prefix("http://") {
            format!("ws://{suffix}")
        } else {
            return Err(failure("Codex socket requires an HTTP endpoint"));
        };
        let identity = (url.clone(), request.headers().clone());
        let auth_identity = request
            .extensions()
            .get::<super::auth_refresh::Identity>()
            .cloned();
        let full: Value = serde_json::from_slice(request.body())
            .map_err(|_| failure("Codex socket request is not valid JSON"))?;
        let (mut socket, lease) = {
            let mut state = self
                .0
                .lock()
                .map_err(|_| failure("Codex socket state unavailable"))?;
            if state.active {
                return Err(failure("Codex socket already has an active response"));
            }
            if state.identity.as_ref() != Some(&identity) || state.auth_identity != auth_identity {
                state.socket = None;
            }
            state.identity = Some(identity);
            state.auth_identity = auth_identity;
            state.active = true;
            (state.socket.take(), Lease(self.clone(), state.generation))
        };
        if socket.is_none() {
            let mut upgrade = url
                .into_client_request()
                .map_err(|_| failure("Codex socket URL is invalid"))?;
            for (name, value) in request.headers() {
                if name != http::header::CONTENT_LENGTH && name != http::header::HOST {
                    upgrade.headers_mut().insert(name.clone(), value.clone());
                }
            }
            super::recovery::authorize(mode, request.headers()).await?;
            super::recovery::validate_attempt(&request)?;
            match tokio::time::timeout(timeout, connect(upgrade)).await {
                Ok(Ok((connected, response))) => {
                    super::recovery::authorize(mode, request.headers()).await?;
                    if let Some(routing) = &routing {
                        routing.observe_headers(response.headers());
                    }
                    if let Some(metadata) = request
                        .extensions()
                        .get::<super::metadata::CodexMetadataHandle>()
                    {
                        metadata.apply_websocket_headers(response.headers());
                    }
                    socket = Some(connected);
                }
                Ok(Err(tokio_tungstenite::tungstenite::Error::Http(response))) => {
                    return Err(Error::Instance(Box::new(WebSocketUpgradeRejected {
                        status: response.status().as_u16(),
                    })))
                }
                Ok(Err(error)) => return Err(super::network_error::socket_setup(error)),
                Err(_) => return Err(NetworkFailure::SetupDeadline.into_http_error()),
            }
        }
        let mut socket = socket.ok_or_else(|| failure("Codex socket connection missing"))?;
        let mut wire = full;
        if let Some(object) = wire.as_object_mut() {
            object.remove("stream");
        }
        wire["type"] = Value::String("response.create".into());
        super::recovery::authorize(mode, request.headers()).await?;
        super::recovery::validate_attempt(&request)?;
        tokio::time::timeout(timeout, socket.send(Message::Text(wire.to_string().into())))
            .await
            .map_err(|_| NetworkFailure::IdleDeadline.into_http_error())?
            .map_err(super::network_error::socket_stream)?;
        let handle = self.clone();
        let mode = mode.clone();
        let stream = async_stream::try_stream! {
            let _lease = lease;
            let mut socket = socket;
            loop {
                let next = tokio::time::timeout(timeout, socket.next()).await
                    .map_err(|_| NetworkFailure::IdleDeadline.into_http_error())?;
                if let Err(error) = super::recovery::authorize(&mode, request.headers()).await {
                    handle.reset().await;
                    if let Some(routing) = request.extensions().get::<super::routing::CodexRoutingHandle>() {
                        routing.reset();
                    }
                    Err(error)?;
                }
                match next {
                    Some(Ok(Message::Text(text))) => {
                        let event: Value = serde_json::from_str(&text)
                            .map_err(|_| failure("Codex socket returned malformed JSON"))?;
                        let terminal = matches!(event["type"].as_str(), Some("response.completed" | "response.failed" | "response.incomplete" | "error"));
                        if terminal {
                            if event["type"] == "response.completed" && event["response"]["status"] == "completed" {
                                if let Ok(mut state) = handle.0.lock() {
                                    if state.generation == _lease.1 {
                                        state.socket = Some(socket);
                                    }
                                }
                            }
                            yield Bytes::from(format!("data: {text}\n\n"));
                            break;
                        }
                        yield Bytes::from(format!("data: {text}\n\n"));
                    }
                    Some(Ok(Message::Ping(data))) => {
                        tokio::time::timeout(timeout, socket.send(Message::Pong(data))).await
                            .map_err(|_| NetworkFailure::IdleDeadline.into_http_error())?
                            .map_err(super::network_error::socket_stream)?;
                    }
                    Some(Ok(Message::Pong(_))) => {},
                    Some(Err(error)) => Err(super::network_error::socket_stream(error))?,
                    Some(Ok(Message::Close(_))) | None => Err(NetworkFailure::Interrupted.into_http_error())?,
                    Some(Ok(_)) => Err(failure("Codex socket returned an unsupported message"))?,
                }
            }
        };
        http::Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .body(Box::pin(stream) as rig::http_client::sse::BoxedStream)
            .map_err(Error::Protocol)
    }
}
