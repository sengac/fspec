use super::diagnostic_redaction::Redactor;
use futures::StreamExt;
use serde_json::Value;
use std::time::Duration;

pub const MAX_DIAGNOSTIC_BYTES: usize = 8192;
const MAX_MESSAGE_BYTES: usize = 2048;
const MAX_FIELD_BYTES: usize = 256;
const MAX_DIAGNOSTIC_TIME: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionClass {
    ContextLengthExceeded,
    Other,
    Unknown,
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Complete,
    Truncated,
    DeadlineExceeded,
    ReadFailed,
}

#[derive(Debug, thiserror::Error)]
#[error("Codex rejection status={status:?} class={class:?} code={code:?} message={message:?} request_id={request_id:?} retry_after={retry_after:?} attempts={attempts:?} evidence={state:?}: {body:?}")]
pub struct CodexRejection {
    pub status: Option<u16>,
    pub class: RejectionClass,
    pub code: Option<String>,
    pub message: Option<String>,
    pub request_id: Option<String>,
    pub retry_after: Option<String>,
    pub attempts: Option<usize>,
    pub state: EvidenceState,
    pub body: String,
}

impl CodexRejection {
    fn new(status: Option<u16>, state: EvidenceState, attempts: Option<usize>) -> Self {
        Self {
            status,
            class: RejectionClass::Unknown,
            code: None,
            message: None,
            request_id: None,
            retry_after: None,
            attempts,
            state,
            body: String::new(),
        }
    }

    fn parse(&mut self, error: &Value, redactor: &Redactor) {
        let code = error.get("code").and_then(Value::as_str);
        self.class = match code {
            Some("context_length_exceeded") => RejectionClass::ContextLengthExceeded,
            Some(code) if !code.is_empty() && code.len() <= MAX_FIELD_BYTES => {
                RejectionClass::Other
            }
            _ => RejectionClass::Unknown,
        };
        self.code = code.map(|value| redactor.clean(value, MAX_FIELD_BYTES));
        self.message = error
            .get("message")
            .and_then(Value::as_str)
            .map(|value| redactor.clean(value, MAX_MESSAGE_BYTES));
    }

    pub(super) fn backend(error: Value, redactor: &Redactor) -> Self {
        let mut diagnostic = Self::new(None, EvidenceState::Complete, None);
        diagnostic.parse(&error, redactor);
        let body = error.to_string();
        if body.len() > MAX_DIAGNOSTIC_BYTES {
            diagnostic.state = EvidenceState::Truncated;
        }
        diagnostic.body = redactor.clean_body(&body, MAX_DIAGNOSTIC_BYTES, false);
        diagnostic
    }

    pub(super) async fn http(
        response: reqwest::Response,
        redactor: &Redactor,
        attempts: usize,
        timeout: Duration,
    ) -> Self {
        let mut diagnostic = Self::new(
            Some(response.status().as_u16()),
            EvidenceState::Complete,
            Some(attempts),
        );
        let header = |name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(|value| redactor.clean_body(value, MAX_FIELD_BYTES, false))
        };
        diagnostic.request_id = header("x-request-id");
        diagnostic.retry_after = header("retry-after");
        let mut network = response.bytes_stream();
        let mut body = Vec::new();
        let deadline = tokio::time::Instant::now() + timeout.min(MAX_DIAGNOSTIC_TIME);
        loop {
            match tokio::time::timeout_at(deadline, network.next()).await {
                Ok(Some(Ok(chunk))) => {
                    let remaining = MAX_DIAGNOSTIC_BYTES - body.len();
                    body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    if chunk.len() > remaining {
                        diagnostic.state = EvidenceState::Truncated;
                        break;
                    }
                }
                Ok(None) => break,
                Ok(Some(Err(_))) => {
                    diagnostic.state = EvidenceState::ReadFailed;
                    break;
                }
                Err(_) => {
                    diagnostic.state = EvidenceState::DeadlineExceeded;
                    break;
                }
            }
        }
        let mut parsed = false;
        if diagnostic.state == EvidenceState::Complete {
            if let Ok(value) = serde_json::from_slice::<Value>(&body) {
                diagnostic.parse(&value["error"], redactor);
                parsed = true;
            }
        }
        let end = body
            .utf8_chunks()
            .last()
            .map_or(body.len(), |chunk| body.len() - chunk.invalid().len());
        let body = String::from_utf8_lossy(&body[..end]);
        diagnostic.body = if parsed {
            redactor.clean_body(&body, MAX_DIAGNOSTIC_BYTES, false)
        } else {
            redactor.clean_fragment(&body, MAX_DIAGNOSTIC_BYTES)
        };
        diagnostic
    }

    pub(super) fn into_http_error(self) -> rig::http_client::Error {
        rig::http_client::Error::Instance(Box::new(self))
    }
}
