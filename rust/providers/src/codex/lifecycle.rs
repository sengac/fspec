use rig::completion::CompletionError;
use rig::providers::openai::responses_api::{
    InputTokensDetails, OutputTokensDetails, ResponsesUsage,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub(super) struct Terminal {
    pub id: String,
    pub status: String,
    pub output: Vec<Value>,
    pub end_turn: Option<bool>,
    pub usage: Option<TerminalUsage>,
    pub error: Option<Value>,
    pub incomplete_details: Option<IncompleteDetails>,
}

#[derive(Deserialize)]
pub(super) struct IncompleteDetails {
    pub reason: String,
}

#[derive(Deserialize)]
pub(super) struct TerminalUsage {
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    input_tokens_details: Option<InputTokensDetails>,
    output_tokens_details: Option<OutputTokensDetails>,
}

impl TerminalUsage {
    pub fn into_usage(self) -> ResponsesUsage {
        ResponsesUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            total_tokens: self.total_tokens,
            input_tokens_details: self.input_tokens_details,
            output_tokens_details: self.output_tokens_details,
        }
    }
}

pub(super) fn invalid(message: impl std::fmt::Display) -> CompletionError {
    CompletionError::ResponseError(format!("Codex malformed stream event: {message}"))
}

pub(super) fn backend_error(
    value: Value,
    redactor: &crate::codex::diagnostic_redaction::Redactor,
) -> CompletionError {
    crate::codex::diagnostics::CodexRejection::backend(value, redactor)
        .into_http_error()
        .into()
}

impl Terminal {
    pub fn parse(kind: &str, value: &mut Value) -> Result<Self, CompletionError> {
        let fields = value
            .as_object_mut()
            .ok_or_else(|| invalid("expected terminal response object"))?;
        fields
            .entry("status")
            .or_insert_with(|| Value::String(kind.trim_start_matches("response.").into()));
        fields
            .entry("output")
            .or_insert_with(|| Value::Array(Vec::new()));
        let terminal: Self = serde_json::from_value(value.clone()).map_err(invalid)?;
        if terminal.id.is_empty() || kind != format!("response.{}", terminal.status) {
            return Err(invalid("terminal status or response identity mismatch"));
        }
        if terminal.status == "failed" && terminal.error.is_none() {
            return Err(invalid("failed response missing error"));
        }
        if terminal.status == "incomplete" && terminal.incomplete_details.is_none() {
            return Err(invalid("incomplete response missing reason"));
        }
        Ok(terminal)
    }

    pub fn should_continue(&self, saw_tool: bool) -> bool {
        if saw_tool || self.has_refusal() {
            return false;
        }
        match self.status.as_str() {
            "completed" => self.end_turn == Some(false),
            "incomplete" => {
                self.end_turn != Some(true)
                    && self
                        .incomplete_details
                        .as_ref()
                        .is_some_and(|details| details.reason == "interrupted")
            }
            _ => false,
        }
    }

    pub fn error(
        &self,
        redactor: &crate::codex::diagnostic_redaction::Redactor,
    ) -> Option<CompletionError> {
        match self.status.as_str() {
            "failed" => self
                .error
                .as_ref()
                .map(|error| backend_error(error.clone(), redactor)),
            "incomplete" => self.incomplete_details.as_ref().map(|details| {
                let outcome = match details.reason.as_str() {
                    "content_filter" => "content-filtered",
                    "max_output_tokens" => "budget-limited",
                    _ => "interrupted",
                };
                CompletionError::ResponseError(format!(
                    "Codex incomplete response: {outcome} ({})",
                    details.reason
                ))
            }),
            _ => None,
        }
    }

    pub fn replay(&self, request: &mut Value) -> Result<(), CompletionError> {
        crate::codex::protocol_replay::append(request, &self.output)
    }

    pub fn has_refusal(&self) -> bool {
        self.output.iter().any(|item| {
            item["type"] == "message"
                && item["content"]
                    .as_array()
                    .is_some_and(|content| content.iter().any(|part| part["type"] == "refusal"))
        })
    }
}
