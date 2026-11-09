//! MCP-006: the legacy (MCP 2024-11-05 HTTP+SSE) client transport.
//!
//! Feature: spec/features/mcp-legacy-sse-transport.feature
//!
//! `GET <url>` opens an event stream whose first `endpoint` event names the
//! URI every JSON-RPC message is POSTed to; the server's messages arrive as
//! `message` events. The configured headers go on the GET and every POST, and
//! an endpoint on another origin is refused so they never leave that origin.
//! Errors never carry the url or a header value.

use std::collections::HashMap;

use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT};
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::service::RoleClient;
use tokio::sync::mpsc;

/// Why the legacy SSE transport failed.
#[derive(Debug, thiserror::Error)]
pub enum LegacySseError {
    #[error("the SSE url is invalid")]
    InvalidUrl,
    #[error("the SSE request failed: {0}")]
    Request(String),
    #[error("Server requires authentication (HTTP 401)")]
    Unauthorized,
    #[error("the server answered HTTP {0}")]
    Status(u16),
    #[error("the server did not answer with an event stream")]
    NotEventStream,
    #[error("the event stream ended before its endpoint event")]
    NoEndpoint,
    #[error("the endpoint event is not a valid URI")]
    InvalidEndpoint,
    #[error("the endpoint event names another origin; credentials are not forwarded there")]
    ForeignEndpoint,
    #[error("a JSON-RPC message could not be encoded: {0}")]
    Encode(String),
}

/// A connected legacy SSE session: the message endpoint, the headers sent on
/// every POST and the reader task forwarding server messages.
pub struct LegacySseTransport {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    headers: HeaderMap,
    incoming: mpsc::Receiver<ServerJsonRpcMessage>,
    reader: tokio::task::JoinHandle<()>,
}

impl LegacySseTransport {
    /// Open the event stream at `url` with `headers` and wait for its endpoint.
    pub async fn connect(
        url: &str,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<Self, LegacySseError> {
        let url = reqwest::Url::parse(url).map_err(|_| LegacySseError::InvalidUrl)?;
        let headers: HeaderMap = headers.into_iter().collect();
        let client = reqwest::Client::new();
        let response = client
            .get(url.clone())
            .headers(headers.clone())
            .header(ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(|error| LegacySseError::Request(error.without_url().to_string()))?;
        check_status(response.status())?;
        let is_event_stream = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));
        if !is_event_stream {
            return Err(LegacySseError::NotEventStream);
        }
        let mut events = response.bytes_stream().eventsource();
        let endpoint = loop {
            match events.next().await {
                Some(Ok(event)) if event.event == "endpoint" => break event.data,
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(LegacySseError::Request(error.to_string())),
                None => return Err(LegacySseError::NoEndpoint),
            }
        };
        let endpoint = url
            .join(endpoint.trim())
            .map_err(|_| LegacySseError::InvalidEndpoint)?;
        if endpoint.origin() != url.origin() {
            return Err(LegacySseError::ForeignEndpoint);
        }
        let (tx, incoming) = mpsc::channel(32);
        let reader = tokio::spawn(async move {
            while let Some(Ok(event)) = events.next().await {
                // SSE: an event without a type is a "message" event.
                if !(event.event.is_empty() || event.event == "message") {
                    continue;
                }
                match serde_json::from_str::<ServerJsonRpcMessage>(&event.data) {
                    Ok(message) => {
                        if tx.send(message).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => tracing::warn!("MCP-006: undecodable SSE message event: {error}"),
                }
            }
        });
        Ok(Self {
            client,
            endpoint,
            headers,
            incoming,
            reader,
        })
    }
}

fn check_status(status: reqwest::StatusCode) -> Result<(), LegacySseError> {
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LegacySseError::Unauthorized);
    }
    if !status.is_success() {
        return Err(LegacySseError::Status(status.as_u16()));
    }
    Ok(())
}

impl rmcp::transport::Transport<RoleClient> for LegacySseTransport {
    type Error = LegacySseError;

    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        let request = serde_json::to_vec(&item)
            .map_err(|error| LegacySseError::Encode(error.to_string()))
            .map(|body| {
                self.client
                    .post(self.endpoint.clone())
                    .headers(self.headers.clone())
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(body)
            });
        async move {
            let response = request?
                .send()
                .await
                .map_err(|error| LegacySseError::Request(error.without_url().to_string()))?;
            check_status(response.status())
        }
    }

    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Option<ServerJsonRpcMessage>> + Send {
        self.incoming.recv()
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.reader.abort();
        self.incoming.close();
        Ok(())
    }
}

impl Drop for LegacySseTransport {
    fn drop(&mut self) {
        self.reader.abort();
    }
}
