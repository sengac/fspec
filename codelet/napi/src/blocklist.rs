//! NAPI bindings for blocklist operations
//!
//! Feature: spec/features/blocklist-core-command-tool-filtering.feature
//!
//! Exposes blocklist functionality to TypeScript for configuration management
//! and command checking.

use codelet_tools::blocklist::{
    BlocklistAction, BlocklistConfig, BlocklistRule, CheckResult,
    init_blocklist, load_blocklist_config, project_config_path, system_config_path,
    check_command_raw,
};
use napi::Result;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// JavaScript-friendly blocklist rule structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct JsBlocklistRule {
    /// Unique identifier for the rule
    pub id: String,
    /// Regex pattern to match against commands
    pub pattern: String,
    /// Action: "block", "allow", or "prompt"
    pub action: String,
    /// Reason for blocking (shown to AI)
    pub reason: String,
    /// Guidance on what to do instead (educational)
    pub guidance: Option<String>,
}

impl From<BlocklistRule> for JsBlocklistRule {
    fn from(rule: BlocklistRule) -> Self {
        Self {
            id: rule.id,
            pattern: rule.pattern,
            action: match rule.action {
                BlocklistAction::Block => "block".to_string(),
                BlocklistAction::Allow => "allow".to_string(),
                BlocklistAction::Prompt => "prompt".to_string(),
            },
            reason: rule.reason,
            guidance: rule.guidance,
        }
    }
}

impl From<JsBlocklistRule> for BlocklistRule {
    fn from(rule: JsBlocklistRule) -> Self {
        Self {
            id: rule.id,
            pattern: rule.pattern,
            action: match rule.action.as_str() {
                "allow" => BlocklistAction::Allow,
                "prompt" => BlocklistAction::Prompt,
                _ => BlocklistAction::Block,
            },
            reason: rule.reason,
            guidance: rule.guidance,
        }
    }
}

/// JavaScript-friendly blocklist config structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct JsBlocklistConfig {
    /// Version of the config schema
    pub version: String,
    /// List of blocklist rules
    pub rules: Vec<JsBlocklistRule>,
}

impl From<BlocklistConfig> for JsBlocklistConfig {
    fn from(config: BlocklistConfig) -> Self {
        Self {
            version: config.version,
            rules: config.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<JsBlocklistConfig> for BlocklistConfig {
    fn from(config: JsBlocklistConfig) -> Self {
        Self {
            version: config.version,
            rules: config.rules.into_iter().map(Into::into).collect(),
        }
    }
}

/// JavaScript-friendly check result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct JsCheckResult {
    /// Whether the command is allowed to execute
    pub allowed: bool,
    /// Whether the command is blocked
    pub blocked: bool,
    /// Reason for blocking (if blocked)
    pub reason: Option<String>,
    /// Guidance on what to do instead (if blocked)
    pub guidance: Option<String>,
    /// ID of the matching rule (if any)
    pub matched_rule_id: Option<String>,
}

impl From<CheckResult> for JsCheckResult {
    fn from(result: CheckResult) -> Self {
        Self {
            allowed: result.allowed,
            blocked: result.blocked,
            reason: result.reason,
            guidance: result.guidance,
            matched_rule_id: result.matched_rule_id,
        }
    }
}

/// Initialize the blocklist system with the project root.
/// Should be called once at application startup.
///
/// @param projectRoot - Path to the project root (optional, for project-specific rules)
#[napi]
pub fn blocklist_init(project_root: Option<String>) -> Result<()> {
    let path = project_root.map(PathBuf::from);
    init_blocklist(path.as_deref());
    Ok(())
}

/// Load blocklist configuration from system and project paths.
/// Returns merged config with project rules taking precedence.
///
/// @param projectRoot - Path to the project root (optional)
/// @returns Merged blocklist configuration
#[napi]
pub fn blocklist_load(project_root: Option<String>) -> Result<JsBlocklistConfig> {
    let path = project_root.map(PathBuf::from);
    let config = load_blocklist_config(path.as_deref());
    Ok(config.into())
}

/// Save blocklist configuration to the project config file.
///
/// @param projectRoot - Path to the project root
/// @param config - Blocklist configuration to save
#[napi]
pub fn blocklist_save(project_root: String, config: JsBlocklistConfig) -> Result<()> {
    let path = project_config_path(&PathBuf::from(&project_root));
    let rust_config: BlocklistConfig = config.into();
    rust_config.save_to_file(&path).map_err(|e| {
        napi::Error::from_reason(format!("Failed to save blocklist config: {}", e))
    })
}

/// Check a command against the blocklist.
/// Returns the check result with blocked status, reason, and guidance.
///
/// @param command - The bash command to check
/// @returns Check result
#[napi]
pub fn blocklist_check(command: String) -> Result<JsCheckResult> {
    let result = check_command_raw(&command);
    Ok(result.into())
}

/// Get the system blocklist config path (~/.fspec/blocklist.json)
///
/// @returns Path to system config, or null if home directory cannot be determined
#[napi]
pub fn blocklist_system_path() -> Option<String> {
    system_config_path().map(|p| p.to_string_lossy().to_string())
}

/// Get the project blocklist config path (.fspec/blocklist.json)
///
/// @param projectRoot - Path to the project root
/// @returns Path to project config
#[napi]
pub fn blocklist_project_path(project_root: String) -> String {
    project_config_path(&PathBuf::from(&project_root))
        .to_string_lossy()
        .to_string()
}

// ============================================================================
// SESSION ALLOWANCES (BLOCK-005)
// ============================================================================

/// Add a pattern to session allowances.
/// Called when user selects "Allow Session" in the triple confirmation dialog.
/// The pattern will be allowed for the remainder of the session (until TUI restart).
///
/// @param pattern - The pattern to allow for the session
#[napi]
pub fn blocklist_allow_session(pattern: String) -> Result<()> {
    codelet_tools::blocklist::allow_for_session(&pattern);
    Ok(())
}

/// Check if a pattern is already allowed for the current session.
/// Used to skip the prompt dialog if user previously selected "Allow Session".
///
/// @param pattern - The pattern to check
/// @returns true if the pattern is allowed, false otherwise
#[napi]
pub fn blocklist_is_session_allowed(pattern: String) -> bool {
    codelet_tools::blocklist::is_session_allowed(&pattern)
}

/// Clear all session allowances.
/// Called on TUI restart to reset session memory.
#[napi]
pub fn blocklist_clear_session_allowances() -> Result<()> {
    codelet_tools::blocklist::clear_session_allowances();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_blocklist_rule_conversion() {
        let js_rule = JsBlocklistRule {
            id: "test".to_string(),
            pattern: "^test".to_string(),
            action: "block".to_string(),
            reason: "Test reason".to_string(),
            guidance: Some("Test guidance".to_string()),
        };

        let rust_rule: BlocklistRule = js_rule.clone().into();
        assert_eq!(rust_rule.id, "test");
        assert!(matches!(rust_rule.action, BlocklistAction::Block));

        let back: JsBlocklistRule = rust_rule.into();
        assert_eq!(back.id, js_rule.id);
        assert_eq!(back.action, "block");
    }

    #[test]
    fn test_action_conversion() {
        let block = JsBlocklistRule {
            id: "1".to_string(),
            pattern: "".to_string(),
            action: "block".to_string(),
            reason: "".to_string(),
            guidance: None,
        };
        assert!(matches!(BlocklistRule::from(block).action, BlocklistAction::Block));

        let allow = JsBlocklistRule {
            id: "2".to_string(),
            pattern: "".to_string(),
            action: "allow".to_string(),
            reason: "".to_string(),
            guidance: None,
        };
        assert!(matches!(BlocklistRule::from(allow).action, BlocklistAction::Allow));

        let prompt = JsBlocklistRule {
            id: "3".to_string(),
            pattern: "".to_string(),
            action: "prompt".to_string(),
            reason: "".to_string(),
            guidance: None,
        };
        assert!(matches!(BlocklistRule::from(prompt).action, BlocklistAction::Prompt));
    }
}
