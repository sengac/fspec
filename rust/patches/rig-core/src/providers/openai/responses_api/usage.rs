use super::ResponsesUsage;
use crate::completion::{CompletionError, GetTokenUsage};

pub(super) fn checked_sum(left: u64, right: u64) -> Result<u64, CompletionError> {
    left.checked_add(right).ok_or_else(|| {
        CompletionError::ResponseError("Responses usage aggregation overflow".to_string())
    })
}

impl ResponsesUsage {
    pub fn checked_add(self, rhs: Self) -> Result<Self, CompletionError> {
        if self.token_usage().is_none() || rhs.token_usage().is_none() {
            return Err(CompletionError::ResponseError(
                "Responses usage aggregation requires consistent physical usage".to_string(),
            ));
        }
        Ok(Self {
            input_tokens: checked_sum(self.input_tokens, rhs.input_tokens)?,
            output_tokens: checked_sum(self.output_tokens, rhs.output_tokens)?,
            total_tokens: checked_sum(self.total_tokens, rhs.total_tokens)?,
            input_tokens_details: self
                .input_tokens_details
                .zip(rhs.input_tokens_details)
                .map(|(left, right)| left + right)
                .transpose()?,
            output_tokens_details: self
                .output_tokens_details
                .zip(rhs.output_tokens_details)
                .map(|(left, right)| left + right)
                .transpose()?,
        })
    }
}

impl GetTokenUsage for ResponsesUsage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        crate::completion::Usage::from_inclusive(
            self.input_tokens,
            self.output_tokens,
            self.total_tokens,
            self.input_tokens_details
                .as_ref()
                .map(|detail| detail.cached_tokens),
            self.output_tokens_details
                .as_ref()
                .map(|detail| detail.reasoning_tokens),
        )
    }
}
