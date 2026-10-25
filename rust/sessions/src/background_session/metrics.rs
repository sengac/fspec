use super::*;

impl BackgroundSession {
    pub fn get_debug_enabled(&self) -> bool {
        self.is_debug_enabled.load(Ordering::Acquire)
    }

    pub fn set_debug_enabled(&self, enabled: bool) {
        self.is_debug_enabled.store(enabled, Ordering::Release);
    }

    pub fn update_tokens(&self, input_tokens: u32, output_tokens: u32) {
        self.cached_input_tokens
            .store(input_tokens, Ordering::Release);
        self.cached_output_tokens
            .store(output_tokens, Ordering::Release);
    }

    pub fn store_pre_compaction_tokens(&self, tokens: u32) {
        self.pre_compaction_tokens.store(tokens, Ordering::Release);
    }

    pub fn snapshot_pre_compaction_tokens(&self) -> u32 {
        let current = self.cached_input_tokens.load(Ordering::Acquire);
        self.store_pre_compaction_tokens(current);
        current
    }

    pub fn update_reasoning_tokens(&self, reasoning_tokens: u32) {
        self.cached_reasoning_tokens
            .store(reasoning_tokens, Ordering::Release);
    }

    pub fn get_tokens(&self) -> (u32, u32, Option<u32>) {
        let reasoning = self.cached_reasoning_tokens.load(Ordering::Acquire);
        (
            self.cached_input_tokens.load(Ordering::Acquire),
            self.cached_output_tokens.load(Ordering::Acquire),
            if reasoning > 0 { Some(reasoning) } else { None },
        )
    }

    pub fn set_model(&self, provider_id: Option<String>, model_id: Option<String>) {
        *self.provider_id.write().expect("provider_id lock poisoned") = provider_id;
        *self.model_id.write().expect("model_id lock poisoned") = model_id;
    }

    pub fn set_model_limits(
        &self,
        context_window: u32,
        max_output_tokens: u32,
        compaction_threshold: u32,
    ) {
        self.cached_context_window
            .store(context_window, Ordering::Release);
        self.cached_max_output_tokens
            .store(max_output_tokens, Ordering::Release);
        self.cached_compaction_threshold
            .store(compaction_threshold, Ordering::Release);
    }

    pub fn get_base_thinking_level(&self) -> u8 {
        self.base_thinking_level.load(Ordering::Acquire)
    }

    pub fn set_base_thinking_level(&self, level: u8) {
        let clamped = level.min(3);
        self.base_thinking_level.store(clamped, Ordering::Release);
    }

    pub fn get_model_id(&self) -> Option<String> {
        self.model_id
            .read()
            .expect("model_id lock poisoned")
            .clone()
    }
}
