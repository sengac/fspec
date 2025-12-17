//! Provider Manager for dynamic provider selection
//!
//! Handles credential detection and provider instantiation based on:
//! - Available credentials (environment variables and auth files)
//! - CLI arguments (--provider flag)
//! - Priority order: Claude API > Claude OAuth > Gemini > Codex > OpenAI

use super::credentials::ProviderCredentials;
use super::{
    claude, codex, gemini, openai, ClaudeProvider, CodexProvider, GeminiProvider, OpenAIProvider,
    ProviderError,
};
use std::str::FromStr;

/// Provider type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
}

impl FromStr for ProviderType {
    type Err = ProviderError;

    fn from_str(name: &str) -> Result<Self, ProviderError> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" => Ok(ProviderType::Gemini),
            _ => Err(ProviderError::config("manager", format!("Unknown provider: {name}"))),
        }
    }
}

impl ProviderType {
    /// Get provider name as string
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderType::Claude => "claude",
            ProviderType::OpenAI => "openai",
            ProviderType::Codex => "codex",
            ProviderType::Gemini => "gemini",
        }
    }
}

/// Provider Manager for dynamic provider selection
#[derive(Debug)]
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
}

impl ProviderManager {
    /// Create new ProviderManager with automatic provider selection
    ///
    /// Priority order: Claude API > Claude OAuth > Gemini > Codex > OpenAI
    pub fn new() -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();

        if !credentials.has_any() {
            return Err(ProviderError::auth(
                "manager",
                "No provider credentials found. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, \
                 GOOGLE_GENERATIVE_AI_API_KEY, or run 'codex auth login' to authenticate.",
            ));
        }

        let current_provider = Self::detect_default_provider(&credentials)?;

        Ok(Self {
            credentials,
            current_provider,
        })
    }

    /// Create ProviderManager with explicit provider selection
    pub fn with_provider(provider_name: &str) -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        let has_credentials = match requested_provider {
            ProviderType::Claude => credentials.has_claude(),
            ProviderType::OpenAI => credentials.has_openai(),
            ProviderType::Codex => credentials.has_codex(),
            ProviderType::Gemini => credentials.has_gemini(),
        };

        if !has_credentials {
            let available = credentials.available_providers();
            return Err(ProviderError::auth(
                provider_name,
                format!(
                    "Provider {} not available. Available providers: {}",
                    provider_name,
                    available.join(", ")
                ),
            ));
        }

        Ok(Self {
            credentials,
            current_provider: requested_provider,
        })
    }

    /// Detect default provider based on priority
    fn detect_default_provider(credentials: &ProviderCredentials) -> Result<ProviderType, ProviderError> {
        // Priority: Claude > Gemini > Codex > OpenAI
        if credentials.has_claude() {
            return Ok(ProviderType::Claude);
        }
        if credentials.has_gemini() {
            return Ok(ProviderType::Gemini);
        }
        if credentials.has_codex() {
            return Ok(ProviderType::Codex);
        }
        if credentials.has_openai() {
            return Ok(ProviderType::OpenAI);
        }

        Err(ProviderError::auth("manager", "No provider credentials available"))
    }

    /// Get current provider name
    pub fn current_provider_name(&self) -> &str {
        self.current_provider.as_str()
    }

    /// Get Claude provider (if selected)
    pub fn get_claude(&self) -> Result<ClaudeProvider, ProviderError> {
        if self.current_provider == ProviderType::Claude {
            ClaudeProvider::new()
        } else {
            Err(ProviderError::config("manager", "Current provider is not Claude"))
        }
    }

    /// Get OpenAI provider (if selected)
    pub fn get_openai(&self) -> Result<OpenAIProvider, ProviderError> {
        if self.current_provider == ProviderType::OpenAI {
            OpenAIProvider::new()
        } else {
            Err(ProviderError::config("manager", "Current provider is not OpenAI"))
        }
    }

    /// Get Codex provider (if selected)
    pub fn get_codex(&self) -> Result<CodexProvider, ProviderError> {
        if self.current_provider == ProviderType::Codex {
            CodexProvider::new()
        } else {
            Err(ProviderError::config("manager", "Current provider is not Codex"))
        }
    }

    /// Get Gemini provider (if selected)
    pub fn get_gemini(&self) -> Result<GeminiProvider, ProviderError> {
        if self.current_provider == ProviderType::Gemini {
            GeminiProvider::new()
        } else {
            Err(ProviderError::config("manager", "Current provider is not Gemini"))
        }
    }

    /// Check if any provider is available
    pub fn has_any_provider(&self) -> bool {
        self.credentials.has_any()
    }

    /// List all available providers for display
    pub fn list_available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.credentials.has_claude() {
            providers.push("Claude (/claude)".to_string());
        }
        if self.credentials.has_openai() {
            providers.push("OpenAI (/openai)".to_string());
        }
        if self.credentials.has_gemini() {
            providers.push("Gemini (/gemini)".to_string());
        }
        if self.credentials.has_codex() {
            providers.push("Codex (/codex)".to_string());
        }
        providers
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<(), ProviderError> {
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        let has_credentials = match requested_provider {
            ProviderType::Claude => self.credentials.has_claude(),
            ProviderType::OpenAI => self.credentials.has_openai(),
            ProviderType::Codex => self.credentials.has_codex(),
            ProviderType::Gemini => self.credentials.has_gemini(),
        };

        if !has_credentials {
            return Err(ProviderError::auth(
                provider_name,
                format!("Provider {provider_name} not available. No credentials found."),
            ));
        }

        self.current_provider = requested_provider;
        Ok(())
    }

    /// Get prompt prefix for REPL (e.g., "[claude] > ")
    pub fn get_prompt_prefix(&self) -> String {
        format!("[{}] > ", self.current_provider.as_str())
    }

    /// Get context window size for the current provider
    ///
    /// Returns the context window in tokens for the currently selected provider.
    /// References the canonical CONTEXT_WINDOW constant from each provider module.
    pub fn context_window(&self) -> usize {
        match self.current_provider {
            ProviderType::Claude => claude::CONTEXT_WINDOW,
            ProviderType::OpenAI => openai::CONTEXT_WINDOW,
            ProviderType::Gemini => gemini::CONTEXT_WINDOW,
            ProviderType::Codex => codex::CONTEXT_WINDOW,
        }
    }
}
