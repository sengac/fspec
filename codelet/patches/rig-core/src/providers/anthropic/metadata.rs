//! Claude Code metadata handling for OAuth authentication
//!
//! Reads user_id, account_uuid, and session_id from ~/.claude.json
//! to construct the metadata.user_id field required for OAuth requests.

use serde::Deserialize;
use std::path::PathBuf;

/// OAuth account information from ~/.claude.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthAccount {
    account_uuid: String,
}

/// Project information from ~/.claude.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInfo {
    last_session_id: Option<String>,
}

/// Root structure for ~/.claude.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeConfig {
    /// The hashed user ID
    #[serde(rename = "userID")]
    user_id: Option<String>,
    /// OAuth account info
    oauth_account: Option<OAuthAccount>,
    /// Projects map (path -> project info)
    projects: Option<std::collections::HashMap<String, ProjectInfo>>,
}

/// Get the path to ~/.claude.json
fn get_claude_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude.json"))
}

/// Read and parse Claude Code metadata from ~/.claude.json
///
/// Returns the user_id in the format expected by Claude Code OAuth:
/// `user_{userID}_account_{accountUuid}_session_{sessionId}`
///
/// Falls back to generated values if the file is not found or parsing fails.
pub fn get_oauth_user_id() -> String {
    // Try to read from ~/.claude.json
    if let Some(config_path) = get_claude_config_path() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<ClaudeConfig>(&content) {
                // Extract user_id
                let user_id = config.user_id.unwrap_or_else(|| {
                    // Fallback: generate a hash
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    "codelet_user".hash(&mut hasher);
                    format!("{:x}", hasher.finish())
                });

                // Extract account_uuid
                let account_uuid = config
                    .oauth_account
                    .map(|a| a.account_uuid)
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                // Try to get session_id from current working directory's project
                let session_id = config
                    .projects
                    .and_then(|projects| {
                        // Try to find a session_id from any project (prefer current dir)
                        std::env::current_dir()
                            .ok()
                            .and_then(|cwd| {
                                projects
                                    .get(cwd.to_string_lossy().as_ref())
                                    .and_then(|p| p.last_session_id.clone())
                            })
                            .or_else(|| {
                                // Fallback: try to find any project with a session_id
                                projects
                                    .values()
                                    .find_map(|p| p.last_session_id.clone())
                            })
                    })
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                return format!("user_{}_account_{}_session_{}", user_id, account_uuid, session_id);
            }
        }
    }

    // Fallback: generate random values (existing behavior)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    "codelet_user".hash(&mut hasher);
    let user_hash = format!("{:x}", hasher.finish());

    let account_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();

    format!(
        "user_{}_account_{}_session_{}",
        user_hash, account_id, session_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_oauth_user_id_format() {
        let user_id = get_oauth_user_id();

        // Should be in format: user_<hash>_account_<uuid>_session_<uuid>
        assert!(user_id.starts_with("user_"), "Should start with 'user_': {}", user_id);
        assert!(user_id.contains("_account_"), "Should contain '_account_': {}", user_id);
        assert!(user_id.contains("_session_"), "Should contain '_session_': {}", user_id);
    }
}
