//! System-prompt facade plumbing for GitHub Copilot (PROV-055 rule 8).
//!
//! The Copilot `/chat/completions` endpoint is wire-compatible with OpenAI
//! and reuses [`OpenAISystemPromptFacade`] directly. The `/responses`
//! endpoint needs a distinct facade identity so downstream code (and
//! tests) can tell the two paths apart, but its wire format is still
//! OpenAI-shaped so all transformations delegate to the shared facade.

use crate::copilot::endpoint::CopilotEndpoint;
use codelet_tools::facade::{
    BoxedSystemPromptFacade, OpenAISystemPromptFacade, SystemPromptFacade,
};
use serde_json::Value;

/// Chat-completions system-prompt facade — a thin alias for
/// [`OpenAISystemPromptFacade`], because the Copilot `/chat/completions`
/// endpoint is wire-compatible with OpenAI and should not have its own
/// parallel facade hierarchy.
pub type CopilotChatCompletionsSystemPromptFacade = OpenAISystemPromptFacade;

/// System-prompt facade for the Copilot `/responses` endpoint.
///
/// Implements the real [`SystemPromptFacade`] trait from
/// `codelet_tools::facade::system_prompt`, so it plugs directly into
/// [`BoxedSystemPromptFacade`] dispatch. Delegates everything except the
/// provider identifier to the shared [`OpenAISystemPromptFacade`] — the
/// `/responses` endpoint is OpenAI-shaped on the wire and only needs its
/// own name so downstream code (and tests) can tell the two paths apart.
pub struct CopilotResponsesSystemPromptFacade;

impl SystemPromptFacade for CopilotResponsesSystemPromptFacade {
    fn provider(&self) -> &'static str {
        "copilot-responses"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        OpenAISystemPromptFacade.identity_prefix()
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        OpenAISystemPromptFacade.transform_preamble(preamble)
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        OpenAISystemPromptFacade.format_for_api(preamble)
    }
}

/// Select the system-prompt facade for a given endpoint.
///
/// - [`CopilotEndpoint::ChatCompletions`] →
///   [`OpenAISystemPromptFacade`] (`provider == "openai"`)
/// - [`CopilotEndpoint::Responses`] →
///   [`CopilotResponsesSystemPromptFacade`] (`provider == "copilot-responses"`)
#[must_use]
pub fn system_prompt_facade_for_endpoint(
    endpoint: CopilotEndpoint,
) -> BoxedSystemPromptFacade {
    match endpoint {
        CopilotEndpoint::ChatCompletions => Box::new(OpenAISystemPromptFacade),
        CopilotEndpoint::Responses => Box::new(CopilotResponsesSystemPromptFacade),
    }
}
