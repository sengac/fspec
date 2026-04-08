//! CopilotEndpointFacade — pure function that selects the correct Copilot API
//! endpoint (`/chat/completions` vs `/responses`) based on the model ID.
//!
//! PROV-055: Rule 5 — gpt-N where N >= 5 routes to `/responses`, with the
//! single explicit exception of `gpt-5-mini` which stays on `/chat/completions`.
//! Every other model (gpt-4*, claude-*, gemini-*, etc.) uses `/chat/completions`.
//!
//! This module is intentionally pure: no IO, no state, no errors — just a
//! deterministic mapping from `&str` to [`CopilotEndpoint`].

/// Which Copilot API endpoint a model should be routed to.
///
/// See PROV-055 rule 5 in the parent work unit for the routing table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotEndpoint {
    /// OpenAI-compatible chat/completions endpoint — default for everything
    /// except GPT-5-family models (see [`Responses`](Self::Responses)).
    ChatCompletions,
    /// OpenAI Responses API — used only for GPT-5+ models *except* gpt-5-mini.
    Responses,
}

impl CopilotEndpoint {
    /// The path suffix (after the base URL) this endpoint corresponds to.
    ///
    /// - [`ChatCompletions`](Self::ChatCompletions) → `"/chat/completions"`
    /// - [`Responses`](Self::Responses) → `"/responses"`
    #[must_use]
    pub const fn path(&self) -> &'static str {
        match self {
            Self::ChatCompletions => "/chat/completions",
            Self::Responses => "/responses",
        }
    }
}

/// Pure-function facade that picks the right Copilot endpoint for a given
/// model ID.
///
/// # Examples
///
/// ```
/// use codelet_providers::copilot::endpoint::{CopilotEndpoint, CopilotEndpointFacade};
///
/// assert_eq!(
///     CopilotEndpointFacade::select("gpt-4o"),
///     CopilotEndpoint::ChatCompletions
/// );
/// assert_eq!(
///     CopilotEndpointFacade::select("gpt-5"),
///     CopilotEndpoint::Responses
/// );
/// // The single explicit exception — gpt-5-mini stays on /chat/completions.
/// assert_eq!(
///     CopilotEndpointFacade::select("gpt-5-mini"),
///     CopilotEndpoint::ChatCompletions
/// );
/// ```
pub struct CopilotEndpointFacade;

impl CopilotEndpointFacade {
    /// Select the [`CopilotEndpoint`] for a given model ID per PROV-055 rule 5.
    ///
    /// # Arguments
    ///
    /// * `model_id` - The Copilot-exposed model identifier (e.g. `"gpt-5"`,
    ///   `"gpt-4o-copilot"`, `"claude-sonnet-4.5"`, `"gemini-2.5-pro"`).
    ///
    /// # Returns
    ///
    /// [`CopilotEndpoint::Responses`] if the model is `gpt-N` where `N >= 5`
    /// AND the model is not `"gpt-5-mini"`. Otherwise
    /// [`CopilotEndpoint::ChatCompletions`].
    #[must_use]
    pub fn select(model_id: &str) -> CopilotEndpoint {
        // Explicit exclusion — gpt-5-mini is the only GPT-5-family model that
        // stays on /chat/completions. Check this first so the general rule
        // below can stay simple.
        if model_id == "gpt-5-mini" {
            return CopilotEndpoint::ChatCompletions;
        }

        // Match the "gpt-N" prefix and extract the numeric family.
        if let Some(rest) = model_id.strip_prefix("gpt-") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if n >= 5 {
                    return CopilotEndpoint::Responses;
                }
            }
        }

        CopilotEndpoint::ChatCompletions
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn gpt_4_family_uses_chat_completions() {
        assert_eq!(
            CopilotEndpointFacade::select("gpt-4"),
            CopilotEndpoint::ChatCompletions
        );
        assert_eq!(
            CopilotEndpointFacade::select("gpt-4o"),
            CopilotEndpoint::ChatCompletions
        );
        assert_eq!(
            CopilotEndpointFacade::select("gpt-4o-copilot"),
            CopilotEndpoint::ChatCompletions
        );
    }

    #[test]
    fn gpt_5_family_uses_responses_except_mini() {
        assert_eq!(
            CopilotEndpointFacade::select("gpt-5"),
            CopilotEndpoint::Responses
        );
        assert_eq!(
            CopilotEndpointFacade::select("gpt-5-codex"),
            CopilotEndpoint::Responses
        );
        assert_eq!(
            CopilotEndpointFacade::select("gpt-5-mini"),
            CopilotEndpoint::ChatCompletions
        );
    }

    #[test]
    fn non_gpt_models_use_chat_completions() {
        assert_eq!(
            CopilotEndpointFacade::select("claude-sonnet-4.5"),
            CopilotEndpoint::ChatCompletions
        );
        assert_eq!(
            CopilotEndpointFacade::select("gemini-2.5-pro"),
            CopilotEndpoint::ChatCompletions
        );
    }

    #[test]
    fn path_strings_are_stable() {
        assert_eq!(CopilotEndpoint::ChatCompletions.path(), "/chat/completions");
        assert_eq!(CopilotEndpoint::Responses.path(), "/responses");
    }
}
