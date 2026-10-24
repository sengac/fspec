//! Compatibility exports for the shared background-session engine.
//!
//! NAPI session hooks and standalone sessions use the same recovery,
//! persistence, and session-scoped chunk publication lifecycle.

pub use codelet_agent_loop::{
    agent_loop, agent_loop_dispatch_supports_provider, BackgroundOutput, BackgroundProgressEmitter,
};

#[cfg(test)]
mod agent_loop_dispatch_tests {
    use super::agent_loop_dispatch_supports_provider;

    #[test]
    fn agent_loop_dispatch_supports_github_copilot_arm() {
        assert!(agent_loop_dispatch_supports_provider("github-copilot"));
        assert!(!agent_loop_dispatch_supports_provider("does-not-exist"));
    }

    #[test]
    fn agent_loop_dispatch_supports_copilot_short_alias() {
        assert!(agent_loop_dispatch_supports_provider("copilot"));
    }

    #[test]
    fn agent_loop_dispatch_still_supports_existing_providers() {
        for provider in ["claude", "openai", "gemini", "zai", "codex"] {
            assert!(agent_loop_dispatch_supports_provider(provider));
        }
    }

    #[test]
    fn copilot_create_rig_agent_signature_matches_dispatch_macro_contract() {
        use codelet_providers::copilot::CopilotProvider;

        let _create_rig_agent_ref =
            |provider: &CopilotProvider,
             session_id: uuid::Uuid,
             preamble: Option<&str>,
             thinking: Option<serde_json::Value>| {
                provider.create_rig_agent(session_id, preamble, thinking)
            };
        assert!(agent_loop_dispatch_supports_provider("github-copilot"));
    }
}
