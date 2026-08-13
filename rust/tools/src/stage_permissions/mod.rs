//! Stage Permissions Module - ACDD File Write Enforcement
//!
//! Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
//!
//! This module implements stage-based file write permissions for ACDD workflow.
//! It prevents AI agents from writing certain file types based on the current
//! work unit's ACDD stage (backlog, specifying, testing, implementing, validating, done).
//!
//! ## Architecture
//!
//! - `config.rs`: StagePermissionsConfig, FileCategory, StagePermission types
//! - `matcher.rs`: StagePermissionsMatcher for glob pattern evaluation
//!
//! ## Config Hierarchy
//!
//! 1. System config: `~/.fspec/stage-permissions.json`
//! 2. Project config: `.fspec/stage-permissions.json`
//! 3. Project rules extend/override system rules
//!
//! ## Default Permissions
//!
//! - backlog: spec only
//! - specifying: spec only
//! - testing: spec, test
//! - implementing: spec, test, impl
//! - validating: nothing
//! - done: nothing

mod config;
mod matcher;

pub use config::{FileCategory, StagePermission, StagePermissionsConfig};
pub use matcher::{StageCheckResult, StagePermissionsMatcher};

use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::warn;

/// Global stage permissions matcher instance
/// Uses RwLock to allow reinitialization (e.g., when project changes)
static STAGE_PERMISSIONS_MATCHER: RwLock<Option<StagePermissionsMatcher>> = RwLock::new(None);

/// Error type for blocked file writes
#[derive(Debug)]
pub struct StageBlockedError {
    pub reason: String,
    pub guidance: Option<String>,
    pub category: String,
    pub stage: String,
}

impl std::fmt::Display for StageBlockedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)?;
        if let Some(ref guidance) = self.guidance {
            write!(f, " {guidance}")?;
        }
        Ok(())
    }
}

impl std::error::Error for StageBlockedError {}

/// Get the system stage permissions config path (~/.fspec/stage-permissions.json)
pub fn system_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".fspec").join("stage-permissions.json"))
}

/// Get the project stage permissions config path (.fspec/stage-permissions.json relative to project root)
pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".fspec").join("stage-permissions.json")
}

/// Load stage permissions configuration from system and project paths.
/// Project config takes precedence over system config.
/// If no config files exist, returns default ACDD permissions.
pub fn load_stage_permissions_config(project_root: Option<&Path>) -> StagePermissionsConfig {
    let mut system_config = StagePermissionsConfig::empty();
    let mut project_config = StagePermissionsConfig::empty();
    let mut any_config_found = false;

    // Load system config if it exists
    if let Some(system_path) = system_config_path() {
        if system_path.exists() {
            match StagePermissionsConfig::load_from_file(&system_path) {
                Ok(config) => {
                    system_config = config;
                    any_config_found = true;
                }
                Err(e) => {
                    warn!(
                        "Failed to load system stage permissions config from {:?}: {}",
                        system_path, e
                    );
                }
            }
        }
    }

    // Load project config if it exists
    if let Some(root) = project_root {
        let project_path = project_config_path(root);
        if project_path.exists() {
            match StagePermissionsConfig::load_from_file(&project_path) {
                Ok(config) => {
                    project_config = config;
                    any_config_found = true;
                }
                Err(e) => {
                    warn!(
                        "Failed to load project stage permissions config from {:?}: {}",
                        project_path, e
                    );
                }
            }
        }
    }

    // If no config files found, use default ACDD permissions
    if !any_config_found {
        return StagePermissionsConfig::default_acdd();
    }

    // Merge configs (project takes precedence)
    StagePermissionsConfig::merge(system_config, project_config)
}

/// Initialize the global stage permissions matcher.
/// Should be called once at startup with the project root.
/// Can be called again to reinitialize with a different project root.
pub fn init_stage_permissions(project_root: Option<&Path>) {
    let config = load_stage_permissions_config(project_root);
    let matcher = StagePermissionsMatcher::new(config);

    // Write lock to update the matcher
    // If lock is poisoned, we recover by taking the poisoned guard
    let mut guard = match STAGE_PERMISSIONS_MATCHER.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(matcher);
}

/// Check if a file write is allowed for the given stage.
/// Returns Ok(()) if allowed, Err(StageBlockedError) if blocked.
///
/// If stage is empty/None, allows all writes (no work unit active).
/// If stage permissions are not initialized, allows all writes.
pub fn check_write_permission(path: &str, stage: Option<&str>) -> Result<(), StageBlockedError> {
    // If no stage (no work unit active), allow all writes
    let stage = match stage {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    // Read lock to access the matcher
    // If lock is poisoned, we recover by taking the poisoned guard
    let guard = match STAGE_PERMISSIONS_MATCHER.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    // If matcher not initialized, allow all writes
    if let Some(ref matcher) = *guard {
        let result = matcher.check_write(path, stage);

        if result.blocked {
            return Err(StageBlockedError {
                reason: result.reason.unwrap_or_else(|| "Write blocked".to_string()),
                guidance: result.guidance,
                category: result.matched_category.unwrap_or_default(),
                stage: result.stage,
            });
        }
    }

    Ok(())
}

/// Check a file write and return the raw result.
/// This is used by NAPI bindings that need the full StageCheckResult.
pub fn check_write_raw(path: &str, stage: &str) -> StageCheckResult {
    // Read lock to access the matcher
    // If lock is poisoned, we recover by taking the poisoned guard
    let guard = match STAGE_PERMISSIONS_MATCHER.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some(ref matcher) = *guard {
        matcher.check_write(path, stage)
    } else {
        StageCheckResult::allowed(None, stage.to_string())
    }
}

/// Reload the stage permissions configuration.
pub fn reload_stage_permissions(project_root: Option<&Path>) {
    init_stage_permissions(project_root);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(dir: &Path) {
        let fspec_dir = dir.join(".fspec");
        std::fs::create_dir_all(&fspec_dir).expect("failed to create .fspec directory");

        let config = StagePermissionsConfig::default_acdd();
        let path = fspec_dir.join("stage-permissions.json");
        let mut file =
            std::fs::File::create(&path).expect("failed to create stage-permissions.json");
        let json = serde_json::to_string_pretty(&config).expect("failed to serialize config");
        file.write_all(json.as_bytes())
            .expect("failed to write stage-permissions.json");
    }

    #[test]
    fn test_load_default_config_when_no_files() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let config = load_stage_permissions_config(Some(tmp.path()));

        // Should use default ACDD permissions
        assert!(!config.categories.is_empty());
        assert!(!config.permissions.is_empty());
    }

    #[test]
    fn test_load_project_config() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path());

        let config = load_stage_permissions_config(Some(tmp.path()));

        assert!(!config.categories.is_empty());
        assert!(!config.permissions.is_empty());
    }

    #[test]
    fn test_stage_blocked_error_display() {
        let error = StageBlockedError {
            reason: "Cannot write implementation files in testing stage".to_string(),
            guidance: Some("Move to implementing stage first.".to_string()),
            category: "impl".to_string(),
            stage: "testing".to_string(),
        };

        assert_eq!(
            error.to_string(),
            "Cannot write implementation files in testing stage Move to implementing stage first."
        );
    }

    #[test]
    fn test_stage_blocked_error_display_without_guidance() {
        let error = StageBlockedError {
            reason: "Write blocked".to_string(),
            guidance: None,
            category: "impl".to_string(),
            stage: "testing".to_string(),
        };

        assert_eq!(error.to_string(), "Write blocked");
    }

    #[test]
    fn test_check_write_permission_no_stage() {
        // No stage means no work unit active - should allow all
        let result = check_write_permission("src/auth.ts", None);
        assert!(result.is_ok());

        let result = check_write_permission("src/auth.ts", Some(""));
        assert!(result.is_ok());
    }
}
