//! Blocklist Configuration Types
//!
//! Defines the data structures for blocklist rules and configuration.

use serde::{Deserialize, Serialize};

/// Action to take when a rule matches
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlocklistAction {
    /// Block the command and return an error
    Block,
    /// Allow the command (used for overrides)
    Allow,
    /// Prompt the user for confirmation (for BLOCK-005)
    Prompt,
}

/// A single blocklist rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistRule {
    /// Unique identifier for the rule
    pub id: String,
    /// Regex pattern to match against commands
    pub pattern: String,
    /// Action to take when pattern matches
    pub action: BlocklistAction,
    /// Reason for blocking (shown to AI)
    pub reason: String,
    /// Guidance on what to do instead (educational)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
}

/// Blocklist configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistConfig {
    /// Version of the config schema
    pub version: String,
    /// List of blocklist rules
    pub rules: Vec<BlocklistRule>,
}

impl BlocklistConfig {
    /// Create an empty configuration
    pub fn empty() -> Self {
        Self {
            version: "1.0.0".to_string(),
            rules: vec![],
        }
    }

    /// Merge system and project configs.
    /// Project rules are checked first (more specific overrides).
    /// Rules with action=Allow can override blocking rules.
    pub fn merge(system: BlocklistConfig, project: BlocklistConfig) -> Self {
        let mut merged_rules = Vec::new();

        // Project rules first (higher priority, more specific overrides)
        merged_rules.extend(project.rules);

        // Then system rules
        merged_rules.extend(system.rules);

        Self {
            version: project.version,
            rules: merged_rules,
        }
    }

    /// Load config from a JSON file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let config: BlocklistConfig = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }

    /// Save config to a JSON file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "test-rule".to_string(),
                pattern: "test.*".to_string(),
                action: BlocklistAction::Block,
                reason: "Test reason".to_string(),
                guidance: Some("Test guidance".to_string()),
            }],
        };

        let json = serde_json::to_string_pretty(&config).expect("serialization should succeed");
        assert!(json.contains("test-rule"));
        assert!(json.contains("block"));

        let parsed: BlocklistConfig =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.version, "1.0.0");
        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(parsed.rules[0].action, BlocklistAction::Block);
    }

    #[test]
    fn test_config_merge() {
        let system = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "system-rule".to_string(),
                pattern: "rm -rf".to_string(),
                action: BlocklistAction::Block,
                reason: "Dangerous".to_string(),
                guidance: None,
            }],
        };

        let project = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "project-override".to_string(),
                pattern: "rm -rf ./node_modules".to_string(),
                action: BlocklistAction::Allow,
                reason: "".to_string(),
                guidance: None,
            }],
        };

        let merged = BlocklistConfig::merge(system, project);

        // Project rules should be first (higher priority)
        assert_eq!(merged.rules.len(), 2);
        assert_eq!(merged.rules[0].id, "project-override");
        assert_eq!(merged.rules[1].id, "system-rule");
    }
}
