use super::RhaiCustomCompletion;

impl rig::completion::GetTokenUsage for RhaiCustomCompletion {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        if self.usage_measured == Some(false)
            || (self.usage_measured.is_none()
                && self.input_tokens == 0
                && self.output_tokens == 0
                && self.cache_read_input_tokens.is_none()
                && self.cache_creation_input_tokens.is_none()
                && self.reasoning_tokens.is_none())
        {
            return None;
        }
        let mut usage = rig::completion::Usage::new();
        usage.input_tokens = self.input_tokens;
        usage.output_tokens = self.output_tokens;
        usage.total_tokens = self
            .input_tokens
            .checked_add(self.output_tokens)?
            .checked_add(self.cache_read_input_tokens.unwrap_or(0))?
            .checked_add(self.cache_creation_input_tokens.unwrap_or(0))?
            .checked_add(self.reasoning_tokens.unwrap_or(0))?;
        usage.cache_read_input_tokens = self.cache_read_input_tokens;
        usage.cache_creation_input_tokens = self.cache_creation_input_tokens;
        usage.reasoning_tokens = self.reasoning_tokens;
        Some(usage)
    }

    fn stop_reason(&self) -> Option<&str> {
        Some(self.stop_reason.as_str())
    }
}
