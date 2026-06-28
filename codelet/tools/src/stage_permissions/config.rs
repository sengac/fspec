//! Stage Permissions Configuration
//!
//! Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
//!
//! Defines configuration types for ACDD stage-based file write permissions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// File category: named group of glob patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCategory {
    /// Category name (e.g., "spec", "test", "impl")
    pub name: String,
    /// Glob patterns for files in this category
    /// Supports negation with "!" prefix (e.g., "!src/**/*.test.ts")
    pub patterns: Vec<String>,
}

/// Stage permissions: which categories are writable in each stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagePermission {
    /// ACDD stage name (e.g., "testing", "implementing")
    pub stage: String,
    /// Names of categories that are writable in this stage
    pub writable_categories: Vec<String>,
}

/// Stage permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagePermissionsConfig {
    /// Configuration version
    pub version: String,
    /// File categories (named groups of glob patterns)
    pub categories: Vec<FileCategory>,
    /// Stage permissions (which categories are writable in each stage)
    pub permissions: Vec<StagePermission>,
}

impl StagePermissionsConfig {
    /// Create an empty configuration
    pub fn empty() -> Self {
        Self {
            version: "1.0.0".to_string(),
            categories: Vec::new(),
            permissions: Vec::new(),
        }
    }

    /// Create default ACDD permissions
    pub fn default_acdd() -> Self {
        Self {
            version: "1.0.0".to_string(),
            categories: vec![
                FileCategory {
                    name: "spec".to_string(),
                    patterns: vec![
                        "spec/**/*.feature".to_string(),
                        "spec/**/*.md".to_string(),
                        "spec/attachments/**".to_string(),
                        "spec/**/*.json".to_string(),
                    ],
                },
                FileCategory {
                    name: "test".to_string(),
                    patterns: vec![
                        "src/**/*.test.ts".to_string(),
                        "src/**/__tests__/**".to_string(),
                        "**/*.spec.ts".to_string(),
                        "**/*.test.rs".to_string(),
                    ],
                },
                FileCategory {
                    name: "impl".to_string(),
                    patterns: vec![
                        "src/**/*.ts".to_string(),
                        "src/**/*.rs".to_string(),
                        "!src/**/*.test.ts".to_string(),
                        "!src/**/__tests__/**".to_string(),
                    ],
                },
            ],
            permissions: vec![
                StagePermission {
                    stage: "backlog".to_string(),
                    writable_categories: vec!["spec".to_string()],
                },
                StagePermission {
                    stage: "specifying".to_string(),
                    writable_categories: vec!["spec".to_string()],
                },
                StagePermission {
                    stage: "testing".to_string(),
                    writable_categories: vec!["spec".to_string(), "test".to_string()],
                },
                StagePermission {
                    stage: "implementing".to_string(),
                    writable_categories: vec![
                        "spec".to_string(),
                        "test".to_string(),
                        "impl".to_string(),
                    ],
                },
                StagePermission {
                    stage: "validating".to_string(),
                    writable_categories: vec![],
                },
                StagePermission {
                    stage: "done".to_string(),
                    writable_categories: vec![],
                },
            ],
        }
    }

    /// Load configuration from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Merge system and project configurations
    /// Project config takes precedence for categories and permissions with the same name/stage
    pub fn merge(system: Self, project: Self) -> Self {
        let mut categories: HashMap<String, FileCategory> = HashMap::new();
        let mut permissions: HashMap<String, StagePermission> = HashMap::new();

        // Add system categories first
        for cat in system.categories {
            categories.insert(cat.name.clone(), cat);
        }
        // Project categories override
        for cat in project.categories {
            categories.insert(cat.name.clone(), cat);
        }

        // Add system permissions first
        for perm in system.permissions {
            permissions.insert(perm.stage.clone(), perm);
        }
        // Project permissions override
        for perm in project.permissions {
            permissions.insert(perm.stage.clone(), perm);
        }

        Self {
            version: if project.version.is_empty() {
                system.version
            } else {
                project.version
            },
            categories: categories.into_values().collect(),
            permissions: permissions.into_values().collect(),
        }
    }

    /// Get writable categories for a given stage
    pub fn get_writable_categories(&self, stage: &str) -> Vec<&str> {
        self.permissions
            .iter()
            .find(|p| p.stage == stage)
            .map(|p| p.writable_categories.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Project config overrides system config
    #[test]
    fn test_merge_configs() {
        // @step Given system config has default categories
        let system = StagePermissionsConfig {
            version: "1.0.0".to_string(),
            categories: vec![FileCategory {
                name: "spec".to_string(),
                patterns: vec!["spec/**".to_string()],
            }],
            permissions: vec![StagePermission {
                stage: "testing".to_string(),
                writable_categories: vec!["spec".to_string()],
            }],
        };

        // @step And project config overrides the testing stage to include tests
        let project = StagePermissionsConfig {
            version: "1.0.0".to_string(),
            categories: vec![FileCategory {
                name: "test".to_string(),
                patterns: vec!["src/**/*.test.ts".to_string()],
            }],
            permissions: vec![StagePermission {
                stage: "testing".to_string(),
                writable_categories: vec!["spec".to_string(), "test".to_string()],
            }],
        };

        // @step When configs are merged
        let merged = StagePermissionsConfig::merge(system, project);

        // @step Then the merged config should have both categories
        assert_eq!(merged.categories.len(), 2);

        // @step And the testing stage should allow spec and test categories
        let writable = merged.get_writable_categories("testing");
        assert!(writable.contains(&"spec"));
        assert!(writable.contains(&"test"));
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Default ACDD permissions
    #[test]
    fn test_default_acdd_permissions() {
        // @step Given default ACDD stage permissions
        let config = StagePermissionsConfig::default_acdd();

        // @step Then backlog stage allows only spec category
        let backlog = config.get_writable_categories("backlog");
        assert_eq!(backlog, vec!["spec"]);

        // @step And specifying stage allows only spec category
        let specifying = config.get_writable_categories("specifying");
        assert_eq!(specifying, vec!["spec"]);

        // @step And testing stage allows spec and test categories
        let testing = config.get_writable_categories("testing");
        assert!(testing.contains(&"spec"));
        assert!(testing.contains(&"test"));
        assert!(!testing.contains(&"impl"));

        // @step And implementing stage allows spec, test, and impl categories
        let implementing = config.get_writable_categories("implementing");
        assert!(implementing.contains(&"spec"));
        assert!(implementing.contains(&"test"));
        assert!(implementing.contains(&"impl"));

        // @step And validating stage allows nothing
        let validating = config.get_writable_categories("validating");
        assert!(validating.is_empty());

        // @step And done stage allows nothing
        let done = config.get_writable_categories("done");
        assert!(done.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let config = StagePermissionsConfig::empty();
        assert!(config.categories.is_empty());
        assert!(config.permissions.is_empty());
    }

    #[test]
    fn test_get_writable_categories_unknown_stage() {
        let config = StagePermissionsConfig::default_acdd();
        let unknown = config.get_writable_categories("unknown_stage");
        assert!(unknown.is_empty());
    }
}
