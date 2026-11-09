//! PROV-171: `OPENAI_CONTEXT_WINDOW` / `OPENAI_MAX_OUTPUT_TOKENS` limit overrides.
//!
//! An unset variable means the limit is not overridden — it comes from the
//! user, profile or catalog, or is unavailable. A set variable must be a
//! positive integer; anything else (including `0` and the empty string) is
//! [`ProviderError::InvalidLimitOverride`] naming the variable and the value.

use crate::ProviderError;

const CONTEXT_WINDOW_VAR: &str = "OPENAI_CONTEXT_WINDOW";
const MAX_OUTPUT_TOKENS_VAR: &str = "OPENAI_MAX_OUTPUT_TOKENS";

/// The limit overrides an OpenAI-compatible session declares through the
/// environment. `None` means the variable is unset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenAIEnvLimits {
    /// `OPENAI_CONTEXT_WINDOW`, when set.
    pub context_window: Option<usize>,
    /// `OPENAI_MAX_OUTPUT_TOKENS`, when set.
    pub max_output_tokens: Option<usize>,
}

impl OpenAIEnvLimits {
    /// Read both overrides from the process environment.
    pub fn from_env() -> Result<Self, ProviderError> {
        Ok(Self {
            context_window: read_limit(CONTEXT_WINDOW_VAR)?,
            max_output_tokens: read_limit(MAX_OUTPUT_TOKENS_VAR)?,
        })
    }
}

fn read_limit(variable: &'static str) -> Result<Option<usize>, ProviderError> {
    let Some(raw) = std::env::var_os(variable) else {
        return Ok(None);
    };
    let value = raw.to_string_lossy();
    match value.parse::<usize>() {
        Ok(limit) if limit > 0 => Ok(Some(limit)),
        _ => Err(ProviderError::InvalidLimitOverride {
            provider: "openai".to_string(),
            variable,
            value: value.into_owned(),
        }),
    }
}
