use super::{update_session_manifest, ManifestCommitError, SessionManifest};
use rig::completion::Usage;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum SettlementError {
    #[error("request settlement failed: {0}")]
    Persistence(#[from] ManifestCommitError),
    #[error("request settlement failed: {0}")]
    Failed(String),
}

pub fn replace_request_context(
    session_id: Uuid,
    usage: Option<Usage>,
) -> Result<SessionManifest, SettlementError> {
    Ok(update_session_manifest(session_id, |current| {
        current.latest_request_usage = usage;
        current.token_usage.current_context_tokens = usage.map_or(0, |usage| usage.total_tokens);
        current.token_usage.cache_read_tokens = usage
            .and_then(|usage| usage.cache_read_input_tokens)
            .unwrap_or(0);
        current.token_usage.cache_creation_tokens = usage
            .and_then(|usage| usage.cache_creation_input_tokens)
            .unwrap_or(0);
        Ok(())
    })?)
}

pub fn settle_request(
    session_id: Uuid,
    request_id: &str,
    usage: &Usage,
) -> Result<SessionManifest, SettlementError> {
    Ok(update_session_manifest(session_id, |current| {
        if let Some(previous) = current.request_settlements.get(request_id) {
            return if previous == usage {
                Ok(())
            } else {
                Err(ManifestCommitError::Store(format!(
                    "conflicting usage for request {request_id}"
                )))
            };
        }
        let input = usage
            .input_tokens
            .checked_add(usage.cache_read_input_tokens.unwrap_or(0))
            .and_then(|value| value.checked_add(usage.cache_creation_input_tokens.unwrap_or(0)))
            .ok_or_else(|| ManifestCommitError::Store("input usage overflow".into()))?;
        let output = usage
            .output_tokens
            .checked_add(usage.reasoning_tokens.unwrap_or(0))
            .ok_or_else(|| ManifestCommitError::Store("output usage overflow".into()))?;
        if input.checked_add(output) != Some(usage.total_tokens) {
            return Err(ManifestCommitError::Store(
                "inconsistent normalized request usage".into(),
            ));
        }
        current.token_usage.cumulative_billed_input = current
            .token_usage
            .cumulative_billed_input
            .checked_add(input)
            .ok_or_else(|| ManifestCommitError::Store("input billing overflow".into()))?;
        current.token_usage.cumulative_billed_output = current
            .token_usage
            .cumulative_billed_output
            .checked_add(output)
            .ok_or_else(|| ManifestCommitError::Store("output billing overflow".into()))?;
        current.token_usage.reasoning_tokens = current
            .token_usage
            .reasoning_tokens
            .checked_add(usage.reasoning_tokens.unwrap_or(0))
            .ok_or_else(|| ManifestCommitError::Store("reasoning usage overflow".into()))?;
        current
            .request_settlements
            .insert(request_id.into(), *usage);
        Ok(())
    })?)
}
