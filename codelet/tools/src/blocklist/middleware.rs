//! Blocklist Middleware - Integration layer for tool execution filtering
//!
//! Provides blocklist loading from config files and command checking.

use super::config::BlocklistConfig;
use super::matcher::{BlocklistMatcher, CheckResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use tracing::error;

/// Global blocklist matcher instance
/// Uses RwLock instead of OnceLock to allow reinitialization (e.g., when project changes)
static BLOCKLIST_MATCHER: RwLock<Option<BlocklistMatcher>> = RwLock::new(None);

/// Stored project root for hot-reloading config
static BLOCKLIST_PROJECT_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Session allowances for patterns that user has allowed for the session
/// BLOCK-005: Cleared on TUI restart (in-memory only)
static SESSION_ALLOWANCES: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// Error type for blocked commands
#[derive(Debug)]
pub struct BlockedError {
    pub reason: String,
    pub guidance: Option<String>,
    pub rule_id: String,
}

impl std::fmt::Display for BlockedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blocked: {}", self.reason)?;
        if let Some(ref guidance) = self.guidance {
            write!(f, " {guidance}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BlockedError {}

/// Get the system blocklist config path (~/.fspec/blocklist.json)
pub fn system_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".fspec").join("blocklist.json"))
}

/// Get the project blocklist config path (.fspec/blocklist.json relative to project root)
pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".fspec").join("blocklist.json")
}

/// Load blocklist configuration from system and project paths.
/// Project config takes precedence over system config.
pub fn load_blocklist_config(project_root: Option<&Path>) -> BlocklistConfig {
    let mut system_config = BlocklistConfig::empty();
    let mut project_config = BlocklistConfig::empty();

    // Load system config if it exists
    if let Some(system_path) = system_config_path() {
        if system_path.exists() {
            match BlocklistConfig::load_from_file(&system_path) {
                Ok(config) => system_config = config,
                Err(e) => {
                    error!(
                        "Failed to load system blocklist config from {system_path:?}: {e}"
                    );
                }
            }
        }
    }

    // Load project config if it exists
    if let Some(root) = project_root {
        let project_path = project_config_path(root);
        if project_path.exists() {
            match BlocklistConfig::load_from_file(&project_path) {
                Ok(config) => project_config = config,
                Err(e) => {
                    error!(
                        "Failed to load project blocklist config from {project_path:?}: {e}"
                    );
                }
            }
        }
    }

    // Merge configs (project takes precedence)
    BlocklistConfig::merge(system_config, project_config)
}

/// Initialize the global blocklist matcher.
/// Should be called once at startup with the project root.
/// Can be called again to reinitialize with a different project root.
pub fn init_blocklist(project_root: Option<&Path>) {
    // Store project root for hot-reloading
    {
        let mut root_guard = match BLOCKLIST_PROJECT_ROOT.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *root_guard = project_root.map(Path::to_path_buf);
    }
    
    let config = load_blocklist_config(project_root);
    
    let matcher = if config.rules.is_empty() {
        None
    } else {
        Some(BlocklistMatcher::new(config))
    };
    
    let mut guard = match BLOCKLIST_MATCHER.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = matcher;
}

/// Get the stored project root for hot-reloading config
fn get_project_root() -> Option<PathBuf> {
    let guard = match BLOCKLIST_PROJECT_ROOT.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clone()
}

/// Check a bash command against the blocklist.
/// Returns Ok(()) if the command is allowed, Err(BlockedError) if blocked.
/// Reloads config on every check to pick up changes without restart.
pub fn check_bash_command(command: &str) -> Result<(), BlockedError> {
    use crate::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};
    
    let project_root = get_project_root();
    let config = load_blocklist_config(project_root.as_deref());
    
    if config.rules.is_empty() {
        return Ok(());
    }
    
    let matcher = BlocklistMatcher::new(config);
    let result = matcher.check_command(command);
    
    // Hard block - immediately reject
    if result.blocked {
        return Err(BlockedError {
            reason: result.reason.unwrap_or_else(|| "Command blocked".to_string()),
            guidance: result.guidance,
            rule_id: result.matched_rule_id.unwrap_or_default(),
        });
    }
    
    // Prompt case (not blocked, not allowed) - user decision required
    if !result.allowed {
        let pattern = result.matched_rule_id.clone().unwrap_or_default();
        
        // Check session allowances first
        if is_session_allowed(&pattern) {
            return Ok(());
        }
        
        // Pause for user decision
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Triple,
            tool_name: "Bash".to_string(),
            message: result.reason.unwrap_or_else(|| "Command requires approval".to_string()),
            details: Some(command.to_string()),
        });
        
        match response {
            PauseResponse::AllowOnce => Ok(()),
            PauseResponse::AllowSession => {
                allow_for_session(&pattern);
                Ok(())
            }
            PauseResponse::Denied | PauseResponse::Interrupted => {
                Err(BlockedError {
                    reason: "User denied access".to_string(),
                    guidance: result.guidance,
                    rule_id: pattern,
                })
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}

/// Check a file path against the blocklist.
/// Returns Ok(()) if the path is allowed, Err(BlockedError) if blocked.
/// Reloads config on every check to pick up changes without restart.
/// Used by Read, Write, and Edit tools to protect sensitive files.
pub fn check_file_path(file_path: &str) -> Result<(), BlockedError> {
    use crate::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};
    
    let project_root = get_project_root();
    let config = load_blocklist_config(project_root.as_deref());
    
    if config.rules.is_empty() {
        return Ok(());
    }
    
    let matcher = BlocklistMatcher::new(config);
    let result = matcher.check_command(file_path);
    
    // Hard block - immediately reject
    if result.blocked {
        return Err(BlockedError {
            reason: result.reason.unwrap_or_else(|| "File access blocked".to_string()),
            guidance: result.guidance,
            rule_id: result.matched_rule_id.unwrap_or_default(),
        });
    }
    
    // Prompt case (not blocked, not allowed) - user decision required
    if !result.allowed {
        let pattern = result.matched_rule_id.clone().unwrap_or_default();
        
        // Check session allowances first
        if is_session_allowed(&pattern) {
            return Ok(());
        }
        
        // Pause for user decision
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Triple,
            tool_name: "Read".to_string(),
            message: result.reason.unwrap_or_else(|| "Sensitive file access".to_string()),
            details: Some(file_path.to_string()),
        });
        
        match response {
            PauseResponse::AllowOnce => Ok(()),
            PauseResponse::AllowSession => {
                allow_for_session(&pattern);
                Ok(())
            }
            PauseResponse::Denied | PauseResponse::Interrupted => {
                Err(BlockedError {
                    reason: "User denied access".to_string(),
                    guidance: result.guidance,
                    rule_id: pattern,
                })
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}

/// Check a bash command and return the raw result.
/// This is used by NAPI bindings that need the full CheckResult.
pub fn check_command_raw(command: &str) -> CheckResult {
    // Read lock to access the matcher
    // If lock is poisoned, we recover by taking the poisoned guard
    let guard = match BLOCKLIST_MATCHER.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    
    if let Some(ref matcher) = *guard {
        matcher.check_command(command)
    } else {
        CheckResult::allowed()
    }
}

/// Reload the blocklist configuration.
/// This creates a new matcher with updated rules.
/// Note: Due to OnceLock semantics, this actually creates a new matcher
/// but the old one remains until process restart.
pub fn reload_blocklist(project_root: Option<&Path>) -> BlocklistMatcher {
    let config = load_blocklist_config(project_root);
    BlocklistMatcher::new(config)
}

// ============================================================================
// SESSION ALLOWANCES (BLOCK-005)
// ============================================================================

/// Add a pattern to session allowances.
/// Called when user selects "Allow Session" in the triple confirmation dialog.
pub fn allow_for_session(pattern: &str) {
    let mut guard = match SESSION_ALLOWANCES.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.insert(pattern.to_string());
}

/// Check if a pattern is allowed for the current session.
/// Returns true if the pattern was previously allowed via allow_for_session.
pub fn is_session_allowed(pattern: &str) -> bool {
    let guard = match SESSION_ALLOWANCES.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.contains(pattern)
}

/// Clear all session allowances.
/// Called on TUI restart to reset session memory.
pub fn clear_session_allowances() {
    let mut guard = match SESSION_ALLOWANCES.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clear();
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::blocklist::{BlocklistAction, BlocklistRule};
    use serial_test::serial;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(dir: &Path, rules: Vec<BlocklistRule>) {
        let fspec_dir = dir.join(".fspec");
        std::fs::create_dir_all(&fspec_dir).expect("failed to create .fspec directory");

        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules,
        };

        let path = fspec_dir.join("blocklist.json");
        let mut file = std::fs::File::create(&path).expect("failed to create blocklist.json");
        let json = serde_json::to_string_pretty(&config).expect("failed to serialize config");
        file.write_all(json.as_bytes())
            .expect("failed to write blocklist.json");
    }

    #[test]
    #[serial]
    fn test_load_empty_config_when_no_files() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        
        // Initialize blocklist with temp dir (no project blocklist.json)
        init_blocklist(Some(tmp.path()));
        
        // The matcher should match system config only (if any)
        // We test that no project config is loaded by checking that 
        // a command not in system config is not blocked
        let result = check_bash_command("this_command_does_not_exist_in_any_blocklist");
        assert!(result.is_ok(), "Random command should not be blocked when no project config exists");
        
        // Cleanup
        clear_session_allowances();
        init_blocklist(None);
    }

    #[test]
    #[serial]
    fn test_load_project_config() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "test-rule".to_string(),
            pattern: "^testcmd123".to_string(),
            action: BlocklistAction::Block,
            reason: "Test reason".to_string(),
            guidance: None,
        }]);
        
        // Initialize blocklist with temp dir containing our test config
        init_blocklist(Some(tmp.path()));
        
        // The project rule should be loaded and active
        let result = check_bash_command("testcmd123 arg1");
        assert!(result.is_err(), "Command matching project rule should be blocked");
        
        let blocked = result.unwrap_err();
        assert_eq!(blocked.rule_id, "test-rule");
        assert!(blocked.reason.contains("Test reason"));
        
        // Cleanup
        clear_session_allowances();
        init_blocklist(None);
    }

    #[test]
    fn test_blocked_error_display() {
        let error = BlockedError {
            reason: "git checkout is deprecated".to_string(),
            guidance: Some("Use git switch instead.".to_string()),
            rule_id: "git-checkout-block".to_string(),
        };
        
        assert_eq!(
            error.to_string(),
            "Blocked: git checkout is deprecated Use git switch instead."
        );
    }

    #[test]
    fn test_blocked_error_display_without_guidance() {
        let error = BlockedError {
            reason: "Dangerous command".to_string(),
            guidance: None,
            rule_id: "dangerous".to_string(),
        };
        
        assert_eq!(error.to_string(), "Blocked: Dangerous command");
    }

    // ========================================
    // SESSION ALLOWANCES TESTS (BLOCK-005)
    // Feature: spec/features/sensitive-path-prompts.feature
    // ========================================

    /// Scenario: Session allowances cleared on TUI restart
    #[test]
    #[serial]
    fn test_session_allowances_add_and_check() {
        // Start with clean state for test isolation
        clear_session_allowances();
        
        // @step Given a blocklist rule prompts for "npm install" commands
        let pattern = "^npm\\s+install";
        
        // @step When the AI runs "npm install" and user allows for session
        allow_for_session(pattern);
        
        // @step Then the AI can run "npm install lodash" without prompting
        assert!(is_session_allowed(pattern));
        
        // @step When the user exits and restarts the TUI
        clear_session_allowances();
        
        // @step And the AI runs "npm install axios"
        // (pattern check after restart)
        
        // @step Then the user should be prompted again
        assert!(!is_session_allowed(pattern));
    }

    /// Scenario: Prompt for SSH config access - user allows for session
    #[test]
    #[serial]
    fn test_session_allowances_pattern_matching() {
        // Start with clean state for test isolation
        clear_session_allowances();
        
        // @step Given a blocklist rule exists prompting for "~/.ssh" access
        let pattern = "~/.ssh";
        
        // @step And the user selects "Allow Session"
        allow_for_session(pattern);
        
        // @step When the AI tries to read "~/.ssh/known_hosts" later in the same session
        // @step Then the file should be read without prompting (same pattern)
        assert!(is_session_allowed(pattern));
        
        // Different pattern should NOT be allowed
        assert!(!is_session_allowed("~/.aws"));
        
        // Cleanup
        clear_session_allowances();
    }

    /// Scenario: Multiple patterns can be allowed in session
    #[test]
    #[serial]
    fn test_session_allowances_multiple_patterns() {
        // Start with clean state for test isolation
        clear_session_allowances();
        
        // Allow multiple patterns
        allow_for_session("~/.ssh");
        allow_for_session(".env");
        allow_for_session("~/.fspec");
        
        // All should be allowed
        assert!(is_session_allowed("~/.ssh"));
        assert!(is_session_allowed(".env"));
        assert!(is_session_allowed("~/.fspec"));
        
        // Cleanup
        clear_session_allowances();
    }

    /// Scenario: Session allowances are isolated per test (verify cleanup)
    #[test]
    #[serial]
    fn test_session_allowances_isolation() {
        // Start with clean state
        clear_session_allowances();
        
        // Nothing should be allowed
        assert!(!is_session_allowed("any-pattern"));
        
        // Add and verify
        allow_for_session("test-pattern");
        assert!(is_session_allowed("test-pattern"));
        
        // Cleanup
        clear_session_allowances();
    }

    // ========================================
    // BLOCK-007: PROMPT ACTION INTEGRATION TESTS
    // Feature: spec/features/integrate-blocklist-prompt-action-with-tool-pause-system.feature
    // ========================================

    /// Scenario: Session allowances bypass prompting
    #[test]
    #[serial]
    fn test_prompt_rule_bypassed_when_session_allowed() {
        use crate::tool_pause::{set_pause_handler, PauseHandler, PauseResponse};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // Start with clean state
        clear_session_allowances();
        set_pause_handler(None);
        
        // Track if pause handler was called
        let pause_called = Arc::new(AtomicBool::new(false));
        let pause_called_clone = pause_called.clone();
        
        // Set up a pause handler that tracks calls
        let handler: PauseHandler = Arc::new(move |_| {
            pause_called_clone.store(true, Ordering::SeqCst);
            PauseResponse::AllowOnce
        });
        set_pause_handler(Some(handler));
        
        // @step Given a blocklist rule with action "prompt" exists for ".env" files
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "env-prompt".to_string(),
            pattern: r"\.env".to_string(),
            action: BlocklistAction::Prompt,
            reason: "Environment files may contain secrets".to_string(),
            guidance: None,
        }]);
        init_blocklist(Some(tmp.path()));
        
        // @step And the user previously allowed ".env" pattern for the session
        allow_for_session("env-prompt");
        
        // @step When the AI attempts to read "~/.env"
        let result = check_file_path("~/.env");
        
        // Clean up before assertions
        set_pause_handler(None);
        clear_session_allowances();
        init_blocklist(None);
        
        // @step Then the file should be read without prompting
        assert!(result.is_ok(), "File should be allowed when session allowance exists");
        assert!(!pause_called.load(Ordering::SeqCst), "Pause handler should NOT be called when session allowance exists");
    }

    /// Scenario: User allows sensitive file access once
    #[test]
    #[serial]
    fn test_prompt_rule_allow_once_response() {
        use crate::tool_pause::{set_pause_handler, PauseHandler, PauseResponse, PauseKind};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // Start with clean state
        clear_session_allowances();
        set_pause_handler(None);
        
        // Track pause handler calls and verify Triple kind
        let pause_called = Arc::new(AtomicBool::new(false));
        let pause_called_clone = pause_called.clone();
        let was_triple = Arc::new(AtomicBool::new(false));
        let was_triple_clone = was_triple.clone();
        
        // @step When the user presses Enter to select "Allow Once"
        let handler: PauseHandler = Arc::new(move |req| {
            pause_called_clone.store(true, Ordering::SeqCst);
            if req.kind == PauseKind::Triple {
                was_triple_clone.store(true, Ordering::SeqCst);
            }
            PauseResponse::AllowOnce
        });
        set_pause_handler(Some(handler));
        
        // @step Given a blocklist rule with action "prompt" exists for ".env" files
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "env-prompt".to_string(),
            pattern: r"\.env".to_string(),
            action: BlocklistAction::Prompt,
            reason: "Sensitive file access".to_string(),
            guidance: None,
        }]);
        init_blocklist(Some(tmp.path()));
        
        // @step And the AI session is active
        // (session is implicitly active when we can call check_file_path)
        
        // @step When the AI attempts to read "~/.env"
        let result = check_file_path("~/.env");
        
        // Clean up
        set_pause_handler(None);
        init_blocklist(None);
        
        // @step Then the TUI should show an inline triple pause with message "Sensitive file access (.env)"
        assert!(pause_called.load(Ordering::SeqCst), "Pause handler should be called for prompt rule");
        
        // @step And the pause should display three options: Allow Once, Allow Session, Deny
        assert!(was_triple.load(Ordering::SeqCst), "Pause should be PauseKind::Triple");
        
        // @step Then the file should be read successfully
        assert!(result.is_ok(), "File should be allowed when user selects AllowOnce");
        
        // @step When the AI attempts to read "~/.env" again
        // @step Then the TUI should prompt again
        // (AllowOnce should NOT add to session allowances, so next access prompts again)
        assert!(!is_session_allowed("env-prompt"), "AllowOnce should NOT add to session allowances");
        
        // Cleanup
        clear_session_allowances();
    }

    /// Scenario: User allows sensitive file access for session
    #[test]
    #[serial]
    fn test_prompt_rule_allow_session_response() {
        use crate::tool_pause::{set_pause_handler, PauseHandler, PauseResponse};
        use std::sync::Arc;
        
        // Start with clean state  
        clear_session_allowances();
        set_pause_handler(None);
        
        // @step When the user navigates right and selects "Allow Session"
        let handler: PauseHandler = Arc::new(|_| PauseResponse::AllowSession);
        set_pause_handler(Some(handler));
        
        // @step Given a blocklist rule with action "prompt" exists for "~/.ssh" access
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "ssh-prompt".to_string(),
            pattern: r"\.ssh".to_string(),
            action: BlocklistAction::Prompt,
            reason: "SSH directory access".to_string(),
            guidance: None,
        }]);
        init_blocklist(Some(tmp.path()));
        
        // @step And the AI session is active
        // (session is implicitly active when we can call check_file_path)
        
        // @step When the AI attempts to read "~/.ssh/config"
        let result = check_file_path("~/.ssh/config");
        
        // Clean up handler but NOT session allowances yet
        set_pause_handler(None);
        init_blocklist(None);
        
        // @step Then the TUI should show an inline triple pause
        // (tested by pause handler being called - verified in allow_once test)
        
        // @step Then the file should be read successfully
        assert!(result.is_ok(), "File should be allowed when user selects AllowSession");
        
        // @step When the AI attempts to read "~/.ssh/known_hosts" later
        // @step Then the file should be read without prompting
        assert!(is_session_allowed("ssh-prompt"), "AllowSession should add pattern to session allowances");
        
        // Cleanup
        clear_session_allowances();
    }

    /// Scenario: User denies sensitive file access
    #[test]
    #[serial]
    fn test_prompt_rule_deny_response() {
        use crate::tool_pause::{set_pause_handler, PauseHandler, PauseResponse};
        use std::sync::Arc;
        
        // Start with clean state
        clear_session_allowances();
        set_pause_handler(None);
        
        // @step When the user navigates to "Deny" and presses Enter
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
        set_pause_handler(Some(handler));
        
        // @step Given a blocklist rule with action "prompt" exists for ".env" files
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "env-prompt".to_string(),
            pattern: r"\.env".to_string(),
            action: BlocklistAction::Prompt,
            reason: "Environment file".to_string(),
            guidance: None,
        }]);
        init_blocklist(Some(tmp.path()));
        
        // @step And the AI session is active
        // (session is implicitly active when we can call check_file_path)
        
        // @step When the AI attempts to read "~/.env"
        let result = check_file_path("~/.env");
        
        // Clean up
        set_pause_handler(None);
        clear_session_allowances();
        init_blocklist(None);
        
        // @step Then the TUI should show an inline triple pause
        // (tested by pause handler being called - verified in allow_once test)
        
        // @step Then the read should be blocked
        assert!(result.is_err(), "File should be blocked when user selects Deny");
        
        // @step And the AI should receive an error message "User denied access to sensitive file"
        let err = result.unwrap_err();
        assert!(err.reason.contains("denied") || err.reason.contains("Denied"), 
                "Error should indicate user denied access, got: {}", err.reason);
    }

    /// Scenario: Session allowances cleared on TUI restart
    #[test]
    #[serial]
    fn test_session_allowances_cleared_on_restart() {
        use crate::tool_pause::{set_pause_handler, PauseHandler, PauseResponse};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        // Start with clean state
        clear_session_allowances();
        set_pause_handler(None);
        
        // Track how many times pause handler is called
        let pause_count = Arc::new(AtomicUsize::new(0));
        let pause_count_clone = pause_count.clone();
        
        let handler: PauseHandler = Arc::new(move |_| {
            pause_count_clone.fetch_add(1, Ordering::SeqCst);
            PauseResponse::AllowSession
        });
        set_pause_handler(Some(handler));
        
        // @step Given a blocklist rule with action "prompt" exists for ".env" files
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        create_test_config(tmp.path(), vec![BlocklistRule {
            id: "env-prompt".to_string(),
            pattern: r"\.env".to_string(),
            action: BlocklistAction::Prompt,
            reason: "Environment file".to_string(),
            guidance: None,
        }]);
        init_blocklist(Some(tmp.path()));
        
        // @step And the user allowed ".env" pattern for the session
        allow_for_session("env-prompt");
        
        // Verify initial state - should be allowed
        assert!(is_session_allowed("env-prompt"), "Pattern should be allowed initially");
        
        // @step When the user restarts the TUI
        clear_session_allowances();
        
        // Verify allowance was cleared
        assert!(!is_session_allowed("env-prompt"), "Pattern should NOT be allowed after restart");
        
        // @step And the AI attempts to read "~/.env"
        let _result = check_file_path("~/.env");
        
        // @step Then the TUI should prompt again
        assert!(pause_count.load(Ordering::SeqCst) > 0, "Pause handler should be called after restart");
        
        // Cleanup
        set_pause_handler(None);
        clear_session_allowances();
        init_blocklist(None);
    }
}
