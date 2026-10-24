use serde_json::Value;
use std::sync::{Arc, Mutex};

#[path = "metadata_delivery.rs"]
mod delivery;
#[path = "metadata_protocol.rs"]
mod protocol;
#[path = "metadata_status.rs"]
mod status;
pub use status::{CodexMetadataStatus, RateLimits, RateWindow};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexPresentationEvent {
    Waiting(bool),
    Verification,
}

type Observer = Arc<dyn Fn(CodexMetadataStatus) + Send + Sync>;
type PresentationObserver = Arc<dyn Fn(CodexPresentationEvent) + Send + Sync>;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Scope {
    turn: u64,
    stream: u64,
}

#[derive(Default)]
struct State {
    status: CodexMetadataStatus,
    etag: Option<String>,
    turn_state: Option<String>,
    header_advice: Option<String>,
    http_response: bool,
    observer: Option<Observer>,
    presentation_observer: Option<PresentationObserver>,
    scope: Scope,
    waiting: bool,
    output_started: bool,
    delivery: delivery::Queue,
}

#[derive(Clone, Default)]
pub struct CodexMetadataHandle(Arc<Mutex<State>>, Option<Scope>);

impl std::fmt::Debug for CodexMetadataHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexMetadataHandle(<private>)")
    }
}

impl CodexMetadataHandle {
    pub fn status(&self) -> CodexMetadataStatus {
        self.0
            .lock()
            .map(|state| state.status.clone())
            .unwrap_or_default()
    }

    pub fn models_etag(&self) -> Option<String> {
        self.0.lock().ok().and_then(|state| state.etag.clone())
    }

    pub fn turn_state(&self) -> Option<String> {
        self.0
            .lock()
            .ok()
            .and_then(|state| state.turn_state.clone())
    }

    pub fn set_observer(&self, observer: Observer) {
        if let Ok(mut state) = self.0.lock() {
            state.observer = Some(observer);
        }
    }

    pub fn set_presentation_observer(&self, observer: PresentationObserver) {
        self.update(|state| delivery::replace_observer(state, observer));
    }

    fn update(&self, update: impl FnOnce(&mut State)) {
        let dispatch = self.0.lock().ok().is_some_and(|mut state| {
            if self.1.is_some_and(|scope| scope != state.scope) {
                return false;
            }
            let previous = state.status.clone();
            update(&mut state);
            delivery::enqueue(&mut state, previous)
        });
        if dispatch {
            delivery::dispatch(&self.0);
        }
    }

    pub fn scoped_stream(&self) -> Self {
        if self.1.is_some() {
            return self.clone();
        }
        let scope = self.0.lock().ok().map(|mut state| {
            state.scope.stream = state.scope.stream.wrapping_add(1);
            state.scope
        });
        Self(self.0.clone(), scope)
    }

    pub fn begin_turn(&self) {
        self.update(|state| {
            state.scope.turn = state.scope.turn.wrapping_add(1);
            state.waiting = false;
            state.output_started = false;
            state.header_advice = None;
            state.http_response = false;
            state.status.buffering = false;
            state.status.faster_model = None;
            state.status.verification_required = false;
            state.status.moderation_received = false;
        });
    }

    pub fn reset(&self) {
        self.begin_turn();
    }

    pub fn begin_response(&self) {
        self.update(|state| {
            state.header_advice = None;
            state.http_response = false;
            state.status.buffering = false;
            state.waiting = false;
        });
    }

    pub fn clear_buffering(&self) {
        self.update(|state| {
            state.status.buffering = false;
            state.waiting = false;
        });
    }

    pub(crate) fn apply_http_headers(&self, headers: &http::HeaderMap) {
        self.apply_transport_headers(headers, true);
    }

    pub(crate) fn apply_websocket_headers(&self, headers: &http::HeaderMap) {
        self.apply_transport_headers(headers, false);
    }

    fn apply_transport_headers(&self, headers: &http::HeaderMap, http_response: bool) {
        let headers = headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_owned(), Value::String(value.to_owned())))
            })
            .collect();
        let event = serde_json::json!({"headers": Value::Object(headers)});
        self.update(|state| {
            state.http_response = http_response;
            state.header_advice = None;
            protocol::apply_headers(state, &event);
            if http_response {
                protocol::apply_header_advice(state, &event["headers"]);
            }
        });
    }

    pub fn apply_event(&self, event: &Value) {
        self.update(|state| protocol::apply(state, event));
    }

    pub(crate) fn guard(&self) -> BufferingGuard {
        BufferingGuard(self.clone())
    }

    fn finish_stream(&self) {
        self.update(|state| {
            state.status.buffering = false;
            state.waiting = false;
            state.scope.stream = state.scope.stream.wrapping_add(1);
        });
    }
}

pub(crate) struct BufferingGuard(CodexMetadataHandle);

impl Drop for BufferingGuard {
    fn drop(&mut self) {
        self.0.finish_stream();
    }
}

fn safe(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte)))
    .then(|| value.to_owned())
}
