use super::Usage;

impl Usage {
    pub fn from_inclusive(
        input: u64,
        output: u64,
        total: u64,
        cached: Option<u64>,
        reasoning: Option<u64>,
    ) -> Option<Self> {
        if input.checked_add(output)? != total {
            return None;
        }
        Some(Self {
            input_tokens: input.checked_sub(cached.unwrap_or(0))?,
            output_tokens: output.checked_sub(reasoning.unwrap_or(0))?,
            total_tokens: total,
            cache_read_input_tokens: cached,
            cache_creation_input_tokens: None,
            reasoning_tokens: reasoning,
        })
    }
}
