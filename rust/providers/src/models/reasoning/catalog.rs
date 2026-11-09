//! A catalog model's reasoning capability, from its models.dev
//! `reasoning_options`.

use super::{ReasoningCapability, ReasoningError, ReasoningLevel};
use crate::models::registry::ModelRegistry;

impl ModelRegistry {
    /// `provider/model`'s reasoning capability; `None` when it has no
    /// reasoning control.
    pub fn reasoning_capability(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Option<ReasoningCapability>, ReasoningError> {
        let info = self.get_model(provider, model)?;
        ReasoningCapability::resolve(
            &format!("{provider}/{model}"),
            info.reasoning_options.as_deref(),
        )
    }

    /// Accept `level` unchanged if `provider/model` takes it.
    pub fn check_reasoning_level(
        &self,
        provider: &str,
        model: &str,
        level: &ReasoningLevel,
    ) -> Result<ReasoningLevel, ReasoningError> {
        let id = format!("{provider}/{model}");
        match self.reasoning_capability(provider, model)? {
            Some(capability) => capability.check(&id, level),
            None => Err(ReasoningError::NoReasoningControl { model: id }),
        }
    }
}
