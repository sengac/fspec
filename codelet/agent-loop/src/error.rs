//! Error type for the agent loop (RPC-072).

use thiserror::Error;

/// Errors that can occur while the agent loop runs one turn.
#[derive(Debug, Error)]
pub enum AgentLoopError {
    /// The session's currently selected provider could not be built.
    /// Usually means credentials are missing or the provider rejected
    /// the model id at construction time.
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),

    /// The provider returned an error from `complete_with_tools`.
    #[error("provider error: {0}")]
    ProviderError(String),
}
