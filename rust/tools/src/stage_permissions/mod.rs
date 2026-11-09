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

/// The system stage permissions config path,
/// `<user dir>/stage-permissions.json` (BUG-656: the one user-directory
/// resolver).
pub fn system_config_path() -> Result<PathBuf, String> {
    codelet_common::user_dir::fspec_user_dir()
        .map(|directory| directory.join("stage-permissions.json"))
}

/// Get the project stage permissions config path (.fspec/stage-permissions.json
/// relative to the project root; WT-015: shared state, so an isolated copy
/// holding a state-root pointer reads the main checkout's).
pub fn project_config_path(project_root: &Path) -> Result<PathBuf, String> {
    codelet_common::spec_paths::ProjectRoots::resolve(project_root)
        .map(|roots| roots.stage_permissions())
        .map_err(|error| format!("cannot resolve the stage permissions config: {error}"))
}

/// Load stage permissions configuration from system and project paths.
/// Project config takes precedence over system config.
/// If no config files exist, returns default ACDD permissions.
pub fn load_stage_permissions_config(
    project_root: Option<&Path>,
) -> Result<StagePermissionsConfig, String> {
    // BUG-449: an unreadable or invalid config file is an error, never
    // silently replaced by the defaults.
    let load = |path: PathBuf| -> Result<Option<StagePermissionsConfig>, String> {
        if !path.exists() {
            return Ok(None);
        }
        StagePermissionsConfig::load_from_file(&path)
            .map(Some)
            .map_err(|error| {
                format!(
                    "cannot load the stage permissions config {}: {error}",
                    path.display()
                )
            })
    };
    let system = load(system_config_path()?)?;
    let project = match project_root {
        Some(root) => load(project_config_path(root)?)?,
        None => None,
    };
    // No config file at all: the default ACDD permissions (BLOCK-006).
    Ok(match (system, project) {
        (None, None) => StagePermissionsConfig::default_acdd(),
        (system, project) => StagePermissionsConfig::merge(
            system.unwrap_or_else(StagePermissionsConfig::empty),
            project.unwrap_or_else(StagePermissionsConfig::empty),
        ),
    })
}

/// Initialize the global stage permissions matcher.
/// Should be called once at startup with the project root.
/// Can be called again to reinitialize with a different project root.
pub fn init_stage_permissions(project_root: Option<&Path>) -> Result<(), String> {
    let config = load_stage_permissions_config(project_root)?;
    let matcher = StagePermissionsMatcher::new(config);

    // Write lock to update the matcher
    // If lock is poisoned, we recover by taking the poisoned guard
    let mut guard = match STAGE_PERMISSIONS_MATCHER.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(matcher);
    Ok(())
}

/// Check if a file write is allowed for the given stage.
/// Returns Ok(()) if allowed, Err(StageBlockedError) if blocked.
///
/// If stage is empty/None, allows all writes (no work unit active).
/// BUG-449: if stage permissions are not initialized, the write is refused.
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

    // BUG-449: an uninitialized matcher refuses (sessions use stage_gate).
    let Some(ref matcher) = *guard else {
        return Err(StageBlockedError {
            reason: "stage permissions are not initialized".to_string(),
            guidance: None,
            category: String::new(),
            stage: stage.to_string(),
        });
    };
    {
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
pub fn reload_stage_permissions(project_root: Option<&Path>) -> Result<(), String> {
    init_stage_permissions(project_root)
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
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
        let tmp = TempDir::new().expect("failed to create temp dir");
        let config = load_stage_permissions_config(Some(tmp.path())).expect("the config loads");

        // Should use default ACDD permissions
        assert!(!config.categories.is_empty());
        assert!(!config.permissions.is_empty());
    }

    #[test]
    fn test_load_project_config() {
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
        let tmp = TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path());

        let config = load_stage_permissions_config(Some(tmp.path())).expect("the config loads");

        assert!(!config.categories.is_empty());
        assert!(!config.permissions.is_empty());
    }

    #[test]
    fn test_stage_blocked_error_display() {
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
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
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
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
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
        // No stage means no work unit active - should allow all
        let result = check_write_permission("src/auth.ts", None);
        assert!(result.is_ok());

        let result = check_write_permission("src/auth.ts", Some(""));
        assert!(result.is_ok());
    }
}
