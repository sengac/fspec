use eventsource_stream::Eventsource;
use futures::StreamExt;
use rig::{
    completion::CompletionError,
    http_client::{self, StreamingResponse},
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct Capture {
    replay: Option<Value>,
    candidate: Arc<Mutex<Option<Value>>>,
    binding: Arc<Mutex<Option<String>>>,
    expected: Option<String>,
    model: String,
    deployment: String,
    authorized: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl Capture {
    pub(crate) fn new(authorized: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        Self {
            replay: None,
            candidate: Arc::default(),
            binding: Arc::default(),
            expected: None,
            model: String::new(),
            deployment: String::new(),
            authorized,
        }
    }

    pub(crate) fn configure(
        &mut self,
        replay: Option<Value>,
        binding: String,
        model: &str,
        deployment: &str,
    ) {
        self.replay = replay;
        self.binding = Arc::new(Mutex::new(Some(binding.clone())));
        self.expected = Some(binding);
        self.model = model.into();
        self.deployment = deployment.into();
    }

    pub(crate) fn identity(&self) -> (&str, &str) {
        (&self.model, &self.deployment)
    }

    pub(crate) fn bind_request(&self, current: String) -> http_client::Result<()> {
        let mut binding = self.binding.lock().map_err(|_| {
            http_error(std::io::Error::other("Copilot request binding unavailable"))
        })?;
        if binding.as_ref() != Some(&current) {
            *self.candidate.lock().map_err(|_| {
                http_error(std::io::Error::other(
                    "Copilot response capture unavailable",
                ))
            })? = None;
        }
        *binding = Some(current);
        Ok(())
    }

    pub(crate) fn candidate(&self) -> Result<Option<(String, Value)>, CompletionError> {
        let opaque = self
            .candidate
            .lock()
            .map(|value| value.clone())
            .map_err(|_| {
                CompletionError::ResponseError("Copilot response capture unavailable".into())
            })?;
        let binding = self
            .binding
            .lock()
            .map_err(|_| {
                CompletionError::ResponseError("Copilot request binding unavailable".into())
            })?
            .clone();
        Ok(binding.zip(opaque))
    }

    pub(crate) fn request(&self, bytes: bytes::Bytes) -> http_client::Result<bytes::Bytes> {
        if !(self.authorized)() {
            return Err(http_error(std::io::Error::other(
                "Copilot request superseded",
            )));
        }
        let mut body: Value = serde_json::from_slice(&bytes).map_err(http_error)?;
        if let Some(body) = body.as_object_mut() {
            body.remove("reasoning_opaque");
            let binding = self.binding.lock().map_err(|_| {
                http_error(std::io::Error::other("Copilot request binding unavailable"))
            })?;
            if *binding == self.expected {
                if let Some(opaque) = &self.replay {
                    body.insert("reasoning_opaque".into(), opaque.clone());
                }
            }
        }
        serde_json::to_vec(&body)
            .map(Into::into)
            .map_err(http_error)
    }

    pub(crate) fn response(&self, mut value: Value, terminal: bool) -> http_client::Result<Value> {
        let valid = terminal
            && value["status"] == "completed"
            && value["id"].as_str().is_some_and(|id| !id.is_empty())
            && value["error"].is_null();
        if let Some(body) = value.as_object_mut() {
            let opaque = body.remove("reasoning_opaque");
            if valid {
                *self.candidate.lock().map_err(|_| {
                    http_error(std::io::Error::other(
                        "Copilot response capture unavailable",
                    ))
                })? = opaque;
            }
        }
        Ok(value)
    }

    pub(crate) fn stream(&self, response: StreamingResponse) -> StreamingResponse {
        let capture = self.clone();
        let (parts, source) = response.into_parts();
        let mut source = source.eventsource();
        http::Response::from_parts(
            parts,
            Box::pin(async_stream::stream! {
                while let Some(event) = source.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => { yield Err(http_error(std::io::Error::other("Copilot SSE transport failed"))); return; }
                    };
                    let result = (|| {
                        let data = match serde_json::from_str::<Value>(&event.data) {
                            Ok(mut value) => {
                                let completed = value["type"] == "response.completed";
                                if let Some(response) = value.get_mut("response") {
                                    *response = capture.response(response.take(), completed)?;
                                }
                                value = capture.response(value, false)?;
                                serde_json::to_string(&value).map_err(http_error)?
                            }
                            Err(_) => event.data,
                        };
                        let retry = event.retry.map(|delay| format!("retry: {}\n", delay.as_millis())).unwrap_or_default();
                        Ok::<_, http_client::Error>(bytes::Bytes::from(format!("event: {}\nid: {}\n{retry}data: {data}\n\n", event.event, event.id)))
                    })();
                    yield result;
                }
            }),
        )
    }
}

fn http_error(error: impl std::error::Error + Send + Sync + 'static) -> http_client::Error {
    http_client::Error::Instance(Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sse_metadata_survives_copilot_only_opaque_capture(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // @step Given a synthetic Copilot response event carries an identifier event name retry directive and owned opaque
        let source = futures::stream::iter([Ok::<_, http_client::Error>(bytes::Bytes::from_static(
            b"event: synthetic-terminal\nid: synthetic-event-id\nretry: 37\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"synthetic-response\",\"status\":\"completed\",\"reasoning_opaque\":\"synthetic-private-value\"}}\n\n"
        ))]);
        let source: http_client::sse::BoxedStream = Box::pin(source);
        let response = http::Response::builder().status(200).body(source)?;
        let capture = Capture::new(Arc::new(|| true));
        // @step When the exact production capture transforms its byte stream
        let mut events = capture.stream(response).into_body().eventsource();
        let event = events.next().await.ok_or("missing transformed event")??;
        // @step Then downstream event metadata remains unchanged and opaque is absent from its event data
        assert_eq!(event.event, "synthetic-terminal");
        assert_eq!(event.id, "synthetic-event-id");
        assert!(!event.data.contains("synthetic-private-value"));
        assert!(!event.data.contains("reasoning_opaque"));
        assert_eq!(event.retry, Some(std::time::Duration::from_millis(37)));
        Ok(())
    }
}
