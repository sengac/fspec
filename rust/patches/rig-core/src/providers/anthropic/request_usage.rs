use super::streaming::PartialUsage;
use crate::completion::{CompletionError, GetTokenUsage, Usage};

#[derive(Default)]
pub(super) struct RequestUsage {
    measured: Option<PartialUsage>,
}

impl RequestUsage {
    pub fn start(&mut self, usage: &super::completion::Usage) -> Option<Usage> {
        self.measured = Some(PartialUsage {
            input_tokens: Some(usage.input_tokens.try_into().ok()?),
            output_tokens: usage.output_tokens.try_into().ok()?,
            cache_read_input_tokens: usage
                .cache_read_input_tokens
                .map(usize::try_from)
                .transpose()
                .ok()?,
            cache_creation_input_tokens: usage
                .cache_creation_input_tokens
                .map(usize::try_from)
                .transpose()
                .ok()?,
        });
        self.measured.as_ref()?.token_usage()
    }

    pub fn merge(&mut self, delta: &PartialUsage) -> Result<PartialUsage, CompletionError> {
        let current = self.measured.as_mut().ok_or_else(|| {
            CompletionError::ResponseError(
                "incomplete Anthropic usage: missing message_start".into(),
            )
        })?;
        if delta
            .input_tokens
            .is_some_and(|value| Some(value) != current.input_tokens)
            || delta.cache_read_input_tokens.is_some_and(|value| {
                current
                    .cache_read_input_tokens
                    .is_some_and(|previous| previous != value)
            })
            || delta.cache_creation_input_tokens.is_some_and(|value| {
                current
                    .cache_creation_input_tokens
                    .is_some_and(|previous| previous != value)
            })
            || delta.output_tokens < current.output_tokens
        {
            return Err(CompletionError::ResponseError(
                "inconsistent Anthropic usage".into(),
            ));
        }
        current.output_tokens = delta.output_tokens;
        current.cache_read_input_tokens = delta
            .cache_read_input_tokens
            .or(current.cache_read_input_tokens);
        current.cache_creation_input_tokens = delta
            .cache_creation_input_tokens
            .or(current.cache_creation_input_tokens);
        if current.token_usage().is_none() {
            return Err(CompletionError::ResponseError(
                "invalid Anthropic usage: occupancy overflow".into(),
            ));
        }
        Ok(current.clone())
    }
}
