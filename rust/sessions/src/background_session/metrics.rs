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

    /// PROV-165: cache the selected model's resolved limits.
    pub fn set_model_limits(&self, limits: crate::model_resolution::ResolvedModelLimits) {
        *self
            .model_limits
            .write()
            .expect("model_limits lock poisoned") = limits;
    }

    /// PROV-165: the selected model's resolved limits (`None` = unavailable).
    pub fn model_limits(&self) -> crate::model_resolution::ResolvedModelLimits {
        *self
            .model_limits
            .read()
            .expect("model_limits lock poisoned")
    }

    /// MODEL-011: the chosen level, exactly as chosen.
    pub fn get_base_thinking_level(&self) -> codelet_rpc_types::ThinkingLevel {
        self.base_thinking_level
            .read()
            .expect("base_thinking_level lock poisoned")
            .clone()
    }

    /// MODEL-011: store `level` unchanged (BUG-455: ModelDefault means no
    /// level was chosen). Callers that take a user's choice check it first
    /// ([`crate::thinking_options::accepted`]).
    pub fn set_base_thinking_level(&self, level: codelet_rpc_types::ThinkingLevel) {
        *self
            .base_thinking_level
            .write()
            .expect("base_thinking_level lock poisoned") = level;
    }

    /// MODEL-012: set the session's level from its current model's saved
    /// level (when a session is created or resumed, and when it switches
    /// model). A saved level the model offers is applied; otherwise the level
    /// is not set, and a saved level that is not offered or cannot be read is
    /// explained in a warning notice (product owner, 2026-11-04/05).
    pub fn apply_saved_thinking_level(&self) {
        let applied = match self.thinking_target() {
            Ok(target) => crate::thinking_options::saved_level_for(
                &target,
                crate::default_thinking_level_persistence::load_default_thinking_level(
                    target.model(),
                ),
            ),
            Err(error) => crate::thinking_options::SavedLevel {
                level: codelet_rpc_types::ThinkingLevel::ModelDefault,
                notice: Some(format!(
                    "the saved thinking level cannot be applied: {error}; the thinking level is not set"
                )),
            },
        };
        self.set_base_thinking_level(applied.level);
        if let Some(message) = applied.notice {
            tracing::warn!(session = %self.id, %message, "saved thinking level not applied");
            self.handle_output(codelet_rpc_types::StreamChunk::UserNotification {
                message,
                severity: codelet_rpc_types::NotificationSeverity::Warning,
            });
        }
    }

    /// MODEL-011: cache the selected model's thinking target.
    pub fn set_thinking_target(&self, target: codelet_providers::thinking_wire::ThinkingTarget) {
        *self
            .thinking_target
            .write()
            .expect("thinking_target lock poisoned") = Some(target);
    }

    /// MODEL-011: the selected model's thinking target; an error when no
    /// model has been resolved for the session yet.
    pub fn thinking_target(
        &self,
    ) -> Result<codelet_providers::thinking_wire::ThinkingTarget, String> {
        self.thinking_target
            .read()
            .map_err(|_| {
                format!(
                    "the thinking target lock of session {} is poisoned",
                    self.id
                )
            })?
            .clone()
            .ok_or_else(|| format!("session {} has no model resolved yet", self.id))
    }

    pub fn get_model_id(&self) -> Option<String> {
        self.model_id
            .read()
            .expect("model_id lock poisoned")
            .clone()
    }
}
