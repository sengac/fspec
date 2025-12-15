//! Provider credential detection module
//!
//! Detects available LLM provider credentials from:
//! - Environment variables (ANTHROPIC_API_KEY, CLAUDE_CODE_OAUTH_TOKEN, OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY)
//! - Auth files (~/.codex/auth.json for Codex OAuth)

/// Provider credentials detected from environment variables and auth files
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub claude_available: bool,
    pub openai_available: bool,
    pub codex_available: bool,
    pub gemini_available: bool,
}

impl ProviderCredentials {
    /// Detect all available provider credentials
    pub fn detect() -> Self {
        Self {
            claude_available: std::env::var("ANTHROPIC_API_KEY").is_ok()
                || std::env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok(),
            openai_available: std::env::var("OPENAI_API_KEY").is_ok(),
            codex_available: has_codex_auth(),
            gemini_available: std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").is_ok(),
        }
    }

    /// Check if any provider credentials are available
    pub fn has_any(&self) -> bool {
        self.claude_available
            || self.openai_available
            || self.codex_available
            || self.gemini_available
    }

    /// Check if Claude credentials are available
    pub fn has_claude(&self) -> bool {
        self.claude_available
    }

    /// Check if OpenAI credentials are available
    pub fn has_openai(&self) -> bool {
        self.openai_available
    }

    /// Check if Codex credentials are available
    pub fn has_codex(&self) -> bool {
        self.codex_available
    }

    /// Check if Gemini credentials are available
    pub fn has_gemini(&self) -> bool {
        self.gemini_available
    }

    /// List all available provider names
    pub fn available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.claude_available {
            providers.push("claude".to_string());
        }
        if self.gemini_available {
            providers.push("gemini".to_string());
        }
        if self.codex_available {
            providers.push("codex".to_string());
        }
        if self.openai_available {
            providers.push("openai".to_string());
        }
        providers
    }
}

/// Check if Codex auth.json exists with valid credentials
/// Matches codelet's hasCodexCredentials() implementation
fn has_codex_auth() -> bool {
    use crate::codex::codex_auth::read_codex_auth;

    if let Ok(Some(auth)) = read_codex_auth() {
        // Check for either cached API key or OAuth tokens
        if auth.openai_api_key.is_some() {
            return true;
        }
        if let Some(tokens) = auth.tokens {
            return !tokens.refresh_token.is_empty() && !tokens.account_id.is_empty();
        }
    }
    false
}
