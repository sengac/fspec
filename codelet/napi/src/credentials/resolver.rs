//! Credential Resolver
//!
//! Implements the credential resolution priority chain:
//! 1. Credentials file (~/.fspec/credentials/credentials.json)
//! 2. Environment variables
//! 3. Project .env file

use super::store::{get_stored_api_key, get_stored_api_key_with_dir};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Map of provider IDs to their environment variable names
/// Some providers support multiple env vars (checked in order)
fn get_provider_env_vars(provider_id: &str) -> Option<Vec<&'static str>> {
    match provider_id {
        "anthropic" => Some(vec!["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN"]),
        "openai" => Some(vec!["OPENAI_API_KEY"]),
        "cohere" => Some(vec!["COHERE_API_KEY"]),
        "gemini" => Some(vec!["GOOGLE_GENERATIVE_AI_API_KEY", "GEMINI_API_KEY"]),
        "mistral" => Some(vec!["MISTRAL_API_KEY"]),
        "xai" => Some(vec!["XAI_API_KEY"]),
        "together" => Some(vec!["TOGETHER_API_KEY"]),
        "huggingface" => Some(vec!["HUGGINGFACE_API_KEY", "HF_TOKEN"]),
        "openrouter" => Some(vec!["OPENROUTER_API_KEY"]),
        "groq" => Some(vec!["GROQ_API_KEY"]),
        "ollama" => Some(vec!["OLLAMA_API_KEY"]),
        "deepseek" => Some(vec!["DEEPSEEK_API_KEY"]),
        "perplexity" => Some(vec!["PERPLEXITY_API_KEY"]),
        "moonshot" => Some(vec!["MOONSHOT_API_KEY"]),
        "hyperbolic" => Some(vec!["HYPERBOLIC_API_KEY"]),
        "mira" => Some(vec!["MIRA_API_KEY"]),
        "galadriel" => Some(vec!["GALADRIEL_API_KEY"]),
        "azure" => Some(vec!["AZURE_OPENAI_API_KEY"]),
        "voyageai" => Some(vec!["VOYAGEAI_API_KEY"]),
        "zai" => Some(vec!["ZAI_API_KEY", "ZAI_PLAN_API_KEY"]),
        _ => None,
    }
}

/// Get primary environment variable name for a provider
pub fn get_primary_env_var(provider_id: &str) -> Option<&'static str> {
    get_provider_env_vars(provider_id).and_then(|vars| vars.first().copied())
}

/// Parse a .env file and return key-value pairs
fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on first '='
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let mut value = line[pos + 1..].trim().to_string();
            // Remove quotes if present
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            result.insert(key, value);
        }
    }
    result
}

/// Get API key from environment variables
fn get_env_api_key(provider_id: &str) -> Option<String> {
    let env_vars = get_provider_env_vars(provider_id)?;
    for env_var in env_vars {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}

/// Get API key from .env file in project directory
fn get_dotenv_api_key(provider_id: &str, project_dir: &Path) -> Option<String> {
    let env_file = project_dir.join(".env");
    if !env_file.exists() {
        return None;
    }

    let content = fs::read_to_string(&env_file).ok()?;
    let parsed = parse_dotenv(&content);

    let env_vars = get_provider_env_vars(provider_id)?;
    for env_var in env_vars {
        if let Some(key) = parsed.get(env_var) {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
    }
    None
}

/// PROV-026: Get credential from Claude OAuth tokens (claude_auth.json)
/// Returns the access_token if claude_auth.json exists with valid tokens.
fn get_claude_oauth_credential() -> Option<String> {
    use codelet_providers::claude_auth::read_claude_auth_sync;

    if let Ok(Some(auth)) = read_claude_auth_sync() {
        if !auth.access_token.is_empty() && !auth.refresh_token.is_empty() {
            return Some(auth.access_token);
        }
    }
    None
}

/// Extract provider ID from model string (e.g., "anthropic/claude-sonnet-4-20250514" -> "anthropic")
pub fn extract_provider_from_model(model: &str) -> &str {
    model.split('/').next().unwrap_or("")
}

/// Resolve credential for a provider following the priority chain:
/// 1. Credentials file (~/.fspec/credentials/credentials.json)
/// 2. Environment variables
/// 3. Project .env file (if project_dir provided)
/// 4. Claude OAuth tokens from claude_auth.json (anthropic provider only)
///
/// Returns the API key if found, None otherwise.
pub fn resolve_credential(
    provider_id: &str,
    project_dir: Option<&Path>,
    data_dir: Option<&std::path::PathBuf>,
) -> Result<Option<String>, String> {
    // 1. Try credentials file first
    let stored_key = if let Some(dir) = data_dir {
        get_stored_api_key_with_dir(provider_id, dir)?
    } else {
        get_stored_api_key(provider_id)?
    };

    if let Some(key) = stored_key {
        return Ok(Some(key));
    }

    // 2. Try environment variables
    if let Some(key) = get_env_api_key(provider_id) {
        return Ok(Some(key));
    }

    // 3. Try .env file in project directory
    if let Some(project) = project_dir {
        if let Some(key) = get_dotenv_api_key(provider_id, project) {
            return Ok(Some(key));
        }
    }

    // 4. PROV-026: Check claude_auth.json as fallback for anthropic provider
    if provider_id == "anthropic" {
        if let Some(key) = get_claude_oauth_credential() {
            return Ok(Some(key));
        }
    }

    Ok(None)
}

/// Resolve credential for session creation from model string
/// Extracts provider from model and resolves credential
pub fn resolve_credential_for_session(
    model: &str,
    data_dir: &std::path::PathBuf,
) -> Result<Option<String>, String> {
    let provider_id = extract_provider_from_model(model);
    if provider_id.is_empty() {
        return Ok(None);
    }
    resolve_credential(provider_id, None, Some(data_dir))
}

/// Resolve credential and set environment variable for rig/provider use.
/// This is called during session creation to set up the env var that providers expect.
///
/// PROV-026: For anthropic provider, when credential comes from claude_auth.json,
/// sets CLAUDE_CODE_OAUTH_TOKEN instead of ANTHROPIC_API_KEY (since it's an OAuth token).
pub fn resolve_and_set_env_var(
    provider_id: &str,
    project_dir: Option<&Path>,
) -> Result<bool, String> {
    let key = resolve_credential(provider_id, project_dir, None)?;

    if let Some(api_key) = key {
        // PROV-026: For anthropic, check if the credential is an OAuth token
        // (starts with sk-ant-oat) and set CLAUDE_CODE_OAUTH_TOKEN instead
        if provider_id == "anthropic" && api_key.starts_with("sk-ant-oat") {
            std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", &api_key);
            return Ok(true);
        }

        if let Some(env_var) = get_primary_env_var(provider_id) {
            std::env::set_var(env_var, &api_key);
            return Ok(true);
        }
    }

    Ok(false)
}

/// List of all known provider IDs
/// Used to update env vars for all providers on credential reload
pub const ALL_PROVIDER_IDS: &[&str] = &[
    "anthropic",
    "openai",
    "cohere",
    "gemini",
    "mistral",
    "xai",
    "together",
    "huggingface",
    "openrouter",
    "groq",
    "ollama",
    "deepseek",
    "perplexity",
    "moonshot",
    "hyperbolic",
    "mira",
    "galadriel",
    "azure",
    "voyageai",
    "zai",
];

/// Update environment variables for all providers that have credentials in the store
/// This is called by credentials_reload() to ensure active sessions pick up credential changes
/// immediately without requiring session restart.
///
/// User Story: "existing Rust sessions automatically pick up the new credentials
/// without requiring a session restart"
pub fn update_all_provider_env_vars() -> Result<(), String> {
    for provider_id in ALL_PROVIDER_IDS {
        // Get the API key from the credential store (which was just reloaded)
        if let Ok(Some(api_key)) = super::store::get_stored_api_key(provider_id) {
            if let Some(env_var) = get_primary_env_var(provider_id) {
                std::env::set_var(env_var, &api_key);
            }
        }
    }
    Ok(())
}
