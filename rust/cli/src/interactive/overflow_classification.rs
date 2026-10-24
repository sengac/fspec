use codelet_providers::codex::diagnostics::{CodexRejection, RejectionClass};
use codelet_providers::request_admission::RequestAdmissionError;
use rig::completion::CompletionError;

pub(super) fn is_context_overflow(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<CodexRejection>()
            .is_some_and(|rejection| rejection.class == RejectionClass::ContextLengthExceeded)
            || source
                .downcast_ref::<RequestAdmissionError>()
                .is_some_and(|failure| {
                    matches!(failure, RequestAdmissionError::BudgetExceeded { .. })
                })
            || source
                .downcast_ref::<CompletionError>()
                .is_some_and(|failure| match failure {
                    CompletionError::ProviderError(body) => has_overflow_code(body),
                    CompletionError::HttpError(failure) => http_overflow(failure),
                    _ => false,
                })
            || source
                .downcast_ref::<rig::http_client::Error>()
                .is_some_and(http_overflow)
    })
}

fn http_overflow(error: &rig::http_client::Error) -> bool {
    match error {
        rig::http_client::Error::InvalidStatusCodeWithMessage(_, body) => has_overflow_code(body),
        _ => false,
    }
}

fn has_overflow_code(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .is_some_and(|value| {
            matches!(
                value
                    .pointer("/error/code")
                    .or_else(|| value.get("code"))
                    .and_then(serde_json::Value::as_str),
                Some("context_length_exceeded" | "prompt_too_long" | "context_window_exceeded")
            )
        })
}
