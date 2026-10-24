use rig::completion::CompletionError;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Codex native memory unsupported ({boundary}); use fspec's committed DAG; disable native memory at the source")]
pub struct NativeMemoryUnsupported {
    boundary: &'static str,
}

#[derive(Debug, Error)]
#[error("Codex delta mode is unsupported; explicitly select full-current-DAG requests")]
pub struct DeltaModeUnsupported;

pub(crate) fn unsupported(boundary: &'static str) -> CompletionError {
    CompletionError::RequestError(Box::new(NativeMemoryUnsupported { boundary }))
}

pub(crate) fn validate_item(item: &Value) -> Result<(), CompletionError> {
    let family = match item["type"].as_str() {
        Some("compaction") => "compaction",
        Some("compaction_summary") => "compaction_summary",
        Some("context_compaction") => "context_compaction",
        Some("compaction_trigger") => "compaction_trigger",
        _ => return Ok(()),
    };
    Err(unsupported(family))
}

pub(crate) fn validate_request(request: &Value) -> Result<(), CompletionError> {
    if request.get("previous_response_id").is_some() {
        return Err(CompletionError::RequestError(Box::new(
            DeltaModeUnsupported,
        )));
    }
    if request.get("context_management").is_some() {
        return Err(unsupported("request configuration"));
    }
    if request["truncation"] == "auto" {
        return Err(unsupported("automatic truncation"));
    }
    if let Some(items) = request["input"].as_array() {
        for item in items {
            validate_item(item)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_response(response: &Value) -> Result<(), CompletionError> {
    if let Some(items) = response["output"].as_array() {
        for item in items {
            validate_item(item)?;
        }
    }
    Ok(())
}
