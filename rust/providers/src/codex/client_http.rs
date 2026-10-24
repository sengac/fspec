use super::{prepare_oauth_request, RefreshingCodexClient, TokenMode};
use crate::codex::{recovery, routing};

fn client(
    result: Result<reqwest::Client, std::sync::Arc<rig::http_client::Error>>,
) -> rig::http_client::Result<reqwest::Client> {
    result.map_err(|_| {
        rig::http_client::Error::Instance("Codex HTTP client initialization failed".into())
    })
}

impl rig::http_client::HttpClientExt for RefreshingCodexClient {
    fn send<T, U>(
        &self,
        req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        let inner = self.inner.clone();
        let mode = self.mode.clone();
        let timeout = self.idle_timeout;
        let req = req.map(Into::into);

        async move {
            let inner = client(inner)?;
            let reset = req
                .extensions()
                .get::<routing::CodexRoutingHandle>()
                .cloned();
            let result = async {
                let mut req = match &mode {
                    TokenMode::OAuth { token_state } => {
                        Self::ensure_fresh_token(token_state).await?;
                        let state = token_state.read().await;
                        prepare_oauth_request(req, &state)
                    }
                    TokenMode::ApiKey => req,
                };
                let routing = routing::prepare(&mut req)?;
                recovery::authorize(&mode, req.headers()).await?;
                let redactor = Self::diagnostic_redactor(&mode, req.headers()).await;
                let (parts, body) = req.into_parts();
                let response = tokio::time::timeout(
                    timeout,
                    inner
                        .request(parts.method, parts.uri.to_string())
                        .headers(parts.headers)
                        .body(body)
                        .send(),
                )
                .await
                .map_err(|_| {
                    rig::http_client::Error::Instance(
                        "Codex network setup deadline exceeded".into(),
                    )
                })?
                .map_err(|_| {
                    rig::http_client::Error::Instance(
                        "Codex network setup failed; request was not replayed".into(),
                    )
                })?;
                if !response.status().is_success() {
                    return Err(crate::codex::diagnostics::CodexRejection::http(
                        response, &redactor, 1, timeout,
                    )
                    .await
                    .into_http_error());
                }
                if let Some(routing) = routing {
                    routing.observe_headers(response.headers());
                }
                let mut result = http::Response::builder()
                    .status(response.status())
                    .version(response.version());
                if let Some(headers) = result.headers_mut() {
                    *headers = response.headers().clone();
                }
                let body: rig::http_client::LazyBody<U> = Box::pin(async move {
                    response
                        .bytes()
                        .await
                        .map(U::from)
                        .map_err(|error| rig::http_client::Error::Instance(error.into()))
                });
                result.body(body).map_err(rig::http_client::Error::Protocol)
            }
            .await;
            if result.is_err() {
                if let Some(reset) = reset {
                    reset.reset();
                }
            }
            result
        }
    }

    fn send_multipart<U>(
        &self,
        req: http::Request<rig::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        let inner = self.inner.clone();
        let mode = self.mode.clone();

        async move {
            let reset = req
                .extensions()
                .get::<routing::CodexRoutingHandle>()
                .cloned();
            let result = async {
                let req = match &mode {
                    TokenMode::OAuth { token_state } => {
                        Self::ensure_fresh_token(token_state).await?;
                        let state = token_state.read().await;
                        prepare_oauth_request(req, &state)
                    }
                    TokenMode::ApiKey => req,
                };
                recovery::authorize(&mode, req.headers()).await?;
                client(inner)?.send_multipart(req).await
            }
            .await;
            if result.is_err() {
                if let Some(reset) = reset {
                    reset.reset();
                }
            }
            result
        }
    }

    fn send_streaming<T>(
        &self,
        req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<rig::http_client::StreamingResponse>,
    > + Send
    where
        T: Into<bytes::Bytes>,
    {
        let inner = self.inner.clone();
        let mode = self.mode.clone();
        let timeout = self.idle_timeout;
        let websocket = self.websocket.clone();
        let req = req.map(Into::into);
        async move {
            let reset = req
                .extensions()
                .get::<routing::CodexRoutingHandle>()
                .cloned();
            let result = async {
                if let Some(websocket) = &websocket {
                    return websocket.send(&mode, &req, timeout).await;
                }
                recovery::send(client(inner)?, mode, req, timeout).await
            }
            .await;
            if result.is_err() {
                if let Some(reset) = reset {
                    reset.reset();
                }
                if let Some(websocket) = websocket {
                    websocket.reset().await;
                }
            }
            result
        }
    }
}
